
use serde::Serialize;
use tauri::State;
use crate::infrastructure::memory::types::MemoryEntry;
use crate::infrastructure::memory::state::MemoryState;
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::fs::fs_utils::validate_id_component;

#[derive(Debug, Serialize)]
pub struct MemoryStats { pub main: usize, pub archival: usize }

#[tauri::command]
pub async fn memory_list(state: State<'_, MemoryState>, book_id: String) -> Result<IpcResponse<Vec<MemoryEntry>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let entries = state.store.list_all(&book_id).await;
    Ok(IpcResponse::ok(entries))
}

#[tauri::command]
pub async fn memory_search(state: State<'_, MemoryState>, book_id: String, query: String, top_k: Option<u32>) -> Result<IpcResponse<Vec<MemoryEntry>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    if query.trim().is_empty() { return Ok(IpcResponse::ok(Vec::new())); }
    let k = top_k.unwrap_or(10) as usize;
    let entries = state.store.search(&book_id, &query, k).await;
    Ok(IpcResponse::ok(entries))
}

#[tauri::command]
pub async fn memory_stats(state: State<'_, MemoryState>, book_id: String) -> Result<IpcResponse<MemoryStats>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let (main, archival) = state.store.stats(&book_id).await;
    Ok(IpcResponse::ok(MemoryStats { main, archival }))
}

#[tauri::command]
pub async fn memory_format_context(state: State<'_, MemoryState>, book_id: String) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let context = state.store.format_context(&book_id).await;
    Ok(IpcResponse::ok(context))
}

#[tauri::command]
pub async fn memory_create(
    state: State<'_, MemoryState>, book_id: String, content: String, entry_type: String, chapter: Option<u32>, tags: Vec<String>,
) -> Result<IpcResponse<MemoryEntry>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    if content.trim().is_empty() { return Err(AppError::invalid_input("Content cannot be empty")); }
    if content.len() > 10000 { return Err(AppError::invalid_input("Content too long (max 10000 chars)")); }
    let valid_types = ["character", "plot", "setting", "dialogue", "fact", "style"];
    if !valid_types.contains(&entry_type.as_str()) { return Err(AppError::invalid_input(format!("Invalid entry_type: {} (allowed: {:?})", entry_type, valid_types))); }
    let entry = state.store.create_manual(&book_id, content, &entry_type, chapter, tags).await;
    Ok(IpcResponse::created(entry))
}

#[tauri::command]
pub async fn memory_update(
    state: State<'_, MemoryState>, book_id: String, entry_id: String, content: String, tags: Vec<String>,
) -> Result<IpcResponse<MemoryEntry>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    validate_id_component(&entry_id, "entry_id")?;
    if content.trim().is_empty() { return Err(AppError::invalid_input("Content cannot be empty")); }
    if content.len() > 10000 { return Err(AppError::invalid_input("Content too long")); }
    let updated = state.store.update_entry(&book_id, &entry_id, content, tags).await.ok_or_else(|| AppError::not_found("Memory entry not found"))?;
    Ok(IpcResponse::ok(updated))
}

#[tauri::command]
pub async fn memory_delete(state: State<'_, MemoryState>, book_id: String, entry_id: String) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    validate_id_component(&entry_id, "entry_id")?;
    let deleted = state.store.delete_entry(&book_id, &entry_id).await;
    if !deleted { return Err(AppError::not_found("Memory entry not found")); }
    Ok(IpcResponse::ok(deleted))
}