use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;

#[tauri::command]
pub async fn get_stats(
    state: State<'_, AppState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_stats");
    let db = state.db.lock().await;
    let stats = db.get_stats()?;
    tracing::debug!("Stats retrieved");
    Ok(IpcResponse::ok(stats))
}

#[tauri::command]
pub async fn get_daily_activity(
    state: State<'_, AppState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    tracing::debug!("get_daily_activity");
    let db = state.db.lock().await;
    let activity = db.get_daily_activity()?;
    tracing::debug!("Daily activity retrieved");
    Ok(IpcResponse::ok(activity))
}
