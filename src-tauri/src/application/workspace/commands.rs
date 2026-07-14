
use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::types::CreateWorkspaceRequest;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::project_memory::state::ProjectMemoryState;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, DbState>,
    registry: State<'_, WorkspaceRegistry>,
    req: CreateWorkspaceRequest,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
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

    if path.contains("..") {
        return Err(AppError::path_traversal());
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

    // 创建后自动授权工作空间根路径，使 fs_* 命令立即可用
    registry.authorize(&path_buf)
        .map_err(|e| AppError::internal(format!("Failed to authorize workspace: {}", e)))?;

    let workspace = state.db.create_workspace(req)?;
    tracing::info!(workspace_id = %workspace.id, "Workspace created and authorized");
    Ok(IpcResponse::created(workspace))
}

#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Workspace>>, AppError> {
    tracing::debug!("list_workspaces");
    let workspaces = state.db.list_workspaces()?;
    tracing::debug!(count = workspaces.len(), "Workspaces listed");
    Ok(IpcResponse::ok(workspaces))
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
    validate_id_component(&id, "workspace_id")?;
    tracing::debug!(workspace_id = %id, "get_workspace");
    let workspace = state.db.get_workspace(&id)?
        .ok_or_else(|| {
            tracing::warn!(workspace_id = %id, "Workspace not found");
            AppError::workspace_not_found()
        })?;
    Ok(IpcResponse::ok(workspace))
}

#[tauri::command]
pub async fn delete_workspace(
    state: State<'_, DbState>,
    pm_state: State<'_, ProjectMemoryState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "workspace_id")?;
    tracing::info!(workspace_id = %id, "delete_workspace");
    let deleted = state.db.delete_workspace(&id)?;

    // 级联清理 workspace 级 project_memory 文件(失败不阻塞 workspace 删除,
    // 仅记录警告 —— DB 已删除,文件残留可在后续 GC 中清理)
    if deleted {
        if let Err(e) = pm_state.store.delete(&id) {
            tracing::warn!(
                workspace_id = %id,
                error = %e,
                "Failed to cleanup project_memory dir during workspace deletion"
            );
        }
    }

    tracing::info!(workspace_id = %id, deleted, "Workspace deleted (cascade sessions + project_memory)");
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn touch_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&id, "workspace_id")?;
    state.db.touch_workspace(&id)?;
    Ok(IpcResponse::ok(()))
}