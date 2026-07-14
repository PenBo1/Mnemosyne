// 交互运行时 IPC 命令。
//
// 命令列表（前端使用 camelCase 调用）：
// - interaction_run_request: 运行交互请求（核心入口，分发 intent 到 pipeline/edit/chat）
// - interaction_list_sessions: 列出交互会话（可选按 book_id 过滤）
// - interaction_get_session: 获取单个交互会话（含完整 payload）
// - interaction_delete_session: 删除交互会话
// - interaction_update_automation_mode: 更新自动化模式
// - interaction_edit_chapter: 执行编辑事务（plan + execute，直接调 edit_controller）
//
// 约定：
// - 命令仅做参数提取 + 校验 + 委派，不含业务逻辑
// - 路径操作通过 DataDir getters
// - book_id 走 validate_id（与 pipeline/commands.rs 一致）
// - session_id 走 validate_id_component（与 session/commands.rs 一致）

use crate::core::agent::commands::AgentState;
use crate::domain::interaction::edit_controller::{self, EditRequest, ExecutedEditTransaction};
use crate::domain::interaction::runtime;
use crate::domain::interaction::session_store::create_new_interaction_session;
use crate::domain::interaction::types::{
    AutomationMode, InteractionRequest, InteractionRuntimeResult, InteractionSession, SessionKind,
};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;

// ── 校验工具 ─────────────────────────────────────────────────

fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id(book_id, "book_id").map_err(AppError::invalid_input)
}

/// 解析 automation_mode 字符串（前端传 lowercase: auto/semi/manual）
fn parse_automation_mode(s: &str) -> Result<AutomationMode, AppError> {
    match s.to_lowercase().as_str() {
        "auto" => Ok(AutomationMode::Auto),
        "semi" => Ok(AutomationMode::Semi),
        "manual" => Ok(AutomationMode::Manual),
        other => Err(AppError::invalid_input(format!(
            "未知的 automation_mode: {}（支持: auto/semi/manual）",
            other
        ))),
    }
}

// ── 运行交互请求 ─────────────────────────────────────────────

/// 运行交互请求（核心入口）。
///
/// 流程：
/// 1. 根据 request.session_id 加载已有会话，或创建新会话（SessionKind::Book）
/// 2. 调用 runtime::run_interaction_request 执行 intent 分发
/// 3. 持久化会话（无论成功/失败，都记录事件流）
/// 4. 返回 InteractionRuntimeResult
#[tauri::command]
pub async fn interaction_run_request(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    db_state: State<'_, DbState>,
    request: InteractionRequest,
) -> Result<IpcResponse<InteractionRuntimeResult>, AppError> {
    // 1. 加载或创建会话
    let mut session: InteractionSession = match request.session_id.as_deref() {
        Some(sid) if !sid.trim().is_empty() => {
            validate_id_component(sid, "session_id")?;
            db_state.db.load_interaction_session(sid)?
        }
        _ => create_new_interaction_session(SessionKind::Book),
    };
    let session_id = session.session_id.clone();

    // 2. 执行运行时分发
    let result =
        runtime::run_interaction_request(request, &mut session, &data_dir, &agent_state.engine)
            .await;

    // 3. 持久化会话（无论成功/失败，都记录事件流）
    if let Err(e) = db_state.db.persist_interaction_session(&session) {
        tracing::warn!(session_id = %session_id, error = %e, "Failed to persist interaction session");
    }

    // 4. 返回结果
    match result {
        Ok(r) => {
            tracing::info!(session_id = %session_id, "Interaction request completed");
            Ok(IpcResponse::ok(r))
        }
        Err(e) => {
            tracing::warn!(session_id = %session_id, error = %e, "Interaction request failed");
            Err(e)
        }
    }
}

// ── 列出交互会话 ─────────────────────────────────────────────

/// 列出交互会话（可选按 book_id 过滤，返回完整 payload）。
#[tauri::command]
pub async fn interaction_list_sessions(
    state: State<'_, DbState>,
    book_id: Option<String>,
) -> Result<IpcResponse<Vec<InteractionSession>>, AppError> {
    if let Some(ref bid) = book_id {
        validate_book_id(bid)?;
    }
    tracing::debug!(book_id = ?book_id, "interaction_list_sessions");
    let sessions = state
        .db
        .list_full_interaction_sessions(book_id.as_deref())?;
    tracing::debug!(count = sessions.len(), "Interaction sessions listed");
    Ok(IpcResponse::ok(sessions))
}

// ── 获取单个交互会话 ─────────────────────────────────────────

/// 获取单个交互会话（含完整 payload）。
#[tauri::command]
pub async fn interaction_get_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<InteractionSession>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::debug!(session_id = %session_id, "interaction_get_session");
    let session = state.db.load_interaction_session(&session_id)?;
    Ok(IpcResponse::ok(session))
}

// ── 删除交互会话 ─────────────────────────────────────────────

/// 删除交互会话。
#[tauri::command]
pub async fn interaction_delete_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::info!(session_id = %session_id, "interaction_delete_session");
    let deleted = state.db.delete_interaction_session(&session_id)?;
    tracing::info!(session_id = %session_id, deleted, "Interaction session deleted");
    Ok(IpcResponse::ok(deleted))
}

// ── 更新自动化模式 ───────────────────────────────────────────

/// 更新交互会话的自动化模式（轻量更新，不重写 payload）。
#[tauri::command]
pub async fn interaction_update_automation_mode(
    state: State<'_, DbState>,
    session_id: String,
    mode: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let mode_enum = parse_automation_mode(&mode)?;
    tracing::info!(
        session_id = %session_id,
        mode = %mode_enum.as_str(),
        "interaction_update_automation_mode"
    );
    let updated = state
        .db
        .update_interaction_session_automation_mode(&session_id, mode_enum)?;
    Ok(IpcResponse::ok(updated))
}

// ── 执行编辑事务 ─────────────────────────────────────────────

/// 执行编辑事务（plan + execute）。
///
/// 直接调用 edit_controller，不走 runtime 分发。
/// 适用于前端明确知道要执行哪种编辑操作的场景（如实体改名、章节替换、真相文件编辑）。
#[tauri::command]
pub async fn interaction_edit_chapter(
    data_dir: State<'_, DataDir>,
    request: EditRequest,
) -> Result<IpcResponse<ExecutedEditTransaction>, AppError> {
    validate_book_id(request.book_id())?;
    tracing::info!(book_id = %request.book_id(), "interaction_edit_chapter");
    let planned = edit_controller::plan_edit_transaction(request)?;
    let executed = edit_controller::execute_edit_transaction(planned, &data_dir)?;
    tracing::info!(
        book_id = %executed.book_id,
        transaction_type = %executed.transaction_type,
        touched = executed.touched_files.len(),
        "Edit transaction executed"
    );
    Ok(IpcResponse::ok(executed))
}
