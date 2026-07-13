use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::sandbox::policy::SandboxPolicy;
use crate::infrastructure::sandbox::state::SandboxState;
use crate::infrastructure::validation::validate_path;
use tauri::State;
use std::path::PathBuf;

const MAX_PATH_LEN: usize = 4096;
const MAX_COMMAND_LEN: usize = 10_000;
const MAX_URL_LEN: usize = 2048;

fn validate_sandbox_path(path: &str) -> Result<PathBuf, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > MAX_PATH_LEN {
        return Err(AppError::invalid_input(format!(
            "Path too long (max {} chars)", MAX_PATH_LEN
        )));
    }
    validate_path(path).map_err(|e| AppError::invalid_input(e))?;

    let path_buf = PathBuf::from(path);
    if path_buf.components().any(|c| c.as_os_str() == "..") {
        return Err(AppError::invalid_input("Path traversal denied"));
    }
    Ok(path_buf)
}

#[tauri::command]
pub async fn sandbox_status(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<crate::infrastructure::sandbox::types::SandboxStatus>, AppError> {
    let status = state.get_status();
    Ok(IpcResponse::ok(status))
}

#[tauri::command]
pub async fn sandbox_validate_path(
    state: State<'_, SandboxState>,
    path: String,
    is_write: bool,
) -> Result<IpcResponse<bool>, AppError> {
    let path_buf = validate_sandbox_path(&path)?;
    let allowed = state.validate_path(&path_buf, is_write)?;
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_validate_command(
    state: State<'_, SandboxState>,
    command: String,
) -> Result<IpcResponse<bool>, AppError> {
    if command.trim().is_empty() {
        return Err(AppError::invalid_input("Command cannot be empty"));
    }
    if command.len() > MAX_COMMAND_LEN {
        return Err(AppError::invalid_input(format!(
            "Command too long (max {} chars)", MAX_COMMAND_LEN
        )));
    }
    let allowed = state.validate_command(&command)?;
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_validate_url(
    state: State<'_, SandboxState>,
    url: String,
) -> Result<IpcResponse<bool>, AppError> {
    if url.trim().is_empty() {
        return Err(AppError::invalid_input("URL cannot be empty"));
    }
    if url.len() > MAX_URL_LEN {
        return Err(AppError::invalid_input(format!(
            "URL too long (max {} chars)", MAX_URL_LEN
        )));
    }
    let allowed = state.validate_url(&url)?;
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_get_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<SandboxPolicy>, AppError> {
    let policy = state.get_policy();
    Ok(IpcResponse::ok(policy))
}