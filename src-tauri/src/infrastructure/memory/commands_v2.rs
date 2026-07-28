//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆 V2 命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::sync::Arc;

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::memory::core_memory::CoreMemoryStore;
use crate::infrastructure::memory::recall_memory::RecallMemoryStore;
use crate::infrastructure::memory::archival_memory::ArchivalMemoryStore;
use crate::infrastructure::db::stores::recall_memory::RecallMemoryRow;
use crate::infrastructure::db::stores::archival_memory::ArchivalMemoryRow;
use crate::shared::error::{AppError, IpcResponse};

// ── CoreMemory Commands ─────────────────────────────────────────

#[tauri::command]
pub async fn core_memory_append(
    state: State<'_, DbState>,
    role: String,
    content: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&role, "role")?;
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Content cannot be empty"));
    }
    if content.len() > 50000 {
        return Err(AppError::invalid_input("Content too long (max 50000 chars)"));
    }

    let store = CoreMemoryStore::new(Arc::new(state.data_dir.clone()));
    store.append(&role, &content).await?;
    Ok(IpcResponse::ok_void())
}

#[tauri::command]
pub async fn core_memory_replace(
    state: State<'_, DbState>,
    role: String,
    old_content: String,
    new_content: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&role, "role")?;
    if old_content.trim().is_empty() {
        return Err(AppError::invalid_input("Old content cannot be empty"));
    }

    let store = CoreMemoryStore::new(Arc::new(state.data_dir.clone()));
    let replaced = store.replace(&role, &old_content, &new_content).await?;
    Ok(IpcResponse::ok(replaced))
}

#[tauri::command]
pub async fn core_memory_load(
    state: State<'_, DbState>,
    role: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&role, "role")?;

    let store = CoreMemoryStore::new(Arc::new(state.data_dir.clone()));
    let content = store.load(&role).await?;
    Ok(IpcResponse::ok(content))
}

// ── RecallMemory Commands ─────────────────────────────────────────

#[tauri::command]
pub async fn recall_memory_insert(
    state: State<'_, DbState>,
    id: String,
    session_id: String,
    message_index: i64,
    role: String,
    content: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&id, "id")?;
    validate_id_component(&session_id, "session_id")?;
    if message_index < 0 {
        return Err(AppError::invalid_input("message_index must be >= 0"));
    }

    let store = RecallMemoryStore::new(state.db.clone());
    store.insert(&id, &session_id, message_index, &role, &content)?;
    Ok(IpcResponse::ok_void())
}

#[tauri::command]
pub async fn recall_memory_search(
    state: State<'_, DbState>,
    session_id: Option<String>,
    role: Option<String>,
    query: Option<String>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<RecallMemoryRow>>, AppError> {
    if let Some(sid) = &session_id {
        validate_id_component(sid, "session_id")?;
    }

    let store = RecallMemoryStore::new(state.db.clone());
    let limit = limit.unwrap_or(50).min(500);
    let results = store.search(
        session_id.as_deref(),
        role.as_deref(),
        query.as_deref(),
        limit,
    )?;
    Ok(IpcResponse::ok(results))
}

// ── ArchivalMemory Commands ─────────────────────────────────────────

#[tauri::command]
pub async fn archival_memory_insert(
    state: State<'_, DbState>,
    id: String,
    content: String,
    tags: Vec<String>,
    citation: Option<String>,
    embedding_vector: Option<Vec<f32>>,
    embedding_model: Option<String>,
    embedding_dim: Option<i64>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&id, "id")?;
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Content cannot be empty"));
    }
    if content.len() > 100000 {
        return Err(AppError::invalid_input("Content too long (max 100000 chars)"));
    }

    let store = ArchivalMemoryStore::new(state.db.clone());
    let tags_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    let emb_vec = embedding_vector.as_deref();
    let emb_model = embedding_model.as_deref();
    let emb_dim = embedding_dim.map(|d| d as usize);

    store.insert(
        &id,
        &content,
        &tags_refs,
        citation.as_deref(),
        emb_vec,
        emb_model,
        emb_dim,
    )?;
    Ok(IpcResponse::ok_void())
}

#[tauri::command]
pub async fn archival_memory_search(
    state: State<'_, DbState>,
    query: String,
    tags_filter: Option<Vec<String>>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<ArchivalMemoryRow>>, AppError> {
    if query.trim().is_empty() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    let store = ArchivalMemoryStore::new(state.db.clone());
    let limit = limit.unwrap_or(20).min(200);
    let tags_refs: Option<Vec<&str>> = tags_filter.as_ref().map(|t| t.iter().map(|s| s.as_str()).collect());
    let tags_slice: Option<&[&str]> = tags_refs.as_deref();

    let results = store.search_by_content(&query, tags_slice, limit)?;
    Ok(IpcResponse::ok(results))
}

#[tauri::command]
pub async fn archival_memory_get(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<Option<ArchivalMemoryRow>>, AppError> {
    validate_id_component(&id, "id")?;

    let store = ArchivalMemoryStore::new(state.db.clone());
    let result = store.get_by_id(&id)?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn archival_memory_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "id")?;

    let store = ArchivalMemoryStore::new(state.db.clone());
    let deleted = store.delete(&id)?;
    if !deleted {
        return Err(AppError::not_found(format!("Archival memory {} not found", id)));
    }
    Ok(IpcResponse::ok(deleted))
}