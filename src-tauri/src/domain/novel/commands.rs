use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::validate_id;
use crate::domain::novel::types::BookSource;
use tauri::State;

const MAX_TITLE_LEN: usize = 500;
const MAX_GENRE_LEN: usize = 100;

fn validate_novel_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "novel_id").map_err(|e| AppError::invalid_input(e))
}

fn validate_workspace_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "workspace_id").map_err(|e| AppError::invalid_input(e))
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

fn validate_genre(genre: &str) -> Result<(), AppError> {
    if genre.len() > MAX_GENRE_LEN {
        return Err(AppError::invalid_input(format!(
            "Genre too long (max {} chars)", MAX_GENRE_LEN
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn novel_create(
    state: State<'_, DbState>,
    workspace_id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    validate_workspace_id(&workspace_id)?;
    validate_title(&title)?;
    validate_genre(&genre)?;

    let req = crate::infrastructure::db::types::CreateNovelRequest {
        workspace_id,
        title,
        genre,
        platform: "local".to_string(),
        language: "zh".to_string(),
        target_chapters: 100,
        chapter_words: 3000,
    };

    let novel = state.db.create_novel(&req)?;
    Ok(IpcResponse::created(novel))
}

#[tauri::command]
pub async fn novel_update(
    state: State<'_, DbState>,
    id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    validate_novel_id(&id)?;
    validate_title(&title)?;
    validate_genre(&genre)?;

    let req = crate::infrastructure::db::types::UpdateNovelRequest {
        title: Some(title),
        genre: Some(genre),
        platform: None,
        language: None,
        target_chapters: None,
        chapter_words: None,
    };

    let novel = state.db.update_novel(&id, &req)?;
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn novel_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Novel>>, AppError> {
    let novels = state.db.list_novels()?;
    Ok(IpcResponse::ok(novels))
}

#[tauri::command]
pub async fn novel_get(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    validate_novel_id(&id)?;
    let novel = state.db.get_novel_by_id(&id)?
        .ok_or_else(|| AppError::not_found("Novel not found"))?;
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn novel_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_novel_id(&id)?;
    let deleted = state.db.delete_novel(&id)?;
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn novel_source_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<BookSource>>, AppError> {
    let sources_path = state.data_dir.root().join("novel_sources.json");
    
    if !sources_path.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }
    
    let content = std::fs::read_to_string(&sources_path)
        .map_err(|_| AppError::file_read_error(sources_path.display().to_string()))?;
    
    let sources: Vec<BookSource> = serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse novel sources: {}", e)))?;
    
    Ok(IpcResponse::ok(sources))
}

#[tauri::command]
pub async fn novel_source_toggle(
    state: State<'_, DbState>,
    name: String,
    enabled: bool,
) -> Result<IpcResponse<bool>, AppError> {
    let sources_path = state.data_dir.root().join("novel_sources.json");
    
    if !sources_path.exists() {
        return Err(AppError::not_found("Novel sources file not found"));
    }
    
    let content = std::fs::read_to_string(&sources_path)
        .map_err(|_| AppError::file_read_error(sources_path.display().to_string()))?;
    
    let mut sources: Vec<BookSource> = serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse novel sources: {}", e)))?;
    
    let mut updated = false;
    
    for source in &mut sources {
        if source.name == name {
            source.disabled = !enabled;
            updated = true;
            break;
        }
    }
    
    if !updated {
        return Err(AppError::not_found(format!("Source '{}' not found", name)));
    }
    
    let json = serde_json::to_string_pretty(&sources)
        .map_err(|e| AppError::internal(format!("Failed to serialize novel sources: {}", e)))?;
    
    std::fs::write(&sources_path, json)
        .map_err(|_| AppError::file_write_error(sources_path.display().to_string()))?;
    
    Ok(IpcResponse::ok(true))
}