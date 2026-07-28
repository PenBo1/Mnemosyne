//! ═══════════════════════════════════════════════════════════════════════════
//! 工具限制命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::time::Instant;

use crate::infrastructure::tool_limits::config::ToolLimitsConfig;
use crate::infrastructure::tool_limits::state::ToolLimitsState;
use crate::shared::error::{AppError, IpcResponse};

#[tauri::command]
pub async fn tool_limits_get(
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<ToolLimitsConfig>, AppError> {
    let start = Instant::now();
    tracing::info!("tool_limits_get: enter");
    
    let config = state.get();
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "tool_limits_get: exit"
    );
    Ok(IpcResponse::ok(config))
}

#[tauri::command]
pub async fn tool_limits_update(
    config: ToolLimitsConfig,
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!("tool_limits_update: enter");
    
    state.update(config).map_err(|e| {
        tracing::error!(error = %e, "tool_limits_update: Failed to update tool limits");
        e
    })?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "tool_limits_update: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn tool_limits_reset(
    state: State<'_, ToolLimitsState>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!("tool_limits_reset: enter");
    
    state.reset().map_err(|e| {
        tracing::error!(error = %e, "tool_limits_reset: Failed to reset tool limits");
        e
    })?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "tool_limits_reset: exit"
    );
    Ok(IpcResponse::ok(()))
}