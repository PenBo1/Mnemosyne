use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infra::db::models::CreateWorkspaceRequest;
use crate::infra::fs_utils::validate_id_component;

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, AppState>,
    req: CreateWorkspaceRequest,
) -> Result<IpcResponse<crate::infra::db::models::Workspace>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::invalid_input("Workspace name cannot be empty"));
    }
    if req.name.len() > 255 {
        return Err(AppError::invalid_input("Workspace name too long (max 255 chars)"));
    }

    let path = req.path.clone().unwrap_or_default();
    if path.is_empty() {
        return Err(AppError::missing_field("path"));
    }

    let path_buf = std::path::PathBuf::from(&path);
    if path.contains("..") || path.contains('/') && path.starts_with('/') || path.contains('\\') && path.len() > 2 && path.as_bytes()[1] == b':' {
        return Err(AppError::path_traversal());
    }
    std::fs::create_dir_all(&path_buf)
        .map_err(|e| {
            tracing::error!(error = %e, path = %path, "Failed to create workspace directory");
            AppError::file_write_error(path.clone())
        })?;

    for sub in ["chapters", "story/state", "story/snapshots", "story/drafts"] {
        std::fs::create_dir_all(path_buf.join(sub))
            .map_err(|e| {
                tracing::error!(error = %e, sub = %sub, "Failed to create workspace subdirectory");
                AppError::file_write_error(format!("{}/{}", path, sub))
            })?;
    }

    let workspace = state.db.create_workspace(req).await?;
    tracing::info!(workspace_id = %workspace.id, "Workspace created");
    Ok(IpcResponse::created(workspace))
}

#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::Workspace>>, AppError> {
    tracing::debug!("list_workspaces");
    let workspaces = state.db.list_workspaces().await?;
    tracing::debug!(count = workspaces.len(), "Workspaces listed");
    Ok(IpcResponse::ok(workspaces))
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infra::db::models::Workspace>, AppError> {
    validate_id_component(&id, "workspace_id")?;
    tracing::debug!(workspace_id = %id, "get_workspace");
    let workspace = state.db.get_workspace(&id).await?
        .ok_or_else(|| {
            tracing::warn!(workspace_id = %id, "Workspace not found");
            AppError::workspace_not_found()
        })?;
    Ok(IpcResponse::ok(workspace))
}

#[tauri::command]
pub async fn delete_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "workspace_id")?;
    tracing::info!(workspace_id = %id, "delete_workspace");
    let deleted = state.db.delete_workspace(&id).await?;
    tracing::info!(workspace_id = %id, deleted, "Workspace deleted");
    Ok(IpcResponse::ok(deleted))
}
