use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::infra::db::models::CreateWorkspaceRequest;

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, AppState>,
    req: CreateWorkspaceRequest,
) -> Result<IpcResponse<crate::infra::db::models::Workspace>, AppError> {
    tracing::info!(name = %req.name, path = ?req.path, "create_workspace");
    let path = req.path.clone().unwrap_or_default();
    if path.is_empty() {
        return Err(AppError::missing_field("path"));
    }

    let path_buf = std::path::PathBuf::from(&path);
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

    let db = state.db.lock().await;
    let workspace = db.create_workspace(req)?;
    tracing::info!(workspace_id = %workspace.id, "Workspace created");
    Ok(IpcResponse::created(workspace))
}

#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::db::models::Workspace>>, AppError> {
    tracing::debug!("list_workspaces");
    let db = state.db.lock().await;
    let workspaces = db.list_workspaces()?;
    tracing::debug!(count = workspaces.len(), "Workspaces listed");
    Ok(IpcResponse::ok(workspaces))
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<crate::infra::db::models::Workspace>, AppError> {
    tracing::debug!(workspace_id = %id, "get_workspace");
    let db = state.db.lock().await;
    let workspace = db.get_workspace(&id)?
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
    tracing::info!(workspace_id = %id, "delete_workspace");
    let db = state.db.lock().await;
    let deleted = db.delete_workspace(&id)?;
    tracing::info!(workspace_id = %id, deleted, "Workspace deleted");
    Ok(IpcResponse::ok(deleted))
}
