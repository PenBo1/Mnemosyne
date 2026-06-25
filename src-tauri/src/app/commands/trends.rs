use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infra::fs_utils::validate_id_component;

#[tauri::command]
pub async fn create_trend(
    state: State<'_, AppState>,
    keyword: String,
    platform: String,
    score: f64,
    metadata: serde_json::Value,
) -> Result<IpcResponse<crate::infra::db::models::Trend>, AppError> {
    if keyword.trim().is_empty() {
        return Err(AppError::invalid_input("Trend keyword cannot be empty"));
    }
    if keyword.len() > 255 {
        return Err(AppError::invalid_input("Trend keyword too long (max 255 chars)"));
    }
    if platform.len() > 100 {
        return Err(AppError::invalid_input("Platform name too long (max 100 chars)"));
    }
    if !score.is_finite() || score < 0.0 || score > 1_000_000.0 {
        return Err(AppError::invalid_input("Score must be a finite number between 0 and 1000000"));
    }
    tracing::info!(keyword = %keyword, platform = %platform, score = score, "create_trend");
    let trend = state.db.create_trend(&keyword, &platform, score, metadata).await?;
    tracing::info!(trend_id = %trend.id, "Trend created");
    Ok(IpcResponse::created(trend))
}

#[tauri::command]
pub async fn list_trends(
    state: State<'_, AppState>,
    platform: Option<String>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::Trend>>, AppError> {
    tracing::debug!(platform = ?platform, limit = ?limit, "list_trends");
    let trends = state.db.list_trends(platform.as_deref(), limit).await?;
    tracing::debug!(count = trends.len(), "Trends listed");
    Ok(IpcResponse::ok(trends))
}

#[tauri::command]
pub async fn delete_trend(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "trend_id")?;
    tracing::info!(trend_id = %id, "delete_trend");
    let deleted = state.db.delete_trend(&id).await?;
    tracing::info!(trend_id = %id, deleted, "Trend deleted");
    Ok(IpcResponse::ok(deleted))
}
