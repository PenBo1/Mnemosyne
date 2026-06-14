use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;

#[tauri::command]
pub async fn create_novel(
    state: State<'_, AppState>,
    workspace_id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infra::db::models::Novel>, AppError> {
    tracing::info!(workspace_id = %workspace_id, title = %title, genre = %genre, "create_novel");
    let db = state.db.lock().await;
    let novel_id = uuid::Uuid::new_v4().to_string();
    let novel = db.create_novel_with_workspace(&novel_id, &workspace_id, &title, &genre)?;
    tracing::info!(novel_id = %novel.id, "Novel created");
    Ok(IpcResponse::created(novel))
}

#[tauri::command]
pub async fn update_novel(
    state: State<'_, AppState>,
    id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infra::db::models::Novel>, AppError> {
    let db = state.db.lock().await;
    let novel = db.update_novel(&id, &title, &genre)?;
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn list_novels(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::Novel>>, AppError> {
    tracing::debug!("list_novels");
    let db = state.db.lock().await;
    let novels = db.list_novels()?;
    tracing::debug!(count = novels.len(), "Novels listed");
    Ok(IpcResponse::ok(novels))
}

#[tauri::command]
pub async fn delete_novel(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    tracing::info!(novel_id = %id, "delete_novel");
    let db = state.db.lock().await;
    let deleted = db.delete_novel(&id)?;
    tracing::info!(novel_id = %id, deleted, "Novel deleted");
    Ok(IpcResponse::ok(deleted))
}
