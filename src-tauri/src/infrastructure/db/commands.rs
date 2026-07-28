//! ═══════════════════════════════════════════════════════════════════════════
//! 数据库命令 - Tauri 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
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