use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;

#[tauri::command]
pub async fn create_trend(
    state: State<'_, AppState>,
    keyword: String,
    platform: String,
    score: f64,
    metadata: serde_json::Value,
) -> Result<IpcResponse<crate::infra::db::models::Trend>, AppError> {
    tracing::info!(keyword = %keyword, platform = %platform, score = score, "create_trend");
    let db = state.db.lock().await;
    let trend = db.create_trend(&keyword, &platform, score, metadata)?;
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
    let db = state.db.lock().await;
    let trends = db.list_trends(platform.as_deref(), limit)?;
    tracing::debug!(count = trends.len(), "Trends listed");
    Ok(IpcResponse::ok(trends))
}

#[tauri::command]
pub async fn delete_trend(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    tracing::info!(trend_id = %id, "delete_trend");
    let db = state.db.lock().await;
    let deleted = db.delete_trend(&id)?;
    tracing::info!(trend_id = %id, deleted, "Trend deleted");
    Ok(IpcResponse::ok(deleted))
}
