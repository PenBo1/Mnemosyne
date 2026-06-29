use tauri::State;
use crate::shared::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infrastructure::db::models::{CreatePromptRequest, UpdatePromptRequest};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;

#[tauri::command]
pub async fn create_prompt(
    state: State<'_, AppState>,
    req: CreatePromptRequest,
) -> Result<IpcResponse<crate::infrastructure::db::models::Prompt>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::invalid_input("Prompt name cannot be empty"));
    }
    if req.name.len() > 255 {
        return Err(AppError::invalid_input("Prompt name too long (max 255 chars)"));
    }
    if req.content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Prompt content too long (max 1MB)"));
    }
    tracing::info!(name = %req.name, category = %req.category, "create_prompt");
    let prompt = state.db.create_prompt(req).await?;
    tracing::info!(prompt_id = %prompt.id, "Prompt created");
    Ok(IpcResponse::created(prompt))
}

#[tauri::command]
pub async fn list_prompts(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::Prompt>>, AppError> {
    tracing::debug!(category = ?category, "list_prompts");
    let prompts = state.db.list_prompts(category.as_deref()).await?;
    tracing::debug!(count = prompts.len(), "Prompts listed");
    Ok(IpcResponse::ok(prompts))
}

#[tauri::command]
pub async fn get_prompt(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::models::Prompt>, AppError> {
    validate_id_component(&id, "prompt_id")?;
    tracing::debug!(prompt_id = %id, "get_prompt");
    let prompt = state.db.get_prompt(&id).await?
        .ok_or_else(|| AppError::prompt_not_found())?;
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn update_prompt(
    state: State<'_, AppState>,
    req: UpdatePromptRequest,
) -> Result<IpcResponse<crate::infrastructure::db::models::Prompt>, AppError> {
    validate_id_component(&req.id, "prompt_id")?;
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::invalid_input("Prompt name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(AppError::invalid_input("Prompt name too long (max 255 chars)"));
        }
    }
    tracing::info!(prompt_id = %req.id, "update_prompt");
    let prompt = state.db.update_prompt(req).await?;
    tracing::info!("Prompt updated");
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn delete_prompt(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "prompt_id")?;
    tracing::info!(prompt_id = %id, "delete_prompt");
    let deleted = state.db.delete_prompt(&id).await?;
    tracing::info!(prompt_id = %id, deleted, "Prompt deleted");
    Ok(IpcResponse::ok(deleted))
}
