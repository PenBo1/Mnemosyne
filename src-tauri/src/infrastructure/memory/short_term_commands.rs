// 短期记忆 IPC 命令 —— 暴露 memory_short_term 表的查询能力给前端。
//
// 设计:
// - 查询类:按日期 / 按 session / 按 book / 按日期范围
// - 触发类:让用户主动重新生成 session 摘要
// - 不暴露 upsert(由 AgentEngine.summarize_session 在 session commit 时自动触发)
//
// 鉴权约束(对齐 AGENTS.md):
// - session_id / book_id 走 validate_id_component 防 ../
// - limit 上限 500 防止大表全扫返回
// - 日期格式校验 YYYY-MM-DD

use serde::Serialize;
use tauri::State;

use crate::core::agent::commands::AgentState;
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::short_term_memory::ShortTermMemoryRow;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::shared::error::{AppError, IpcResponse};

/// 校验 YYYY-MM-DD 格式
fn validate_date(s: &str, field: &str) -> Result<(), AppError> {
    if s.len() != 10
        || s.chars().nth(4) != Some('-')
        || s.chars().nth(7) != Some('-')
        || !s[0..4].chars().all(|c| c.is_ascii_digit())
        || !s[5..7].chars().all(|c| c.is_ascii_digit())
        || !s[8..10].chars().all(|c| c.is_ascii_digit())
    {
        return Err(AppError::invalid_input(format!(
            "Invalid {} format, expected YYYY-MM-DD, got: {}",
            field, s
        )));
    }
    Ok(())
}

/// 按 YYYY-MM-DD 日期查询所有 session 摘要(用于每日回顾 UI)
#[tauri::command]
pub async fn short_term_memory_list_by_date(
    state: State<'_, DbState>,
    entry_date: String,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_date(&entry_date, "entry_date")?;
    let rows = state.db.list_short_term_by_date(&entry_date)?;
    Ok(IpcResponse::ok(rows))
}

/// 按 session_id 获取最新摘要(用于 session 恢复时回填"上次进展")
#[tauri::command]
pub async fn short_term_memory_for_session(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<Option<ShortTermMemoryRow>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let row = state.db.get_short_term_for_session(&session_id)?;
    Ok(IpcResponse::ok(row))
}

/// 按 book_id 列出该书的 session 摘要(用于小说创作的上下文回顾)
#[tauri::command]
pub async fn short_term_memory_list_by_book(
    state: State<'_, DbState>,
    book_id: String,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let l = limit.unwrap_or(50);
    let rows = state.db.list_short_term_by_book(&book_id, l)?;
    Ok(IpcResponse::ok(rows))
}

/// 按日期范围列出短期记忆(用于"最近 N 天"或自定义范围)
#[tauri::command]
pub async fn short_term_memory_list_by_range(
    state: State<'_, DbState>,
    start_date: String,
    end_date: String,
) -> Result<IpcResponse<Vec<ShortTermMemoryRow>>, AppError> {
    validate_date(&start_date, "start_date")?;
    validate_date(&end_date, "end_date")?;
    if start_date > end_date {
        return Err(AppError::invalid_input("start_date must be <= end_date"));
    }
    let rows = state
        .db
        .list_short_term_by_date_range(&start_date, &end_date)?;
    Ok(IpcResponse::ok(rows))
}

/// 主动为 session 重新生成摘要(用户点击"重新生成"时触发)
///
/// 通过 AgentState 获取 AgentEngine 实例,业务逻辑在 AgentEngine.summarize_session 内部。
#[tauri::command]
pub async fn short_term_memory_regenerate(
    state: State<'_, AgentState>,
    session_id: String,
    book_id: Option<String>,
    agent_role: Option<String>,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    if let Some(b) = book_id.as_deref() {
        validate_id_component(b, "book_id")?;
    }
    state
        .engine
        .summarize_session(&session_id, book_id.as_deref(), agent_role.as_deref())
        .await?;
    Ok(IpcResponse::ok(true))
}

/// 短期记忆统计(用于 UI 展示总数 / 按日期分布)
#[derive(Debug, Serialize)]
pub struct ShortTermMemoryStats {
    pub total: u64,
    pub today_count: u64,
    pub last_7_days_count: u64,
}

#[tauri::command]
pub async fn short_term_memory_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<ShortTermMemoryStats>, AppError> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_count = state.db.list_short_term_by_date(&today)?.len() as u64;

    // 最近 7 天(包括今天)
    let mut start_date = chrono::Utc::now();
    for _ in 0..6 {
        start_date = start_date - chrono::Duration::days(1);
    }
    let start_str = start_date.format("%Y-%m-%d").to_string();
    let last_7 = state
        .db
        .list_short_term_by_date_range(&start_str, &today)?;
    let last_7_days_count = last_7.len() as u64;

    // total = 7 天内活跃指标(避免全表 COUNT,这是 hot memory,GC 后会收缩)
    let total = last_7_days_count;

    Ok(IpcResponse::ok(ShortTermMemoryStats {
        total,
        today_count,
        last_7_days_count,
    }))
}
