
use serde::Serialize;
use tauri::State;
use crate::infrastructure::memory::types::{MemoryEntry, MemoryRetrievalRequest, MemoryRetrievalResult};
use crate::infrastructure::memory::state::MemoryState;
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::db::stores::memory_archive::MemoryArchiveRow;

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

/// P2.2: 批量检索记忆选择（facts + summaries），通过 SQLite 加速。
///
/// 前端 retrieveMemorySelection 优先调用此命令。如果返回 hasData=false
/// 或调用失败，前端降级到 markdown/JSON 文件读取（显式 log warning）。
///
/// 参数使用 camelCase（前端惯例），Rust 自动转换 snake_case。
#[tauri::command]
pub async fn memory_retrieve_selection(
    state: State<'_, MemoryState>,
    request: MemoryRetrievalRequest,
) -> Result<IpcResponse<MemoryRetrievalResult>, AppError> {
    validate_id_component(&request.book_id, "book_id")?;
    if let Some(ch) = request.chapter_number {
        if ch < 0 {
            return Err(AppError::invalid_input("chapterNumber must be >= 0"));
        }
    }
    let result = state.store.retrieve_selection(&request)?;
    Ok(IpcResponse::ok(result))
}

/// P2.2: 简化版 —— 按章节号检索 facts + summaries（所有类别 include=true）。
///
/// 适用于不需要精细控制 include_* 的调用方。固定 maxItemsPerCategory=20。
#[tauri::command]
pub async fn memory_retrieve_for_chapter(
    state: State<'_, MemoryState>,
    book_id: String,
    chapter_number: i64,
) -> Result<IpcResponse<MemoryRetrievalResult>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    if chapter_number < 0 {
        return Err(AppError::invalid_input("chapterNumber must be >= 0"));
    }
    let req = MemoryRetrievalRequest {
        book_id,
        chapter_number: Some(chapter_number),
        goal: None,
        include_facts: true,
        include_hooks: true,
        include_summaries: true,
        include_volume_summaries: true,
        max_items_per_category: 20,
    };
    let result = state.store.retrieve_selection(&req)?;
    Ok(IpcResponse::ok(result))
}

// ── P2.4 MEMORY.md 归档管理 IPC ──────────────────────────────
//
// daily_summary 任务的 archive_old_memory 在 MEMORY.md 超 100KB 时
// 导出旧内容到 MEMORY.archive.<date>.md,并向 memory_archives 表写入元数据。
// 以下命令让前端可以 list/search/read/delete 历史归档。
//
// 归档文件路径:<data_dir>/agents/<role>/MEMORY.archive.<date>.md
// role 必须经过 validate_id_component 校验(防止路径遍历)。

/// 列出某 role 的所有归档元数据,按 archived_at 倒序(最新在前)
#[tauri::command]
pub async fn memory_list_archives(
    state: State<'_, MemoryState>,
    role: String,
) -> Result<IpcResponse<Vec<MemoryArchiveRow>>, AppError> {
    validate_id_component(&role, "role")?;
    let rows = state.store.db().list_archives(&role)?;
    Ok(IpcResponse::ok(rows))
}

/// 列出所有 role 的归档元数据(用于跨 role 全局视图)
#[tauri::command]
pub async fn memory_list_all_archives(
    state: State<'_, MemoryState>,
) -> Result<IpcResponse<Vec<MemoryArchiveRow>>, AppError> {
    let rows = state.store.db().list_all_archives()?;
    Ok(IpcResponse::ok(rows))
}

/// 在 content_summary 中搜索某 role 的归档(LIKE %query%)
#[tauri::command]
pub async fn memory_search_archives(
    state: State<'_, MemoryState>,
    role: String,
    query: String,
) -> Result<IpcResponse<Vec<MemoryArchiveRow>>, AppError> {
    validate_id_component(&role, "role")?;
    if query.len() > 500 {
        return Err(AppError::invalid_input("Query too long (max 500 chars)"));
    }
    let rows = state.store.db().search_archives(&role, &query)?;
    Ok(IpcResponse::ok(rows))
}

/// 读取归档文件内容(按 role + archive_file 名定位)
#[tauri::command]
pub async fn memory_read_archive(
    state: State<'_, MemoryState>,
    role: String,
    archive_file: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&role, "role")?;
    validate_id_component(&archive_file, "archive_file")?;
    // 归档文件名必须形如 MEMORY.archive.<date>.md,防止访问任意文件
    if !archive_file.starts_with("MEMORY.archive.") || !archive_file.ends_with(".md") {
        return Err(AppError::invalid_input(
            "archive_file must match pattern MEMORY.archive.<date>.md",
        ));
    }
    let path = state
        .store
        .data_dir()
        .join("agents")
        .join(&role)
        .join(&archive_file);
    if !path.exists() {
        return Err(AppError::not_found(format!(
            "Archive file not found: {archive_file}",
        )));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| {
        AppError::internal(format!("Failed to read archive {}: {}", path.display(), e))
    })?;
    Ok(IpcResponse::ok(content))
}

/// 删除归档:同时删除磁盘文件和 DB 元数据记录
///
/// 按 id 查找记录获取 role/archive_file,然后删文件 + 删 DB 行。
/// 文件不存在但 DB 有记录时,仍删 DB 记录(清理孤儿元数据)。
#[tauri::command]
pub async fn memory_delete_archive(
    state: State<'_, MemoryState>,
    id: i64,
) -> Result<IpcResponse<bool>, AppError> {
    let row = state
        .store
        .db()
        .get_archive_by_id(id)?
        .ok_or_else(|| AppError::not_found(format!("Archive id {} not found", id)))?;

    // 删磁盘文件(不存在视为已删,不报错)
    let file_path = state
        .store
        .data_dir()
        .join("agents")
        .join(&row.role)
        .join(&row.archive_file);
    if file_path.exists() {
        if let Err(e) = std::fs::remove_file(&file_path) {
            return Err(AppError::internal(format!(
                "Failed to delete archive file {}: {}",
                file_path.display(),
                e
            )));
        }
    }

    // 删 DB 元数据记录
    let deleted = state.store.db().delete_archive(id)?;
    Ok(IpcResponse::ok(deleted))
}