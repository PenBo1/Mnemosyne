use tauri::State;
use crate::shared::errors::{IpcResponse, AppError};
use crate::infrastructure::db::models::{CreateNovelRequest, UpdateNovelRequest};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;

#[tauri::command]
pub async fn create_novel(
    state: State<'_, AppState>,
    workspace_id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::models::Novel>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Novel title cannot be empty"));
    }
    if title.len() > 500 {
        return Err(AppError::invalid_input("Novel title too long (max 500 chars)"));
    }
    if genre.len() > 100 {
        return Err(AppError::invalid_input("Genre too long (max 100 chars)"));
    }

    tracing::info!(workspace_id = %workspace_id, title = %title, genre = %genre, "create_novel");
    let novel = state.db.create_novel(&CreateNovelRequest {
        workspace_id,
        title,
        genre,
        platform: "local".to_string(),
        language: "zh".to_string(),
        target_chapters: 100,
        chapter_words: 3000,
    }).await?;
    tracing::info!(novel_id = %novel.id, "Novel created");
    Ok(IpcResponse::created(novel))
}

#[tauri::command]
pub async fn update_novel(
    state: State<'_, AppState>,
    id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::models::Novel>, AppError> {
    validate_id_component(&id, "novel_id")?;
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Novel title cannot be empty"));
    }
    if title.len() > 500 {
        return Err(AppError::invalid_input("Novel title too long (max 500 chars)"));
    }
    if genre.len() > 100 {
        return Err(AppError::invalid_input("Genre too long (max 100 chars)"));
    }

    let novel = state.db.update_novel(&id, &UpdateNovelRequest {
        title: Some(title),
        genre: Some(genre),
        platform: None,
        language: None,
        target_chapters: None,
        chapter_words: None,
    }).await?;
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn list_novels(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::Novel>>, AppError> {
    tracing::debug!("list_novels");
    let novels = state.db.list_novels().await?;
    tracing::debug!(count = novels.len(), "Novels listed");
    Ok(IpcResponse::ok(novels))
}

#[tauri::command]
pub async fn delete_novel(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "novel_id")?;
    tracing::info!(novel_id = %id, "delete_novel");
    let deleted = state.db.delete_novel(&id).await?;
    tracing::info!(novel_id = %id, deleted, "Novel deleted");
    Ok(IpcResponse::ok(deleted))
}
