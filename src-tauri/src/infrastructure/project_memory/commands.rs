//! ═══════════════════════════════════════════════════════════════════════════
//! 项目记忆命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use serde::Serialize;
use tauri::State;
use std::time::Instant;

use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::infrastructure::project_memory::state::ProjectMemoryState;
use crate::shared::error::{AppError, IpcResponse};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemoryStats {
    pub bytes: usize,
    pub chars: usize,
    pub max_bytes: usize,
    pub exists: bool,
}

#[tauri::command]
pub async fn project_memory_get(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<String>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %workspace_id, "project_memory_get: enter");
    
    validate_id_component(&workspace_id, "workspace_id")?;
    
    let content = state.store.read(&workspace_id).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_get: Failed to read project memory");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace_id,
        length = content.len(),
        duration_ms = start.elapsed().as_millis(),
        "project_memory_get: exit"
    );
    Ok(IpcResponse::ok(content))
}

#[tauri::command]
pub async fn project_memory_update(
    workspace_id: String,
    content: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(
        workspace_id = %workspace_id,
        content_len = content.len(),
        "project_memory_update: enter"
    );
    
    validate_id_component(&workspace_id, "workspace_id")?;
    // 内容长度上限交给 store 的 MAX_FILE_SIZE 校验,这里只做空校验
    // 允许空字符串(等价于 clear)
    
    state.store.write(&workspace_id, &content).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_update: Failed to write project memory");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = start.elapsed().as_millis(),
        "project_memory_update: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_append(
    workspace_id: String,
    section: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(
        workspace_id = %workspace_id,
        section_len = section.len(),
        "project_memory_append: enter"
    );
    
    validate_id_component(&workspace_id, "workspace_id")?;
    if section.trim().is_empty() {
        tracing::error!("project_memory_append: Section cannot be empty");
        return Err(AppError::invalid_input("Section cannot be empty"));
    }
    
    state.store.append(&workspace_id, &section).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_append: Failed to append project memory");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = start.elapsed().as_millis(),
        "project_memory_append: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_clear(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %workspace_id, "project_memory_clear: enter");
    
    validate_id_component(&workspace_id, "workspace_id")?;
    
    state.store.clear(&workspace_id).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_clear: Failed to clear project memory");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = start.elapsed().as_millis(),
        "project_memory_clear: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_delete(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %workspace_id, "project_memory_delete: enter");
    
    validate_id_component(&workspace_id, "workspace_id")?;
    
    state.store.delete(&workspace_id).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_delete: Failed to delete project memory");
        e
    })?;
    
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = start.elapsed().as_millis(),
        "project_memory_delete: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_stats(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<ProjectMemoryStats>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %workspace_id, "project_memory_stats: enter");
    
    validate_id_component(&workspace_id, "workspace_id")?;
    // exists 用 path.exists() 而非 !content.is_empty(),否则空文件会被误判为不存在
    let exists = state.store.exists(&workspace_id);
    let content = state.store.read(&workspace_id).map_err(|e| {
        tracing::error!(workspace_id = %workspace_id, error = %e, "project_memory_stats: Failed to read project memory");
        e
    })?;
    let max = state.store.max_size();
    
    tracing::info!(
        workspace_id = %workspace_id,
        bytes = content.len(),
        chars = content.chars().count(),
        exists,
        duration_ms = start.elapsed().as_millis(),
        "project_memory_stats: exit"
    );
    
    Ok(IpcResponse::ok(ProjectMemoryStats {
        bytes: content.len(),
        chars: content.chars().count(),
        max_bytes: max,
        exists,
    }))
}