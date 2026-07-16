use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::validate_id;
use tauri::State;

const MAX_TITLE_LEN: usize = 255;
const MAX_CONTENT_LEN: usize = 1_000_000;

fn validate_novel_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "novel_id").map_err(AppError::invalid_input)
}

fn validate_entry_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "entry_id").map_err(AppError::invalid_input)
}

fn validate_link_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "link_id").map_err(AppError::invalid_input)
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Title cannot be empty"));
    }
    if title.len() > MAX_TITLE_LEN {
        return Err(AppError::invalid_input(format!(
            "Title too long (max {} chars)", MAX_TITLE_LEN
        )));
    }
    Ok(())
}

fn validate_content(content: &str) -> Result<(), AppError> {
    if content.len() > MAX_CONTENT_LEN {
        return Err(AppError::invalid_input(format!(
            "Content too large (max {} bytes)", MAX_CONTENT_LEN
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn wiki_list_entries(
    state: State<'_, DbState>,
    novel_id: String,
    category: Option<String>,
) -> Result<IpcResponse<Vec<crate::domain::wiki::models::WikiEntry>>, AppError> {
    validate_novel_id(&novel_id)?;
    let cat = category.and_then(|c| c.parse::<crate::domain::wiki::types::WikiCategory>().ok());
    let entries = state.db.list_wiki_entries(&novel_id, cat.as_ref())?;
    Ok(IpcResponse::ok(entries))
}

#[tauri::command]
pub async fn wiki_get_entry(
    state: State<'_, DbState>,
    entry_id: String,
) -> Result<IpcResponse<crate::domain::wiki::models::WikiEntry>, AppError> {
    validate_entry_id(&entry_id)?;
    let entry = state.db.get_wiki_entry(&entry_id)?
        .ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
    Ok(IpcResponse::ok(entry))
}

#[tauri::command]
pub async fn wiki_create_entry(
    state: State<'_, DbState>,
    novel_id: String,
    title: String,
    content: String,
    category: String,
) -> Result<IpcResponse<crate::domain::wiki::models::WikiEntry>, AppError> {
    validate_novel_id(&novel_id)?;
    validate_title(&title)?;
    validate_content(&content)?;

    let cat = category.parse::<crate::domain::wiki::types::WikiCategory>()
        .map_err(|_| AppError::invalid_input(format!("Invalid category: {}", category)))?;

    let req = crate::domain::wiki::models::CreateWikiEntryRequest {
        novel_id,
        title,
        content,
        category: cat,
        tags: Vec::new(),
        source_chapter: None,
        importance: None,
    };
    let entry = state.db.create_wiki_entry(&req)?;
    Ok(IpcResponse::created(entry))
}

#[tauri::command]
pub async fn wiki_update_entry(
    state: State<'_, DbState>,
    entry_id: String,
    title: Option<String>,
    content: Option<String>,
    category: Option<String>,
) -> Result<IpcResponse<crate::domain::wiki::models::WikiEntry>, AppError> {
    validate_entry_id(&entry_id)?;
    if let Some(ref t) = title {
        validate_title(t)?;
    }
    if let Some(ref c) = content {
        validate_content(c)?;
    }

    let cat = category.and_then(|c| c.parse::<crate::domain::wiki::types::WikiCategory>().ok());
    let req = crate::domain::wiki::models::UpdateWikiEntryRequest {
        title,
        content,
        category: cat,
        tags: None,
        importance: None,
    };
    let entry = state.db.update_wiki_entry(&entry_id, &req)?;
    Ok(IpcResponse::ok(entry))
}

#[tauri::command]
pub async fn wiki_delete_entry(
    state: State<'_, DbState>,
    entry_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_entry_id(&entry_id)?;
    let deleted = state.db.delete_wiki_entry(&entry_id)?;
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn wiki_get_graph(
    state: State<'_, DbState>,
    novel_id: String,
    filter_category: Option<String>,
    min_importance: Option<u32>,
) -> Result<IpcResponse<crate::domain::wiki::models::WikiGraphView>, AppError> {
    validate_novel_id(&novel_id)?;
    let cat = filter_category.and_then(|c| c.parse::<crate::domain::wiki::types::WikiCategory>().ok());
    let graph = state.db.get_wiki_graph_view(&novel_id, cat.as_ref(), min_importance)?;
    Ok(IpcResponse::ok(graph))
}

#[tauri::command]
pub async fn wiki_create_link(
    state: State<'_, DbState>,
    novel_id: String,
    source_entry_id: String,
    target_entry_id: String,
    relation_type: String,
) -> Result<IpcResponse<crate::domain::wiki::models::WikiEntityLink>, AppError> {
    validate_novel_id(&novel_id)?;
    validate_entry_id(&source_entry_id)?;
    validate_entry_id(&target_entry_id)?;

    if relation_type.trim().is_empty() {
        return Err(AppError::invalid_input("Relation type cannot be empty"));
    }
    if relation_type.len() > 100 {
        return Err(AppError::invalid_input("Relation type too long (max 100 chars)"));
    }

    let req = crate::domain::wiki::models::CreateWikiLinkRequest {
        novel_id,
        source_entry_id,
        target_entry_id,
        relation_type,
        relation_desc: String::new(),
        weight: None,
        source_chapter: None,
    };
    let link = state.db.create_wiki_link(&req)?;
    Ok(IpcResponse::created(link))
}

#[tauri::command]
pub async fn wiki_delete_link(
    state: State<'_, DbState>,
    link_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_link_id(&link_id)?;
    let deleted = state.db.delete_wiki_link(&link_id)?;
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn wiki_search(
    state: State<'_, DbState>,
    novel_id: String,
    query: String,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<crate::domain::wiki::models::WikiEntry>>, AppError> {
    validate_novel_id(&novel_id)?;
    if query.trim().is_empty() {
        return Ok(IpcResponse::ok(Vec::new()));
    }
    let limit_val = limit.unwrap_or(20).clamp(1, 100);
    let entries = state.db.search_wiki_entries(&novel_id, &query, Some(limit_val))?;
    Ok(IpcResponse::ok(entries))
}