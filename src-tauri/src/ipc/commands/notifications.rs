use crate::shared::errors::{AppError, IpcResponse};
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
    if payload.title.trim().is_empty() {
        return Err(AppError::invalid_input("Notification title cannot be empty"));
    }
    if payload.title.len() > 255 {
        return Err(AppError::invalid_input("Notification title too long (max 255 chars)"));
    }
    if payload.body.len() > 10_000 {
        return Err(AppError::invalid_input("Notification body too long (max 10000 chars)"));
    }
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
