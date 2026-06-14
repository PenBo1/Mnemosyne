use crate::errors::{AppError, IpcResponse};
use crate::infra::db::CreateSessionRequest;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn session_create(
    state: State<'_, AppState>,
    novel_id: Option<String>,
    title: Option<String>,
) -> Result<IpcResponse<crate::infra::db::Session>, AppError> {
    tracing::info!(novel_id = ?novel_id, title = ?title, "session_create");
    let db = state.db.lock().await;
    let session = db.create_session(CreateSessionRequest { novel_id, title })?;
    tracing::info!(session_id = %session.id, "Session created");
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_list(
    state: State<'_, AppState>,
    novel_id: Option<String>,
) -> Result<IpcResponse<Vec<crate::infra::db::Session>>, AppError> {
    tracing::debug!(novel_id = ?novel_id, "session_list");
    let db = state.db.lock().await;
    let sessions = db.list_sessions(novel_id.as_deref())?;
    tracing::debug!(count = sessions.len(), "Sessions listed");
    Ok(IpcResponse::ok(sessions))
}

#[tauri::command]
pub async fn session_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infra::db::Session>, AppError> {
    tracing::debug!(session_id = %id, "session_get");
    let db = state.db.lock().await;
    let session = db.get_session(&id)?
        .ok_or_else(|| {
            tracing::warn!(session_id = %id, "Session not found");
            AppError::session_not_found()
        })?;
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    tracing::info!(session_id = %id, "session_delete");
    let db = state.db.lock().await;
    let deleted = db.delete_session(&id)?;
    tracing::info!(session_id = %id, deleted, "Session deleted");
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<Vec<crate::infra::db::Message>>, AppError> {
    tracing::debug!(session_id = %session_id, "session_messages");
    let db = state.db.lock().await;
    let messages = db.list_messages(&session_id)?;
    tracing::debug!(session_id = %session_id, count = messages.len(), "Messages listed");
    Ok(IpcResponse::ok(messages))
}
