use tauri::State;
use crate::shared::errors::{AppError, IpcResponse};
use crate::features::wiki::{WikiEntry, WikiCategory, WikiGraphView, WikiEntityLink, CreateWikiEntryRequest, UpdateWikiEntryRequest, CreateWikiLinkRequest};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;

/// 列出某部小说的所有 wiki 条目
#[tauri::command]
pub async fn wiki_list_entries(
    state: State<'_, AppState>,
    novel_id: String,
    category: Option<String>,
) -> Result<IpcResponse<Vec<WikiEntry>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    tracing::debug!(novel_id = %novel_id, category = ?category, "wiki_list_entries");
    
    let cat = category.and_then(|c| c.parse::<WikiCategory>().ok());
    let entries = state.db.list_wiki_entries(&novel_id, cat.as_ref()).await?;
    
    tracing::debug!(count = entries.len(), "Wiki entries listed");
    Ok(IpcResponse::ok(entries))
}

/// 按 ID 获取单个 wiki 条目
#[tauri::command]
pub async fn wiki_get_entry(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<IpcResponse<WikiEntry>, AppError> {
    validate_id_component(&entry_id, "entry_id")?;
    tracing::debug!(entry_id = %entry_id, "wiki_get_entry");
    
    let entry = state.db.get_wiki_entry(&entry_id).await?
        .ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
    
    Ok(IpcResponse::ok(entry))
}

/// 创建新的 wiki 条目
#[tauri::command]
pub async fn wiki_create_entry(
    state: State<'_, AppState>,
    novel_id: String,
    title: String,
    content: String,
    category: String,
    tags: Vec<String>,
    source_chapter: Option<u32>,
    importance: Option<u32>,
) -> Result<IpcResponse<WikiEntry>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    tracing::info!(novel_id = %novel_id, title = %title, "wiki_create_entry");
    
    // 校验
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Title cannot be empty"));
    }
    if title.len() > 500 {
        return Err(AppError::invalid_input("Title too long (max 500 chars)"));
    }
    
    let category_parsed = category.parse::<WikiCategory>()
        .map_err(|e| AppError::invalid_input(e))?;
    
    let request = CreateWikiEntryRequest {
        novel_id,
        title,
        content,
        category: category_parsed,
        tags,
        source_chapter,
        importance,
    };
    
    let entry = state.db.create_wiki_entry(&request).await?;
    tracing::info!(entry_id = %entry.id, "Wiki entry created");
    Ok(IpcResponse::created(entry))
}

/// 更新已有的 wiki 条目
#[tauri::command]
pub async fn wiki_update_entry(
    state: State<'_, AppState>,
    entry_id: String,
    title: Option<String>,
    content: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    importance: Option<u32>,
) -> Result<IpcResponse<WikiEntry>, AppError> {
    validate_id_component(&entry_id, "entry_id")?;
    tracing::info!(entry_id = %entry_id, "wiki_update_entry");
    
    // 如果提供了 title 则校验
    if let Some(ref t) = title {
        if t.trim().is_empty() {
            return Err(AppError::invalid_input("Title cannot be empty"));
        }
        if t.len() > 500 {
            return Err(AppError::invalid_input("Title too long (max 500 chars)"));
        }
    }
    
    let category_parsed = category.and_then(|c| c.parse::<WikiCategory>().ok());
    
    let request = UpdateWikiEntryRequest {
        title,
        content,
        category: category_parsed,
        tags,
        importance,
    };
    
    let entry = state.db.update_wiki_entry(&entry_id, &request).await?;
    Ok(IpcResponse::ok(entry))
}

/// 删除 wiki 条目
#[tauri::command]
pub async fn wiki_delete_entry(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&entry_id, "entry_id")?;
    tracing::info!(entry_id = %entry_id, "wiki_delete_entry");
    
    let deleted = state.db.delete_wiki_entry(&entry_id).await?;
    tracing::info!(entry_id = %entry_id, deleted, "Wiki entry deleted");
    Ok(IpcResponse::ok(deleted))
}

/// 获取 wiki 图谱视图用于可视化
#[tauri::command]
pub async fn wiki_get_graph(
    state: State<'_, AppState>,
    novel_id: String,
    filter_category: Option<String>,
    min_importance: Option<u32>,
) -> Result<IpcResponse<WikiGraphView>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    tracing::debug!(novel_id = %novel_id, "wiki_get_graph");
    
    let cat = filter_category.and_then(|c| c.parse::<WikiCategory>().ok());
    let view = state.db.get_wiki_graph_view(&novel_id, cat.as_ref(), min_importance).await?;
    
    tracing::debug!(nodes = view.nodes.len(), edges = view.edges.len(), "Wiki graph view generated");
    Ok(IpcResponse::ok(view))
}

/// 创建 wiki 实体链接
#[tauri::command]
pub async fn wiki_create_link(
    state: State<'_, AppState>,
    novel_id: String,
    source_entry_id: String,
    target_entry_id: String,
    relation_type: String,
    relation_desc: String,
    weight: Option<u32>,
    source_chapter: Option<u32>,
) -> Result<IpcResponse<WikiEntityLink>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    validate_id_component(&source_entry_id, "source_entry_id")?;
    validate_id_component(&target_entry_id, "target_entry_id")?;
    tracing::info!(novel_id = %novel_id, source = %source_entry_id, target = %target_entry_id, "wiki_create_link");
    
    // 校验
    if relation_type.trim().is_empty() {
        return Err(AppError::invalid_input("Relation type cannot be empty"));
    }
    if relation_type.len() > 100 {
        return Err(AppError::invalid_input("Relation type too long (max 100 chars)"));
    }
    
    let request = CreateWikiLinkRequest {
        novel_id,
        source_entry_id,
        target_entry_id,
        relation_type,
        relation_desc,
        weight,
        source_chapter,
    };
    
    let link = state.db.create_wiki_link(&request).await?;
    tracing::info!(link_id = %link.id, "Wiki link created");
    Ok(IpcResponse::created(link))
}

/// 删除 wiki 实体链接
#[tauri::command]
pub async fn wiki_delete_link(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&link_id, "link_id")?;
    tracing::info!(link_id = %link_id, "wiki_delete_link");
    
    let deleted = state.db.delete_wiki_link(&link_id).await?;
    tracing::info!(link_id = %link_id, deleted, "Wiki link deleted");
    Ok(IpcResponse::ok(deleted))
}

/// 按关键词搜索 wiki 条目
#[tauri::command]
pub async fn wiki_search(
    state: State<'_, AppState>,
    novel_id: String,
    query: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<WikiEntry>>, AppError> {
    validate_id_component(&novel_id, "novel_id")?;
    tracing::debug!(novel_id = %novel_id, query = %query, "wiki_search");
    
    if query.trim().is_empty() {
        return Ok(IpcResponse::ok(Vec::new()));
    }
    
    let entries = state.db.search_wiki_entries(&novel_id, &query, limit).await?;
    
    tracing::debug!(count = entries.len(), "Wiki search results");
    Ok(IpcResponse::ok(entries))
}
