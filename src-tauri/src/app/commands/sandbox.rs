use crate::errors::{IpcResponse, AppError};
use crate::infra::sandbox::policy::SandboxPolicy;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn sandbox_status(
    state: State<'_, AppState>,
) -> Result<IpcResponse<crate::infra::sandbox::enforce::SandboxStatus>, AppError> {
    let enforcer = state.sandbox.lock().await;
    let status = enforcer.status();
    Ok(IpcResponse::ok(status))
}

#[tauri::command]
pub async fn sandbox_validate_file(
    state: State<'_, AppState>,
    path: String,
    is_write: bool,
) -> Result<IpcResponse<bool>, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > 4096 {
        return Err(AppError::invalid_input("Path too long (max 4096 chars)"));
    }
    let enforcer = state.sandbox.lock().await;
    let path_buf = std::path::PathBuf::from(&path);
    match enforcer.validate_file_operation(&path_buf, is_write) {
        Ok(()) => Ok(IpcResponse::ok(true)),
        Err(v) => {
            tracing::warn!(path, is_write, violation = %v, "Sandbox file violation");
            Ok(IpcResponse::ok(false))
        }
    }
}

#[tauri::command]
pub async fn sandbox_validate_command(
    state: State<'_, AppState>,
    command: String,
) -> Result<IpcResponse<bool>, AppError> {
    if command.trim().is_empty() {
        return Err(AppError::invalid_input("Command cannot be empty"));
    }
    if command.len() > 10_000 {
        return Err(AppError::invalid_input("Command too long (max 10000 chars)"));
    }
    let enforcer = state.sandbox.lock().await;
    match enforcer.validate_command(&command) {
        Ok(()) => Ok(IpcResponse::ok(true)),
        Err(v) => {
            tracing::warn!(command, violation = %v, "Sandbox command violation");
            Ok(IpcResponse::ok(false))
        }
    }
}

#[tauri::command]
pub async fn sandbox_validate_network(
    state: State<'_, AppState>,
    url: String,
) -> Result<IpcResponse<bool>, AppError> {
    if url.trim().is_empty() {
        return Err(AppError::invalid_input("URL cannot be empty"));
    }
    if url.len() > 2048 {
        return Err(AppError::invalid_input("URL too long (max 2048 chars)"));
    }
    let enforcer = state.sandbox.lock().await;
    match enforcer.validate_network(&url) {
        Ok(()) => Ok(IpcResponse::ok(true)),
        Err(v) => {
            tracing::warn!(url, violation = %v, "Sandbox network violation");
            Ok(IpcResponse::ok(false))
        }
    }
}

#[tauri::command]
pub async fn sandbox_get_policy(
    state: State<'_, AppState>,
) -> Result<IpcResponse<SandboxPolicy>, AppError> {
    let enforcer = state.sandbox.lock().await;
    Ok(IpcResponse::ok(enforcer.policy().clone()))
}
