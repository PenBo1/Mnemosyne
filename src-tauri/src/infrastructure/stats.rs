//! ═══════════════════════════════════════════════════════════════════════════
//! 统计模块 - AI 指标聚合
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;

#[tauri::command]
pub async fn get_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_stats");
    let stats = state.db.get_stats()?;
    tracing::debug!("Stats retrieved");
    Ok(IpcResponse::ok(stats))
}

#[tauri::command]
pub async fn get_daily_activity(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_daily_activity");
    let activity = state.db.get_daily_activity()?;
    tracing::debug!("Daily activity retrieved");
    Ok(IpcResponse::ok(activity))
}

/// AI 指标聚合:token 总量 / LLM 调用数 / 工具调用数 / 模型用量分组。
/// 数据源为 messages 表中 assistant 消息(AgentEngine 流式调用后写入)。
#[tauri::command]
pub async fn get_ai_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_ai_stats");
    let stats = state.db.get_ai_stats()?;
    Ok(IpcResponse::ok(stats))
}

/// 使用统计聚合:活跃天数、连续活跃天数、热力图、Token 趋势、模型用量。
/// 参数 days 为统计天数范围 (7 或 30)。
#[tauri::command]
pub async fn get_usage_stats(
    state: State<'_, DbState>,
    days: i64,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_usage_stats days={}", days);
    let stats = state.db.get_usage_stats(days)?;
    Ok(IpcResponse::ok(stats))
}