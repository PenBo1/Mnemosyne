//! ═══════════════════════════════════════════════════════════════════════════
//! 通知模块 - 本地 OS 通知
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use serde::Deserialize;
use tauri_plugin_notification::NotificationExt;
use std::time::Instant;

/// 通知载荷
#[derive(Debug, Deserialize)]
pub struct NotificationPayload {
    /// 标题
    pub title: String,
    /// 内容
    pub body: String,
}

/// 发送通知
#[tauri::command]
pub async fn send_notification(
    app: tauri::AppHandle,
    payload: NotificationPayload,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(
        title = %payload.title,
        body_len = payload.body.len(),
        "send_notification: enter"
    );
    
    if payload.title.trim().is_empty() {
        tracing::error!("send_notification: Notification title cannot be empty");
        return Err(AppError::invalid_input("Notification title cannot be empty"));
    }
    if payload.title.len() > 255 {
        tracing::error!(len = payload.title.len(), "send_notification: Notification title too long");
        return Err(AppError::invalid_input("Notification title too long (max 255 chars)"));
    }
    if payload.body.len() > 10_000 {
        tracing::error!(len = payload.body.len(), "send_notification: Notification body too long");
        return Err(AppError::invalid_input("Notification body too long (max 10000 chars)"));
    }
    
    app.notification()
        .builder()
        .title(&payload.title)
        .body(&payload.body)
        .show()
        .map_err(|e| {
            tracing::error!(error = %e, "send_notification: Failed to send notification");
            AppError::internal(e.to_string())
        })?;
    
    tracing::info!(
        title = %payload.title,
        duration_ms = start.elapsed().as_millis(),
        "send_notification: exit"
    );
    Ok(IpcResponse::ok(()))
}