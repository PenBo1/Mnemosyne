use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infra::db::models::{CreatePromptRequest, UpdatePromptRequest};

#[tauri::command]
pub async fn create_prompt(
    state: State<'_, AppState>,
    req: CreatePromptRequest,
) -> Result<IpcResponse<crate::infra::db::models::Prompt>, AppError> {
    tracing::info!(name = %req.name, category = %req.category, "create_prompt");
    let db = state.db.lock().await;
    let prompt = db.create_prompt(req)?;
    tracing::info!(prompt_id = %prompt.id, "Prompt created");
    Ok(IpcResponse::created(prompt))
}

#[tauri::command]
pub async fn list_prompts(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::Prompt>>, AppError> {
    tracing::debug!(category = ?category, "list_prompts");
    let db = state.db.lock().await;
    let prompts = db.list_prompts(category.as_deref())?;
    tracing::debug!(count = prompts.len(), "Prompts listed");
    Ok(IpcResponse::ok(prompts))
}

#[tauri::command]
pub async fn get_prompt(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infra::db::models::Prompt>, AppError> {
    tracing::debug!(prompt_id = %id, "get_prompt");
    let db = state.db.lock().await;
    let prompt = db.get_prompt(&id)?
        .ok_or_else(|| AppError::prompt_not_found())?;
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn update_prompt(
    state: State<'_, AppState>,
    req: UpdatePromptRequest,
) -> Result<IpcResponse<crate::infra::db::models::Prompt>, AppError> {
    tracing::info!(prompt_id = %req.id, "update_prompt");
    let db = state.db.lock().await;
    let prompt = db.update_prompt(req)?;
    tracing::info!("Prompt updated");
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn delete_prompt(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    tracing::info!(prompt_id = %id, "delete_prompt");
    let db = state.db.lock().await;
    let deleted = db.delete_prompt(&id)?;
    tracing::info!(prompt_id = %id, deleted, "Prompt deleted");
    Ok(IpcResponse::ok(deleted))
}
