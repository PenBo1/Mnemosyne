use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::domain::session::{Op, SessionStatus};
use tauri::State;

/// New SQ/EQ-based agent commands.
///
/// These commands use the Session struct with Submission Queue / Event Queue
/// pattern, modeled after Codex CLI's architecture.
///
/// Instead of blocking the Tauri command handler until the entire pipeline
/// completes, these commands:
/// 1. Submit an Op to the session's submission queue
/// 2. Return immediately with a submission ID
/// 3. The session emits Events on the event queue
/// 4. The frontend listens for events via Tauri event system

/// Send a message to the agent session (non-blocking).
///
/// Returns immediately with a submission ID. The actual processing happens
/// in the background submission loop. Events are emitted via Tauri's
/// event system and can be listened to by the frontend.
#[tauri::command]
pub async fn session_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Message content cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    // Ensure session exists
    state.ensure_session(&session_id).await?;

    // Get the session and submit
    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::UserInput(
            crate::domain::session::op::UserInputPayload {
                session_id: session_id.clone(),
                content,
            },
        )).await?
    };

    tracing::info!(
        session_id = %session_id,
        submission_id = %submission_id.0,
        "Message submitted to session"
    );

    Ok(IpcResponse::ok(submission_id.0))
}

/// Cancel the current operation in a session.
#[tauri::command]
pub async fn session_cancel(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.cancel(Some("User cancelled".to_string())).await?;
        tracing::info!(session_id = %session_id, "Session cancel requested");
    }
    Ok(IpcResponse::ok(()))
}

/// Get the current status of a session.
#[tauri::command]
pub async fn session_get_status(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<SessionStatus>, AppError> {
    let sessions = state.sessions.lock().await;
    let status = sessions.get(&session_id)
        .map(|s| s.status())
        .unwrap_or(SessionStatus::Shutdown);
    Ok(IpcResponse::ok(status))
}

/// Shut down a session gracefully.
#[tauri::command]
pub async fn session_shutdown(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.remove(&session_id) {
        session.shutdown().await?;
        tracing::info!(session_id = %session_id, "Session shutdown requested");
    }
    Ok(IpcResponse::ok(()))
}

/// Submit a pipeline operation (write next chapter) to the session.
#[tauri::command]
pub async fn session_write_next_chapter(
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    book_id: String,
    target_words: Option<u32>,
) -> Result<IpcResponse<String>, AppError> {
    // Ensure session exists
    state.ensure_session(&session_id).await?;

    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::WriteNextChapter(
            crate::domain::session::op::WriteNextChapterPayload {
                workspace_id,
                book_id,
                target_words,
            },
        )).await?
    };

    tracing::info!(
        session_id = %session_id,
        submission_id = %submission_id.0,
        "Write next chapter submitted"
    );

    Ok(IpcResponse::ok(submission_id.0))
}

/// Submit a book creation operation to the session.
#[tauri::command]
pub async fn session_create_book(
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    title: String,
    genre: String,
    brief: Option<String>,
) -> Result<IpcResponse<String>, AppError> {
    // Ensure session exists
    state.ensure_session(&session_id).await?;

    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::CreateBook(
            crate::domain::session::op::CreateBookPayload {
                workspace_id,
                title,
                genre,
                brief,
            },
        )).await?
    };

    tracing::info!(
        session_id = %session_id,
        submission_id = %submission_id.0,
        "Create book submitted"
    );

    Ok(IpcResponse::ok(submission_id.0))
}

/// Approve a pending tool execution.
#[tauri::command]
pub async fn session_approve_tool(
    state: State<'_, AppState>,
    session_id: String,
    tool_call_id: String,
) -> Result<IpcResponse<()>, AppError> {
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.approve_tool(&tool_call_id).await?;
    }
    Ok(IpcResponse::ok(()))
}

/// Reject a pending tool execution.
#[tauri::command]
pub async fn session_reject_tool(
    state: State<'_, AppState>,
    session_id: String,
    tool_call_id: String,
    reason: Option<String>,
) -> Result<IpcResponse<()>, AppError> {
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.reject_tool(&tool_call_id, reason).await?;
    }
    Ok(IpcResponse::ok(()))
}
