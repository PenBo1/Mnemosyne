
//! ═══════════════════════════════════════════════════════════════════════════
//! Commands - 工作区 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供工作区管理的 IPC 命令实现：
//! - create_workspace：创建工作区
//! - list_workspaces：列出工作区
//! - list_archived_workspaces：列出已归档工作区
//! - get_workspace：获取工作区
//! - delete_workspace：删除工作区
//! - touch_workspace：更新工作区访问时间
//! - archive_workspace：归档工作区
//! - restore_workspace：恢复工作区

use tauri::State;
use crate::shared::error::{IpcResponse, AppError};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::types::CreateWorkspaceRequest;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::project_memory::state::ProjectMemoryState;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use std::time::Instant;

#[tauri::command]
pub async fn create_workspace(
    state: State<'_, DbState>,
    registry: State<'_, WorkspaceRegistry>,
    req: CreateWorkspaceRequest,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
    let start = Instant::now();
    tracing::info!(
        name = %req.name,
        path = ?req.path,
        "create_workspace: enter"
    );
    
    if req.name.trim().is_empty() {
        tracing::error!("create_workspace: Workspace name cannot be empty");
        return Err(AppError::invalid_input("Workspace name cannot be empty"));
    }
    if req.name.len() > 255 {
        tracing::error!(len = req.name.len(), "create_workspace: Workspace name too long");
        return Err(AppError::invalid_input("Workspace name too long (max 255 chars)"));
    }

    let path = req.path.clone().unwrap_or_default();
    if path.is_empty() {
        tracing::error!("create_workspace: Path is missing");
        return Err(AppError::missing_field("path"));
    }

    // 用 components().any 检测 ParentDir 组件，比 path.contains("..") 更精确
    // （后者会误拒 `my..file.txt` 这类合法路径名）
    let path_buf = std::path::PathBuf::from(&path);
    use std::path::Component;
    if path_buf.components().any(|c| matches!(c, Component::ParentDir)) {
        tracing::error!(path = %path, "create_workspace: Path traversal detected");
        return Err(AppError::path_traversal());
    }
    // 多个 create_dir_all 卸载到阻塞线程池一次性完成
    let path_buf_for_io = path_buf.clone();
    let path_for_err = path.clone();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        std::fs::create_dir_all(&path_buf_for_io)
            .map_err(|e| {
                tracing::error!(error = %e, path = %path_for_err, "Failed to create workspace directory");
                AppError::file_write_error(path_for_err.clone())
            })?;
        for sub in ["chapters", "story/state", "story/snapshots", "story/drafts"] {
            std::fs::create_dir_all(path_buf_for_io.join(sub))
                .map_err(|e| {
                    tracing::error!(error = %e, sub = %sub, "Failed to create workspace subdirectory");
                    AppError::file_write_error(format!("{}/{}", path_for_err, sub))
                })?;
        }
        Ok(())
    })
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "create_workspace: spawn_blocking join failed");
        AppError::internal(format!("spawn_blocking join failed: {}", e))
    })??;

    // 创建后自动授权工作空间根路径，使 fs_* 命令立即可用
    registry.authorize(&path_buf)
        .map_err(|e| {
            tracing::error!(error = %e, "create_workspace: Failed to authorize workspace");
            AppError::internal(format!("Failed to authorize workspace: {}", e))
        })?;

    let workspace = state.db.create_workspace(req).map_err(|e| {
        tracing::error!(error = %e, "create_workspace: Failed to create workspace in DB");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace.id,
        duration_ms = start.elapsed().as_millis(),
        "create_workspace: exit"
    );
    Ok(IpcResponse::created(workspace))
}

#[tauri::command]
pub async fn list_workspaces(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Workspace>>, AppError> {
    let start = Instant::now();
    tracing::info!("list_workspaces: enter");
    
    let workspaces = state.db.list_workspaces().map_err(|e| {
        tracing::error!(error = %e, "list_workspaces: Failed to list workspaces");
        e
    })?;
    
    tracing::info!(
        count = workspaces.len(),
        duration_ms = start.elapsed().as_millis(),
        "list_workspaces: exit"
    );
    Ok(IpcResponse::ok(workspaces))
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %id, "get_workspace: enter");
    
    validate_id_component(&id, "workspace_id")?;
    
    let workspace = state.db.get_workspace(&id).map_err(|e| {
        tracing::error!(workspace_id = %id, error = %e, "get_workspace: Failed to get workspace");
        e
    })?.ok_or_else(|| {
        tracing::error!(workspace_id = %id, "get_workspace: Workspace not found");
        AppError::workspace_not_found()
    })?;
    
    tracing::info!(
        workspace_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "get_workspace: exit"
    );
    Ok(IpcResponse::ok(workspace))
}

#[tauri::command]
pub async fn delete_workspace(
    state: State<'_, DbState>,
    pm_state: State<'_, ProjectMemoryState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %id, "delete_workspace: enter");
    
    validate_id_component(&id, "workspace_id")?;
    
    let deleted = state.db.delete_workspace(&id).map_err(|e| {
        tracing::error!(workspace_id = %id, error = %e, "delete_workspace: Failed to delete workspace");
        e
    })?;

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

    tracing::info!(
        workspace_id = %id,
        deleted,
        duration_ms = start.elapsed().as_millis(),
        "delete_workspace: exit"
    );
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn touch_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %id, "touch_workspace: enter");
    
    validate_id_component(&id, "workspace_id")?;
    
    state.db.touch_workspace(&id).map_err(|e| {
        tracing::error!(workspace_id = %id, error = %e, "touch_workspace: Failed to touch workspace");
        e
    })?;
    
    tracing::info!(
        workspace_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "touch_workspace: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn list_archived_workspaces(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Workspace>>, AppError> {
    let start = Instant::now();
    tracing::info!("list_archived_workspaces: enter");
    
    let workspaces = state.db.list_archived_workspaces().map_err(|e| {
        tracing::error!(error = %e, "list_archived_workspaces: Failed to list archived workspaces");
        e
    })?;
    
    tracing::info!(
        count = workspaces.len(),
        duration_ms = start.elapsed().as_millis(),
        "list_archived_workspaces: exit"
    );
    Ok(IpcResponse::ok(workspaces))
}

#[tauri::command]
pub async fn archive_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %id, "archive_workspace: enter");
    
    validate_id_component(&id, "workspace_id")?;
    
    let workspace = state.db.archive_workspace(&id).map_err(|e| {
        tracing::error!(workspace_id = %id, error = %e, "archive_workspace: Failed to archive workspace");
        e
    })?;
    
    tracing::info!(
        workspace_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "archive_workspace: exit"
    );
    Ok(IpcResponse::ok(workspace))
}

#[tauri::command]
pub async fn restore_workspace(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Workspace>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %id, "restore_workspace: enter");
    
    validate_id_component(&id, "workspace_id")?;
    
    let workspace = state.db.restore_workspace(&id).map_err(|e| {
        tracing::error!(workspace_id = %id, error = %e, "restore_workspace: Failed to restore workspace");
        e
    })?;
    
    tracing::info!(
        workspace_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "restore_workspace: exit"
    );
    Ok(IpcResponse::ok(workspace))
}

/// 更新工作区排序
#[tauri::command]
pub async fn update_workspace_sort_order(
    state: State<'_, DbState>,
    ids: Vec<String>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(count = ids.len(), "update_workspace_sort_order: enter");
    
    for id in &ids {
        validate_id_component(id, "workspace_id")?;
    }
    
    state.db.update_workspace_sort_order(&ids).map_err(|e| {
        tracing::error!(error = %e, "update_workspace_sort_order: Failed to update sort order");
        e
    })?;
    
    tracing::info!(
        count = ids.len(),
        duration_ms = start.elapsed().as_millis(),
        "update_workspace_sort_order: exit"
    );
    Ok(IpcResponse::ok(()))
}