//! ═══════════════════════════════════════════════════════════════════════════
//! 数据库命令 - Tauri 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::time::Instant;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::audit::{
    AuditEventFilter, AuditHistogramBucket, AuditEventRow,
};
use crate::infrastructure::fs::fs_utils::validate_id_component;

// ── 趋势命令 ────────────────────────────────────────────────────────────────

/// 创建趋势记录
#[tauri::command]
pub async fn create_trend(
    state: State<'_, DbState>,
    keyword: String,
    platform: String,
    score: f64,
    metadata: serde_json::Value,
) -> Result<IpcResponse<crate::infrastructure::db::types::Trend>, AppError> {
    if keyword.trim().is_empty() {
        return Err(AppError::invalid_input("Trend keyword cannot be empty"));
    }
    if keyword.len() > 255 {
        return Err(AppError::invalid_input("Trend keyword too long (max 255 chars)"));
    }
    if platform.len() > 100 {
        return Err(AppError::invalid_input("Platform name too long (max 100 chars)"));
    }
    if !score.is_finite() || !(0.0..=1_000_000.0).contains(&score) {
        return Err(AppError::invalid_input("Score must be a finite number between 0 and 1000000"));
    }
    tracing::info!(keyword = %keyword, platform = %platform, score = score, "create_trend");
    let db = state.db.clone();
    let trend = tokio::task::spawn_blocking(move || {
        db.create_trend(&keyword, &platform, score, metadata)
    }).await
        .map_err(|e| AppError::internal(format!("Database task join failed: {}", e)))??;
    tracing::info!(trend_id = %trend.id, "Trend created");
    Ok(IpcResponse::created(trend))
}

/// 获取趋势列表
#[tauri::command]
pub async fn list_trends(
    state: State<'_, DbState>,
    platform: Option<String>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Trend>>, AppError> {
    tracing::debug!(platform = ?platform, limit = ?limit, "list_trends");
    let db = state.db.clone();
    let trends = tokio::task::spawn_blocking(move || {
        db.list_trends(platform.as_deref(), limit)
    }).await
        .map_err(|e| AppError::internal(format!("Database task join failed: {}", e)))??;
    tracing::debug!(count = trends.len(), "Trends listed");
    Ok(IpcResponse::ok(trends))
}

/// 删除趋势记录
#[tauri::command]
pub async fn delete_trend(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "trend_id")?;
    tracing::info!(trend_id = %id, "delete_trend");
    let db = state.db.clone();
    let id_for_log = id.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        db.delete_trend(&id)
    }).await
        .map_err(|e| AppError::internal(format!("Database task join failed: {}", e)))??;
    tracing::info!(trend_id = %id_for_log, deleted, "Trend deleted");
    Ok(IpcResponse::ok(deleted))
}

// ── 审计事件查询命令（Security Kernel 读模型）──────────────────────────────

/// 查询最近的审计事件(按 recorded_at 倒序)。limit 默认 50,上限 1000。
#[tauri::command]
pub async fn audit_events_query(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<AuditEventRow>>, AppError> {
    let limit = limit.unwrap_or(50);
    let rows = state.db.query_audit_events(limit)?;
    Ok(IpcResponse::ok(rows))
}

/// 审计事件聚合统计:总数 / 拒绝数 / 安全相关数 / 按类型分组。
#[tauri::command]
pub async fn audit_event_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let start = Instant::now();
    tracing::info!("audit_event_stats: enter");
    let stats = state.db.audit_event_stats()?;
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "audit_event_stats: exit"
    );
    Ok(IpcResponse::ok(serde_json::to_value(stats)?))
}

/// 按过滤条件查询审计事件。
///
/// 支持 workspace_id / operation(LIKE) / event_type / since / until / only_denied / only_security / offset / limit。
#[tauri::command]
pub async fn audit_events_query_filtered(
    filter: AuditEventFilter,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<AuditEventRow>>, AppError> {
    let rows = state.db.query_audit_events_filtered(&filter)?;
    Ok(IpcResponse::ok(rows))
}

/// 审计事件直方图(按时间桶聚合)。
///
/// granularity ∈ {"hour","day","month"}。since/until 为可选 RFC3339 字符串。
#[tauri::command]
pub async fn audit_event_histogram(
    granularity: String,
    since: Option<String>,
    until: Option<String>,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<AuditHistogramBucket>>, AppError> {
    let buckets = state.db.audit_event_histogram(&granularity, since.as_deref(), until.as_deref())?;
    Ok(IpcResponse::ok(buckets))
}