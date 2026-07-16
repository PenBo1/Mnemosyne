// Project Memory IPC commands —— workspace 级别项目记忆文件读写。
//
// 命令清单:
// - project_memory_get(workspace_id) → 当前内容(不存在返回空串)
// - project_memory_update(workspace_id, content) → 全量覆盖
// - project_memory_append(workspace_id, section) → 追加段落
// - project_memory_clear(workspace_id) → 清空内容(保留文件)
// - project_memory_delete(workspace_id) → 删除文件 + 目录(由 delete_workspace 调用)
// - project_memory_stats(workspace_id) → 字节数 / 大小上限 / 字符数

use serde::Serialize;
use tauri::State;

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
    validate_id_component(&workspace_id, "workspace_id")?;
    let content = state.store.read(&workspace_id)?;
    Ok(IpcResponse::ok(content))
}

#[tauri::command]
pub async fn project_memory_update(
    workspace_id: String,
    content: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    // 内容长度上限交给 store 的 MAX_FILE_SIZE 校验,这里只做空校验
    // 允许空字符串(等价于 clear)
    state.store.write(&workspace_id, &content)?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_append(
    workspace_id: String,
    section: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    if section.trim().is_empty() {
        return Err(AppError::invalid_input("Section cannot be empty"));
    }
    state.store.append(&workspace_id, &section)?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_clear(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    state.store.clear(&workspace_id)?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_delete(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    state.store.delete(&workspace_id)?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn project_memory_stats(
    workspace_id: String,
    state: State<'_, ProjectMemoryState>,
) -> Result<IpcResponse<ProjectMemoryStats>, AppError> {
    validate_id_component(&workspace_id, "workspace_id")?;
    // exists 用 path.exists() 而非 !content.is_empty(),否则空文件会被误判为不存在
    let exists = state.store.exists(&workspace_id);
    let content = state.store.read(&workspace_id)?;
    let max = state.store.max_size();
    Ok(IpcResponse::ok(ProjectMemoryStats {
        bytes: content.len(),
        chars: content.chars().count(),
        max_bytes: max,
        exists,
    }))
}
