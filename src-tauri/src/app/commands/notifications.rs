use crate::errors::{AppError, IpcResponse};
use serde::Deserialize;
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub body: String,
}

#[tauri::command]
pub async fn send_notification(
    app: tauri::AppHandle,
    payload: NotificationPayload,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(title = %payload.title, body = %payload.body, "send_notification");
    app.notification()
        .builder()
        .title(&payload.title)
        .body(&payload.body)
        .show()
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to send notification");
            AppError::internal(e.to_string())
        })?;
    tracing::info!("Notification sent");
    Ok(IpcResponse::ok(()))
}
