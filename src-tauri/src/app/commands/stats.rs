use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;

#[tauri::command]
pub async fn get_stats(
    state: State<'_, AppState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_stats");
    let stats = state.db.get_stats().await?;
    tracing::debug!("Stats retrieved");
    Ok(IpcResponse::ok(stats))
}

#[tauri::command]
pub async fn get_daily_activity(
    state: State<'_, AppState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_daily_activity");
    let activity = state.db.get_daily_activity().await?;
    tracing::debug!("Daily activity retrieved");
    Ok(IpcResponse::ok(activity))
}
