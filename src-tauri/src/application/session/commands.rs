
//! ═══════════════════════════════════════════════════════════════════════════
//! Commands - 会话 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供会话管理的 IPC 命令实现：
//! - session_create：创建会话
//! - session_split：分裂会话
//! - session_list：列出会话
//! - session_list_archived：列出已归档会话
//! - session_get：获取会话
//! - session_delete：删除会话
//! - session_archive：归档会话
//! - session_restore：恢复会话
//! - session_messages：获取消息列表
//! - message_create：创建消息
//! - session_search：搜索消息
//! - short_term_memory_regenerate：重新生成短期记忆

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::stores::session::{CreateSessionRequest, Session, Message, MessageMeta, MessageSearchResult, SessionSplitType};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::core::agent::commands::AgentState;
use serde::Deserialize;
use tauri::State;
use std::time::Instant;

#[tauri::command]
pub async fn session_create(
    state: State<'_, DbState>,
    novel_id: Option<String>,
    workspace_id: Option<String>,
    title: Option<String>,
) -> Result<IpcResponse<Session>, AppError> {
    let start = Instant::now();
    tracing::info!(
        novel_id = ?novel_id,
        workspace_id = ?workspace_id,
        title = ?title,
        "session_create: enter"
    );
    
    if let Some(ref nid) = novel_id {
        validate_id_component(nid, "novel_id")?;
    }
    if let Some(ref wid) = workspace_id {
        validate_id_component(wid, "workspace_id")?;
    }
    if let Some(ref t) = title {
        if t.trim().is_empty() {
            tracing::error!("session_create: Session title cannot be empty");
            return Err(AppError::invalid_input("Session title cannot be empty"));
        }
        if t.len() > 500 {
            tracing::error!(len = t.len(), "session_create: Session title too long");
            return Err(AppError::invalid_input("Session title too long (max 500 chars)"));
        }
    }
    
    let session = state.db.create_session(CreateSessionRequest { novel_id, workspace_id, title }).map_err(|e| {
        tracing::error!(error = %e, "session_create: Failed to create session");
        e
    })?;
    
    tracing::info!(
        session_id = %session.id,
        duration_ms = start.elapsed().as_millis(),
        "session_create: exit"
    );
    Ok(IpcResponse::ok(session))
}

/// session_split 的前端入参（splitType 由字符串解析为枚举）
#[derive(Debug, Deserialize)]
pub struct SessionSplitInput {
    /// 父 session ID
    pub parent_id: String,
    /// 分裂类型："branch" | "compression" | "delegate"
    pub split_type: String,
    /// 分裂原因（可选）
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn session_split(
    state: State<'_, DbState>,
    input: SessionSplitInput,
) -> Result<IpcResponse<Session>, AppError> {
    let start = Instant::now();
    tracing::info!(
        parent_id = %input.parent_id,
        split_type = %input.split_type,
        "session_split: enter"
    );

    validate_id_component(&input.parent_id, "parent_id")?;

    let split_type = match input.split_type.as_str() {
        "branch" => SessionSplitType::Branch,
        "compression" => SessionSplitType::Compression,
        "delegate" => SessionSplitType::Delegate,
        other => {
            tracing::error!(split_type = other, "session_split: Invalid split_type");
            return Err(AppError::invalid_input(format!(
                "Invalid split_type: {} (expected branch/compression/delegate)",
                other
            )));
        }
    };

    if let Some(ref r) = input.reason {
        if r.len() > 2000 {
            tracing::error!(len = r.len(), "session_split: reason too long");
            return Err(AppError::invalid_input("Split reason too long (max 2000 chars)"));
        }
    }

    let session = state
        .db
        .create_session_split(&input.parent_id, split_type, input.reason.as_deref())
        .map_err(|e| {
            tracing::error!(parent_id = %input.parent_id, error = %e, "session_split: Failed to create split session");
            e
        })?;

    tracing::info!(
        session_id = %session.id,
        parent_id = %input.parent_id,
        duration_ms = start.elapsed().as_millis(),
        "session_split: exit"
    );
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_list(
    state: State<'_, DbState>,
    novel_id: Option<String>,
    workspace_id: Option<String>,
) -> Result<IpcResponse<Vec<Session>>, AppError> {
    let start = Instant::now();
    tracing::info!(
        novel_id = ?novel_id,
        workspace_id = ?workspace_id,
        "session_list: enter"
    );
    
    if let Some(ref nid) = novel_id {
        validate_id_component(nid, "novel_id")?;
    }
    if let Some(ref wid) = workspace_id {
        validate_id_component(wid, "workspace_id")?;
    }
    
    let sessions = state.db.list_sessions(novel_id.as_deref(), workspace_id.as_deref()).map_err(|e| {
        tracing::error!(error = %e, "session_list: Failed to list sessions");
        e
    })?;
    
    tracing::info!(
        count = sessions.len(),
        duration_ms = start.elapsed().as_millis(),
        "session_list: exit"
    );
    Ok(IpcResponse::ok(sessions))
}

#[tauri::command]
pub async fn session_get(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<Session>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %id, "session_get: enter");
    
    validate_id_component(&id, "session_id")?;
    
    let session = state.db.get_session(&id).map_err(|e| {
        tracing::error!(session_id = %id, error = %e, "session_get: Failed to get session");
        e
    })?.ok_or_else(|| {
        tracing::error!(session_id = %id, "session_get: Session not found");
        AppError::session_not_found()
    })?;
    
    tracing::info!(
        session_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "session_get: exit"
    );
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %id, "session_delete: enter");
    
    validate_id_component(&id, "session_id")?;
    
    let deleted = state.db.delete_session(&id).map_err(|e| {
        tracing::error!(session_id = %id, error = %e, "session_delete: Failed to delete session");
        e
    })?;
    
    tracing::info!(
        session_id = %id,
        deleted,
        duration_ms = start.elapsed().as_millis(),
        "session_delete: exit"
    );
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn session_list_archived(
    state: State<'_, DbState>,
    workspace_id: Option<String>,
) -> Result<IpcResponse<Vec<Session>>, AppError> {
    let start = Instant::now();
    tracing::info!(
        workspace_id = ?workspace_id,
        "session_list_archived: enter"
    );
    
    if let Some(ref wid) = workspace_id {
        validate_id_component(wid, "workspace_id")?;
    }
    
    let sessions = state.db.list_archived_sessions(workspace_id.as_deref()).map_err(|e| {
        tracing::error!(error = %e, "session_list_archived: Failed to list archived sessions");
        e
    })?;
    
    tracing::info!(
        count = sessions.len(),
        duration_ms = start.elapsed().as_millis(),
        "session_list_archived: exit"
    );
    Ok(IpcResponse::ok(sessions))
}

#[tauri::command]
pub async fn session_archive(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<Session>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %id, "session_archive: enter");
    
    validate_id_component(&id, "session_id")?;
    
    let session = state.db.archive_session(&id).map_err(|e| {
        tracing::error!(session_id = %id, error = %e, "session_archive: Failed to archive session");
        e
    })?;
    
    tracing::info!(
        session_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "session_archive: exit"
    );
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn session_restore(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<Session>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %id, "session_restore: enter");
    
    validate_id_component(&id, "session_id")?;
    
    let session = state.db.restore_session(&id).map_err(|e| {
        tracing::error!(session_id = %id, error = %e, "session_restore: Failed to restore session");
        e
    })?;
    
    tracing::info!(
        session_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "session_restore: exit"
    );
    Ok(IpcResponse::ok(session))
}

#[tauri::command]
pub async fn update_session_sort_order(
    state: State<'_, DbState>,
    ids: Vec<String>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(count = ids.len(), "update_session_sort_order: enter");
    
    for id in &ids {
        validate_id_component(id, "session_id")?;
    }
    
    state.db.update_session_sort_order(&ids).map_err(|e| {
        tracing::error!(error = %e, "update_session_sort_order: Failed to update sort order");
        e
    })?;
    
    tracing::info!(
        count = ids.len(),
        duration_ms = start.elapsed().as_millis(),
        "update_session_sort_order: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn session_messages(
    state: State<'_, DbState>,
    session_id: String,
) -> Result<IpcResponse<Vec<Message>>, AppError> {
    let start = Instant::now();
    tracing::info!(session_id = %session_id, "session_messages: enter");
    
    validate_id_component(&session_id, "session_id")?;
    
    let messages = state.db.list_messages(&session_id).map_err(|e| {
        tracing::error!(session_id = %session_id, error = %e, "session_messages: Failed to list messages");
        e
    })?;
    
    tracing::info!(
        session_id = %session_id,
        count = messages.len(),
        duration_ms = start.elapsed().as_millis(),
        "session_messages: exit"
    );
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
    let start = Instant::now();
    tracing::info!(
        session_id = %session_id,
        role = %role,
        content_len = content.len(),
        has_meta = meta.is_some(),
        "message_create: enter"
    );
    
    validate_id_component(&session_id, "session_id")?;
    if role.trim().is_empty() {
        tracing::error!("message_create: Message role cannot be empty");
        return Err(AppError::invalid_input("Message role cannot be empty"));
    }
    if content.len() > 1_000_000 {
        tracing::error!(len = content.len(), "message_create: Message content too long");
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

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
        )
        .map_err(|e| {
            tracing::error!(session_id = %session_id, error = %e, "message_create: Failed to create message");
            e
        })?;

    tracing::info!(
        message_id = %message.id,
        duration_ms = start.elapsed().as_millis(),
        "message_create: exit"
    );
    Ok(IpcResponse::ok(message))
}

/// 全文搜索会话消息（基于 FTS5 trigram tokenizer，支持中文子串匹配）
///
/// - `session_id` 为 None 时搜索全部会话，否则限定指定会话
/// - `query` 经 FTS5 转义后作为 phrase 查询（trigram 要求长度 >= 3 才能命中）
/// - 按 bm25 相关性排序，返回 snippet 与 score
#[tauri::command]
pub async fn session_search(
    state: State<'_, DbState>,
    session_id: Option<String>,
    query: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<MessageSearchResult>>, AppError> {
    let start = Instant::now();
    tracing::info!(
        session_id = ?session_id,
        query_len = query.len(),
        limit = ?limit,
        "session_search: enter"
    );

    if let Some(ref sid) = session_id {
        validate_id_component(sid, "session_id")?;
    }
    let trimmed = query.trim();
    if trimmed.is_empty() {
        tracing::debug!("session_search: empty query, return empty");
        return Ok(IpcResponse::ok(Vec::new()));
    }
    // 限制 limit 范围，避免过大结果集
    let limit_val = limit.unwrap_or(20).clamp(1, 100);

    let db = state.db.clone();
    let sid_clone = session_id.clone();
    let query_clone = trimmed.to_string();
    let results = tokio::task::spawn_blocking(move || {
        db.search_messages(sid_clone.as_deref(), &query_clone, limit_val)
    })
    .await
    .map_err(|e| AppError::internal(format!("Database task join failed: {}", e)))??;

    tracing::info!(
        count = results.len(),
        duration_ms = start.elapsed().as_millis(),
        "session_search: exit"
    );
    Ok(IpcResponse::ok(results))
}

/// 主动为 session 重新生成短期记忆摘要(用户点击"重新生成"时触发)
///
/// 通过 AgentState 获取 AgentEngine 实例,业务逻辑在 AgentEngine.summarize_session 内部。
/// 放在 application 层以避免 infrastructure → core/agent 反向依赖。
#[tauri::command]
pub async fn short_term_memory_regenerate(
    state: State<'_, AgentState>,
    session_id: String,
    book_id: Option<String>,
    agent_role: Option<String>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        session_id = %session_id,
        book_id = ?book_id,
        agent_role = ?agent_role,
        "short_term_memory_regenerate: enter"
    );
    
    validate_id_component(&session_id, "session_id")?;
    if let Some(b) = book_id.as_deref() {
        validate_id_component(b, "book_id")?;
    }
    
    state
        .engine
        .summarize_session(&session_id, book_id.as_deref(), agent_role.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(session_id = %session_id, error = %e, "short_term_memory_regenerate: Failed to summarize session");
            e
        })?;
    
    tracing::info!(
        session_id = %session_id,
        duration_ms = start.elapsed().as_millis(),
        "short_term_memory_regenerate: exit"
    );
    Ok(IpcResponse::ok(true))
}