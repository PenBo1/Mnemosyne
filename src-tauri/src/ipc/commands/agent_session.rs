use crate::shared::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::features::session::{Op, SessionStatus};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use tauri::State;

/// 向 agent session 发送消息（非阻塞）。
#[tauri::command]
pub async fn session_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Message content cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    state.ensure_session(&session_id).await?;

    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::UserInput(
            crate::features::session::op::UserInputPayload {
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

/// 取消 session 中当前的操作。
#[tauri::command]
pub async fn session_cancel(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.cancel(Some("User cancelled".to_string())).await?;
        tracing::info!(session_id = %session_id, "Session cancel requested");
    }
    Ok(IpcResponse::ok(()))
}

/// 获取 session 的当前状态。
#[tauri::command]
pub async fn session_get_status(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<SessionStatus>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let sessions = state.sessions.lock().await;
    let status = sessions.get(&session_id)
        .map(|s| s.status())
        .unwrap_or(SessionStatus::Shutdown);
    Ok(IpcResponse::ok(status))
}

/// 优雅关闭 session。
#[tauri::command]
pub async fn session_shutdown(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.remove(&session_id) {
        session.shutdown().await?;
        tracing::info!(session_id = %session_id, "Session shutdown requested");
    }
    Ok(IpcResponse::ok(()))
}

/// 向 session 提交 pipeline 操作（写下一章）。
#[tauri::command]
pub async fn session_write_next_chapter(
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    book_id: String,
    target_words: Option<u32>,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    validate_id_component(&workspace_id, "workspace_id")?;
    validate_id_component(&book_id, "book_id")?;
    if let Some(tw) = target_words {
        if tw == 0 || tw > 100_000 {
            return Err(AppError::invalid_input("target_words must be between 1 and 100000"));
        }
    }

    state.ensure_session(&session_id).await?;

    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::WriteNextChapter(
            crate::features::session::op::WriteNextChapterPayload {
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

/// 向 session 提交创建书籍的操作。
#[tauri::command]
pub async fn session_create_book(
    state: State<'_, AppState>,
    session_id: String,
    workspace_id: String,
    title: String,
    genre: String,
    brief: Option<String>,
    target_chapters: Option<u32>,
    chapter_words: Option<u32>,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    validate_id_component(&workspace_id, "workspace_id")?;
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Book title cannot be empty"));
    }
    if title.len() > 500 {
        return Err(AppError::invalid_input("Book title too long (max 500 chars)"));
    }
    if genre.len() > 100 {
        return Err(AppError::invalid_input("Genre too long (max 100 chars)"));
    }
    if let Some(ref b) = brief {
        if b.len() > 10_000 {
            return Err(AppError::invalid_input("Brief too long (max 10000 chars)"));
        }
    }
    // 业务约束：与 novel_create 保持一致
    let target_chapters = target_chapters.map(|n| n.clamp(1, 10_000));
    let chapter_words = chapter_words.map(|n| n.clamp(500, 20_000));

    state.ensure_session(&session_id).await?;

    let submission_id = {
        let sessions = state.sessions.lock().await;
        let session = sessions.get(&session_id)
            .ok_or_else(|| AppError::internal("Session not found after creation"))?;
        session.submit(Op::CreateBook(
            crate::features::session::op::CreateBookPayload {
                workspace_id,
                title,
                genre,
                brief,
                target_chapters,
                chapter_words,
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
    validate_id_component(&session_id, "session_id")?;
    validate_id_component(&tool_call_id, "tool_call_id")?;
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.approve_tool(&tool_call_id).await?;
    }
    Ok(IpcResponse::ok(()))
}

/// 拒绝待执行的 tool 调用。
#[tauri::command]
pub async fn session_reject_tool(
    state: State<'_, AppState>,
    session_id: String,
    tool_call_id: String,
    reason: Option<String>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    validate_id_component(&tool_call_id, "tool_call_id")?;
    let sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get(&session_id) {
        session.reject_tool(&tool_call_id, reason).await?;
    }
    Ok(IpcResponse::ok(()))
}
