use crate::shared::errors::{AppError, IpcResponse};
use crate::infrastructure::db::CreateSessionRequest;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn session_create(
    state: State<'_, AppState>,
    novel_id: Option<String>,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<IpcResponse<crate::infrastructure::db::Session>, AppError> {
    if let Some(ref nid) = novel_id {
        validate_id_component(nid, "novel_id")?;
    }
    if let Some(ref wid) = workspace_id {
        validate_id_component(wid, "workspace_id")?;
    }
    if let Some(ref t) = title {
        if t.trim().is_empty() {
            return Err(AppError::invalid_input("Session title cannot be empty"));
        }
        if t.len() > 500 {
            return Err(AppError::invalid_input("Session title too long (max 500 chars)"));
        }
    }
    tracing::info!(novel_id = ?novel_id, workspace_id = ?workspace_id, title = ?title, "session_create");
    let session = state.db.create_session(CreateSessionRequest { novel_id, workspace_id, title }).await?;
    tracing::info!(session_id = %session.id, "Session created");
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_list(
    state: State<'_, AppState>,
    novel_id: Option<String>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::Session>>, AppError> {
    if let Some(ref nid) = novel_id {
        validate_id_component(nid, "novel_id")?;
    }
    tracing::debug!(novel_id = ?novel_id, "session_list");
    let sessions = state.db.list_sessions(novel_id.as_deref()).await?;
    tracing::debug!(count = sessions.len(), "Sessions listed");
    Ok(IpcResponse::ok(sessions))
}

#[tauri::command]
pub async fn session_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::Session>, AppError> {
    validate_id_component(&id, "session_id")?;
    tracing::debug!(session_id = %id, "session_get");
    let session = state.db.get_session(&id).await?
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
    validate_id_component(&id, "session_id")?;
    tracing::info!(session_id = %id, "session_delete");
    let deleted = state.db.delete_session(&id).await?;
    tracing::info!(session_id = %id, deleted, "Session deleted");
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::Message>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::debug!(session_id = %session_id, "session_messages");
    let messages = state.db.list_messages(&session_id).await?;
    tracing::debug!(session_id = %session_id, count = messages.len(), "Messages listed");
    Ok(IpcResponse::ok(messages))
}
