//! ═══════════════════════════════════════════════════════════════════════════
//! 交互命令 - 交互运行时 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

use crate::core::agent::commands::AgentState;
use crate::domain::interaction::edit_controller::{self, EditRequest, ExecutedEditTransaction};
use crate::domain::interaction::pipeline_ops::InteractionPipelineOpsState;
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
use std::time::Instant;

// ── 辅助函数 ────────────────────────────────────────────────────────────────

fn validate_book_id(book_id: &str) -> Result<(), AppError> {
    validate_id(book_id, "book_id").map_err(AppError::invalid_input)
}

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

// ── IPC 命令 ────────────────────────────────────────────────────────────────

/// 运行交互请求
#[tauri::command]
pub async fn interaction_run_request(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    db_state: State<'_, DbState>,
    pipeline_ops_state: State<'_, InteractionPipelineOpsState>,
    request: InteractionRequest,
) -> Result<IpcResponse<InteractionRuntimeResult>, AppError> {
    let start = Instant::now();
    tracing::info!(
        session_id = ?request.session_id,
        intent = ?request.intent,
        "interaction_run_request: enter"
    );

    // 加载或创建会话
    let mut session: InteractionSession = match request.session_id.as_deref() {
        Some(sid) if !sid.trim().is_empty() => {
            validate_id_component(sid, "session_id")?;
            db_state.db.load_interaction_session(sid).map_err(|e| {
                tracing::error!(session_id = %sid, error = %e, "interaction_run_request: Failed to load session");
                e
            })?
        }
        _ => create_new_interaction_session(SessionKind::Book),
    };
    let session_id = session.session_id.clone();

    // 执行运行时分发
    let result =
        runtime::run_interaction_request(request, &mut session, &data_dir, &agent_state.engine, &*pipeline_ops_state.ops)
            .await;

    // 持久化会话
    if let Err(e) = db_state.db.persist_interaction_session(&session) {
        tracing::warn!(session_id = %session_id, error = %e, "Failed to persist interaction session");
    }

    // 返回结果
    match result {
        Ok(r) => {
            tracing::info!(
                session_id = %session_id,
                duration_ms = start.elapsed().as_millis(),
                "interaction_run_request: exit"
            );
            Ok(IpcResponse::ok(r))
        }
        Err(e) => {
            tracing::error!(session_id = %session_id, error = %e, "interaction_run_request: Failed to run interaction request");
            Err(e)
        }
    }
}

/// 列出交互会话
#[tauri::command]
pub async fn interaction_list_sessions(
    state: State<'_, DbState>,
    book_id: Option<String>,
) -> Result<IpcResponse<Vec<InteractionSession>>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = ?book_id, "interaction_list_sessions: enter");

    if let Some(ref bid) = book_id {
        validate_book_id(bid)?;
    }

    let sessions = state
        .db
        .list_full_interaction_sessions(book_id.as_deref()).map_err(|e| {
            tracing::error!(error = %e, "interaction_list_sessions: Failed to list sessions");
            e
        })?;

    tracing::info!(
        count = sessions.len(),
        duration_ms = start.elapsed().as_millis(),
        "interaction_list_sessions: exit"
    );
    Ok(IpcResponse::ok(sessions))
}

/// 获取单个交互会话
#[tauri::command]
pub async fn interaction_get_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<InteractionSession>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %session_id, "interaction_get_session: enter");

    validate_id_component(&session_id, "session_id")?;

    let session = state.db.load_interaction_session(&session_id).map_err(|e| {
        tracing::error!(session_id = %session_id, error = %e, "interaction_get_session: Failed to get session");
        e
    })?;

    tracing::info!(
        session_id = %session_id,
        duration_ms = start.elapsed().as_millis(),
        "interaction_get_session: exit"
    );
    Ok(IpcResponse::ok(session))
}

/// 删除交互会话
#[tauri::command]
pub async fn interaction_delete_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %session_id, "interaction_delete_session: enter");

    validate_id_component(&session_id, "session_id")?;

    let deleted = state.db.delete_interaction_session(&session_id).map_err(|e| {
        tracing::error!(session_id = %session_id, error = %e, "interaction_delete_session: Failed to delete session");
        e
    })?;

    tracing::info!(
        session_id = %session_id,
        deleted,
        duration_ms = start.elapsed().as_millis(),
        "interaction_delete_session: exit"
    );
    Ok(IpcResponse::ok(deleted))
}

/// 更新自动化模式
#[tauri::command]
pub async fn interaction_update_automation_mode(
    state: State<'_, DbState>,
    session_id: String,
    mode: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        session_id = %session_id,
        mode = %mode,
        "interaction_update_automation_mode: enter"
    );

    validate_id_component(&session_id, "session_id")?;
    let mode_enum = parse_automation_mode(&mode)?;

    let updated = state
        .db
        .update_interaction_session_automation_mode(&session_id, mode_enum).map_err(|e| {
            tracing::error!(session_id = %session_id, error = %e, "interaction_update_automation_mode: Failed to update automation mode");
            e
        })?;

    tracing::info!(
        session_id = %session_id,
        updated,
        duration_ms = start.elapsed().as_millis(),
        "interaction_update_automation_mode: exit"
    );
    Ok(IpcResponse::ok(updated))
}

/// 执行编辑事务
#[tauri::command]
pub async fn interaction_edit_chapter(
    data_dir: State<'_, DataDir>,
    request: EditRequest,
) -> Result<IpcResponse<ExecutedEditTransaction>, AppError> {
    let start = Instant::now();
    tracing::info!(book_id = %request.book_id(), "interaction_edit_chapter: enter");

    validate_book_id(request.book_id())?;

    let planned = edit_controller::plan_edit_transaction(request).map_err(|e| {
        tracing::error!(error = %e, "interaction_edit_chapter: Failed to plan edit transaction");
        e
    })?;
    let executed = edit_controller::execute_edit_transaction(planned, &data_dir).map_err(|e| {
        tracing::error!(error = %e, "interaction_edit_chapter: Failed to execute edit transaction");
        e
    })?;

    tracing::info!(
        book_id = %executed.book_id,
        transaction_type = %executed.transaction_type,
        touched = executed.touched_files.len(),
        duration_ms = start.elapsed().as_millis(),
        "interaction_edit_chapter: exit"
    );
    Ok(IpcResponse::ok(executed))
}