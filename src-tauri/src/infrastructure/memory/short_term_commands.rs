//! ═══════════════════════════════════════════════════════════════════════════
//! 短期记忆命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 仅包含参数验证和委托，业务逻辑在 service 和 validation 模块中。

use tauri::State;
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow;
use crate::shared::error::{AppError, IpcResponse};
use super::validation::{validate_date, validate_date_range, validate_session_id, validate_book_id};
use super::service::ShortTermMemoryService;
use super::dto::ShortTermMemoryStats;

// ── 查询命令 ────────────────────────────────────────────────────────────────

/// 按 YYYY-MM-DD 日期查询所有 session 摘要
#[tauri::command]
pub async fn short_term_memory_list_by_date(
    state: State<'_, DbState>,
    entry_date: String,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_date(&entry_date, "entry_date")?;
    let rows = ShortTermMemoryService::list_by_date(&state.db, &entry_date)?;
    Ok(IpcResponse::ok(rows))
}

/// 按 session_id 获取最新摘要
#[tauri::command]
pub async fn short_term_memory_for_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<Option<ShortTermMemoryRow>>, AppError> {
    validate_session_id(&session_id)?;
    let row = ShortTermMemoryService::get_for_session(&state.db, &session_id)?;
    Ok(IpcResponse::ok(row))
}

/// 按 book_id 列出该书的 session 摘要
#[tauri::command]
pub async fn short_term_memory_list_by_book(
    state: State<'_, DbState>,
    book_id: String,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_book_id(&book_id)?;
    let l = limit.unwrap_or(50);
    let rows = ShortTermMemoryService::list_by_book(&state.db, &book_id, l)?;
    Ok(IpcResponse::ok(rows))
}

/// 按日期范围列出短期记忆
#[tauri::command]
pub async fn short_term_memory_list_by_range(
    state: State<'_, DbState>,
    start_date: String,
    end_date: String,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_date(&start_date, "start_date")?;
    validate_date(&end_date, "end_date")?;
    validate_date_range(&start_date, &end_date)?;
    let rows = ShortTermMemoryService::list_by_range(&state.db, &start_date, &end_date)?;
    Ok(IpcResponse::ok(rows))
}

// ── 统计命令 ────────────────────────────────────────────────────────────────

/// 短期记忆统计
#[tauri::command]
pub async fn short_term_memory_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<ShortTermMemoryStats>, AppError> {
    let stats = ShortTermMemoryService::get_stats(&state.db)?;
    Ok(IpcResponse::ok(stats))
}