// SecurityKernel IPC 命令:审计事件查询与统计。
//
// 供仪表盘 Violations 卡片与审计日志页面调用。

use tauri::State;

use crate::infrastructure::db::state::DbState;
use crate::shared::error::{AppError, IpcResponse};

/// 查询最近的审计事件(按 recorded_at 倒序)。limit 默认 50,上限 1000。
#[tauri::command]
pub async fn audit_events_query(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let limit = limit.unwrap_or(50);
    let rows = state.db.query_audit_events(limit)?;
    Ok(IpcResponse::ok(serde_json::to_value(rows)?))
}

/// 审计事件聚合统计:总数 / 拒绝数 / 安全相关数 / 按类型分组。
#[tauri::command]
pub async fn audit_event_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let stats = state.db.audit_event_stats()?;
    Ok(IpcResponse::ok(serde_json::to_value(stats)?))
}
