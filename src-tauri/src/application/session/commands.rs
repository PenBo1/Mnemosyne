
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::stores::session::{CreateSessionRequest, Session, Message, MessageMeta};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub async fn session_create(
    state: State<'_, DbState>,
    novel_id: Option<String>,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<IpcResponse<Session>, AppError> {
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
    let session = state.db.create_session(CreateSessionRequest { novel_id, workspace_id, title })?;
    tracing::info!(session_id = %session.id, "Session created");
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_list(
    state: State<'_, DbState>,
    novel_id: Option<String>,
    workspace_id: Option<String>,
) -> Result<IpcResponse<Vec<Session>>, AppError> {
    if let Some(ref nid) = novel_id {
        validate_id_component(nid, "novel_id")?;
    }
    if let Some(ref wid) = workspace_id {
        validate_id_component(wid, "workspace_id")?;
    }
    tracing::debug!(novel_id = ?novel_id, workspace_id = ?workspace_id, "session_list");
    let sessions = state.db.list_sessions(novel_id.as_deref(), workspace_id.as_deref())?;
    tracing::debug!(count = sessions.len(), "Sessions listed");
    Ok(IpcResponse::ok(sessions))
}

#[tauri::command]
pub async fn session_get(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<Session>, AppError> {
    validate_id_component(&id, "session_id")?;
    tracing::debug!(session_id = %id, "session_get");
    let session = state.db.get_session(&id)?
        .ok_or_else(|| {
            tracing::warn!(session_id = %id, "Session not found");
            AppError::session_not_found()
        })?;
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "session_id")?;
    tracing::info!(session_id = %id, "session_delete");
    let deleted = state.db.delete_session(&id)?;
    tracing::info!(session_id = %id, deleted, "Session deleted");
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn session_messages(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<Vec<Message>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::debug!(session_id = %session_id, "session_messages");
    let messages = state.db.list_messages(&session_id)?;
    tracing::debug!(session_id = %session_id, count = messages.len(), "Messages listed");
    Ok(IpcResponse::ok(messages))
}

#[derive(Debug, Deserialize)]
pub struct MessageMetaInput {
    pub thinking_content: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub latency_ms: Option<u64>,
}

#[tauri::command]
pub async fn message_create(
    state: State<'_, DbState>,
    session_id: String,
    role: String,
    content: String,
    tool_calls: Option<String>,
    tool_results: Option<String>,
    meta: Option<MessageMetaInput>,
) -> Result<IpcResponse<Message>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    if role.trim().is_empty() {
        return Err(AppError::invalid_input("Message role cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    tracing::debug!(
        session_id = %session_id,
        role = %role,
        has_meta = meta.is_some(),
        content_len = content.len(),
        "message_create"
    );

    let meta_ref = meta.as_ref().map(|m| MessageMeta {
        thinking_content: m.thinking_content.as_deref(),
        model: m.model.as_deref(),
        provider: m.provider.as_deref(),
        input_tokens: m.input_tokens.unwrap_or(0),
        output_tokens: m.output_tokens.unwrap_or(0),
        latency_ms: m.latency_ms,
    });

    let message = state
        .db
        .create_message_with_meta(
            &session_id,
            &role,
            &content,
            tool_calls.as_deref(),
            tool_results.as_deref(),
            meta_ref,
        )?;

    tracing::debug!(message_id = %message.id, "Message created");
    Ok(IpcResponse::ok(message))
}