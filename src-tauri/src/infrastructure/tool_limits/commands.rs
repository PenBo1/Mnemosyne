// Tool Limits IPC commands —— 工具执行上限配置的读写。
//
// 命令清单:
// - tool_limits_get → 当前配置(从内存缓存读)
// - tool_limits_update(config) → 更新配置(写盘 + 刷新缓存)
// - tool_limits_reset → 重置为默认值

use tauri::State;

use crate::infrastructure::tool_limits::config::ToolLimitsConfig;
use crate::infrastructure::tool_limits::state::ToolLimitsState;
use crate::shared::error::{AppError, IpcResponse};

#[tauri::command]
pub async fn tool_limits_get(
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<ToolLimitsConfig>, AppError> {
    Ok(IpcResponse::ok(state.get()))
}

#[tauri::command]
pub async fn tool_limits_update(
    config: ToolLimitsConfig,
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<()>, AppError> {
    state.update(config)?;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn tool_limits_reset(
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<()>, AppError> {
    state.reset()?;
    Ok(IpcResponse::ok(()))
}
