//! Memory 命令 —— 暴露 MemoryStore 的检索/统计/CRUD 能力给前端。
//!
//! MemoryStore 是 Agent 跨章节持久化的记忆系统（P14.07 MemGPT）。
//! 前端通过这些命令查看/搜索/清理 Agent 归档的事实条目。

use serde::Serialize;
use tauri::State;

use crate::core::agent::MemoryEntry;
use crate::shared::errors::{AppError, IpcResponse};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;

/// memory 统计信息
#[derive(Debug, Serialize)]
pub struct MemoryStats {
    /// 主上下文条目数
    pub main: usize,
    /// 归档存储条目数
    pub archival: usize,
}

/// 列出某本书的全部 memory 条目（main_context + archival_store）
#[tauri::command]
pub async fn memory_list(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<Vec<MemoryEntry>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    tracing::debug!(book_id = %book_id, "memory_list");

    let entries = state.memory_store.list_all(&book_id).await;
    tracing::debug!(count = entries.len(), "Memory entries listed");
    Ok(IpcResponse::ok(entries))
}

/// 按查询搜索 memory 条目（BM25 风格）
#[tauri::command]
pub async fn memory_search(
    state: State<'_, AppState>,
    book_id: String,
    query: String,
    top_k: Option<u32>,
) -> Result<IpcResponse<Vec<MemoryEntry>>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    tracing::debug!(book_id = %book_id, query = %query, "memory_search");

    if query.trim().is_empty() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    let k = top_k.unwrap_or(10) as usize;
    let entries = state.memory_store.search(&book_id, &query, k).await;
    tracing::debug!(count = entries.len(), "Memory search results");
    Ok(IpcResponse::ok(entries))
}

/// 获取 memory 统计信息
#[tauri::command]
pub async fn memory_stats(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<MemoryStats>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    tracing::debug!(book_id = %book_id, "memory_stats");

    let (main, archival) = state.memory_store.stats(&book_id).await;
    Ok(IpcResponse::ok(MemoryStats { main, archival }))
}

/// 获取格式化的主上下文字符串（用于注入 prompt）
#[tauri::command]
pub async fn memory_format_context(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    tracing::debug!(book_id = %book_id, "memory_format_context");

    let context = state.memory_store.format_context(&book_id).await;
    Ok(IpcResponse::ok(context))
}

/// 用户手动创建 memory 条目
#[tauri::command]
pub async fn memory_create(
    state: State<'_, AppState>,
    book_id: String,
    content: String,
    entry_type: String,
    chapter: Option<u32>,
    tags: Vec<String>,
) -> Result<IpcResponse<MemoryEntry>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    tracing::info!(book_id = %book_id, entry_type = %entry_type, "memory_create");

    // 校验
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Content cannot be empty"));
    }
    if content.len() > 10000 {
        return Err(AppError::invalid_input("Content too long (max 10000 chars)"));
    }

    let valid_types = ["character", "plot", "setting", "dialogue", "fact", "style"];
    if !valid_types.contains(&entry_type.as_str()) {
        return Err(AppError::invalid_input(format!(
            "Invalid entry_type: {} (allowed: {:?})",
            entry_type, valid_types
        )));
    }

    let entry = state
        .memory_store
        .create_manual(&book_id, content, &entry_type, chapter, tags)
        .await;
    tracing::info!(entry_id = %entry.id, "Memory entry created");
    Ok(IpcResponse::created(entry))
}

/// 更新已有的 memory 条目（content + tags）
#[tauri::command]
pub async fn memory_update(
    state: State<'_, AppState>,
    book_id: String,
    entry_id: String,
    content: String,
    tags: Vec<String>,
) -> Result<IpcResponse<MemoryEntry>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    validate_id_component(&entry_id, "entry_id")?;
    tracing::info!(book_id = %book_id, entry_id = %entry_id, "memory_update");

    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Content cannot be empty"));
    }
    if content.len() > 10000 {
        return Err(AppError::invalid_input("Content too long (max 10000 chars)"));
    }

    let updated = state
        .memory_store
        .update_entry(&book_id, &entry_id, content, tags)
        .await
        .ok_or_else(|| AppError::not_found("Memory entry not found"))?;
    Ok(IpcResponse::ok(updated))
}

/// 删除 memory 条目
#[tauri::command]
pub async fn memory_delete(
    state: State<'_, AppState>,
    book_id: String,
    entry_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    validate_id_component(&entry_id, "entry_id")?;
    tracing::info!(book_id = %book_id, entry_id = %entry_id, "memory_delete");

    let deleted = state.memory_store.delete_entry(&book_id, &entry_id).await;
    if !deleted {
        return Err(AppError::not_found("Memory entry not found"));
    }
    tracing::info!(entry_id = %entry_id, deleted, "Memory entry deleted");
    Ok(IpcResponse::ok(deleted))
}
