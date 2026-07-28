//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::sandbox::execpolicy::{
    evaluator::Evaluation, ExecPolicy, NetworkProtocol,
};
use crate::infrastructure::sandbox::policy::SandboxPolicy;
use crate::infrastructure::sandbox::state::SandboxState;
use crate::infrastructure::sandbox::types::SandboxStatus;
use crate::infrastructure::validation::validate_path;
use tauri::State;
use std::path::PathBuf;
use std::time::Instant;

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
    validate_path(path).map_err(AppError::invalid_input)?;

    let path_buf = PathBuf::from(path);
    if path_buf.components().any(|c| c.as_os_str() == "..") {
        return Err(AppError::invalid_input("Path traversal denied"));
    }
    Ok(path_buf)
}

#[tauri::command]
pub async fn sandbox_status(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<SandboxStatus>, AppError> {
    let start = Instant::now();
    tracing::info!("sandbox_status: enter");
    
    let status = state.get_status();
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "sandbox_status: exit"
    );
    Ok(IpcResponse::ok(status))
}

#[tauri::command]
pub async fn sandbox_validate_path(
    state: State<'_, SandboxState>,
    path: String,
    is_write: bool,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        path = %path,
        is_write,
        "sandbox_validate_path: enter"
    );
    
    let path_buf = validate_sandbox_path(&path)?;
    let allowed = state.validate_path(&path_buf, is_write).map_err(|e| {
        tracing::error!(path = %path, error = %e, "sandbox_validate_path: Failed to validate path");
        e
    })?;
    
    tracing::info!(
        path = %path,
        allowed,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_validate_path: exit"
    );
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_validate_command(
    state: State<'_, SandboxState>,
    command: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(command = %command, "sandbox_validate_command: enter");
    
    if command.trim().is_empty() {
        tracing::error!("sandbox_validate_command: Command cannot be empty");
        return Err(AppError::invalid_input("Command cannot be empty"));
    }
    if command.len() > MAX_COMMAND_LEN {
        tracing::error!(len = command.len(), "sandbox_validate_command: Command too long");
        return Err(AppError::invalid_input(format!(
            "Command too long (max {} chars)", MAX_COMMAND_LEN
        )));
    }
    
    let allowed = state.validate_command(&command).map_err(|e| {
        tracing::error!(command = %command, error = %e, "sandbox_validate_command: Failed to validate command");
        e
    })?;
    
    tracing::info!(
        command = %command,
        allowed,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_validate_command: exit"
    );
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_validate_url(
    state: State<'_, SandboxState>,
    url: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(url = %url, "sandbox_validate_url: enter");
    
    if url.trim().is_empty() {
        tracing::error!("sandbox_validate_url: URL cannot be empty");
        return Err(AppError::invalid_input("URL cannot be empty"));
    }
    if url.len() > MAX_URL_LEN {
        tracing::error!(len = url.len(), "sandbox_validate_url: URL too long");
        return Err(AppError::invalid_input(format!(
            "URL too long (max {} chars)", MAX_URL_LEN
        )));
    }
    
    let allowed = state.validate_url(&url).map_err(|e| {
        tracing::error!(url = %url, error = %e, "sandbox_validate_url: Failed to validate URL");
        e
    })?;
    
    tracing::info!(
        url = %url,
        allowed,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_validate_url: exit"
    );
    Ok(IpcResponse::ok(allowed))
}

#[tauri::command]
pub async fn sandbox_get_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<SandboxPolicy>, AppError> {
    let start = Instant::now();
    tracing::info!("sandbox_get_policy: enter");
    
    let policy = state.get_policy();
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "sandbox_get_policy: exit"
    );
    Ok(IpcResponse::ok(policy))
}

// ── ExecPolicy 命令 ──

/// 获取当前 ExecPolicy。
#[tauri::command]
pub async fn sandbox_get_exec_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    let start = Instant::now();
    tracing::info!("sandbox_get_exec_policy: enter");
    
    let policy = state.get_exec_policy();
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "sandbox_get_exec_policy: exit"
    );
    Ok(IpcResponse::ok(policy))
}

/// 更新 ExecPolicy（结构化更新，自动持久化到 exec_policy.conf）。
#[tauri::command]
pub async fn sandbox_update_exec_policy(
    policy: ExecPolicy,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    let start = Instant::now();
    tracing::info!("sandbox_update_exec_policy: enter");
    
    state.update_exec_policy(policy.clone());
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "sandbox_update_exec_policy: exit"
    );
    Ok(IpcResponse::updated(policy))
}

/// 重置 ExecPolicy 为内置默认值。
#[tauri::command]
pub async fn sandbox_reset_exec_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    let start = Instant::now();
    tracing::info!("sandbox_reset_exec_policy: enter");
    
    state.reset_exec_policy();
    let policy = state.get_exec_policy();
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "sandbox_reset_exec_policy: exit"
    );
    Ok(IpcResponse::updated(policy))
}

/// 评估命令（返回完整 PolicyDecision，含 AskUser）。
#[tauri::command]
pub async fn sandbox_evaluate_command(
    command: String,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    let start = Instant::now();
    tracing::info!(command = %command, "sandbox_evaluate_command: enter");
    
    if command.trim().is_empty() {
        tracing::error!("sandbox_evaluate_command: Command cannot be empty");
        return Err(AppError::invalid_input("Command cannot be empty"));
    }
    if command.len() > MAX_COMMAND_LEN {
        tracing::error!(len = command.len(), "sandbox_evaluate_command: Command too long");
        return Err(AppError::invalid_input(format!(
            "Command too long (max {} chars)", MAX_COMMAND_LEN
        )));
    }
    
    let eval = state.evaluate_command(&command);
    
    tracing::info!(
        command = %command,
        decision = ?eval.decision,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_evaluate_command: exit"
    );
    Ok(IpcResponse::ok(eval))
}

/// 评估路径（返回完整 PolicyDecision）。
#[tauri::command]
pub async fn sandbox_evaluate_path(
    path: String,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    let start = Instant::now();
    tracing::info!(path = %path, "sandbox_evaluate_path: enter");
    
    if path.trim().is_empty() {
        tracing::error!("sandbox_evaluate_path: Path cannot be empty");
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > MAX_PATH_LEN {
        tracing::error!(len = path.len(), "sandbox_evaluate_path: Path too long");
        return Err(AppError::invalid_input(format!(
            "Path too long (max {} chars)", MAX_PATH_LEN
        )));
    }
    
    let eval = state.evaluate_path(&path);
    
    tracing::info!(
        path = %path,
        decision = ?eval.decision,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_evaluate_path: exit"
    );
    Ok(IpcResponse::ok(eval))
}

/// 评估网络请求（返回完整 PolicyDecision）。
#[tauri::command]
pub async fn sandbox_evaluate_network(
    host: String,
    protocol: NetworkProtocol,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    let start = Instant::now();
    tracing::info!(
        host = %host,
        protocol = ?protocol,
        "sandbox_evaluate_network: enter"
    );
    
    if host.trim().is_empty() {
        tracing::error!("sandbox_evaluate_network: Host cannot be empty");
        return Err(AppError::invalid_input("Host cannot be empty"));
    }
    if host.len() > MAX_URL_LEN {
        tracing::error!(len = host.len(), "sandbox_evaluate_network: Host too long");
        return Err(AppError::invalid_input(format!(
            "Host too long (max {} chars)", MAX_URL_LEN
        )));
    }
    
    let eval = state.evaluate_network(&host, protocol);
    
    tracing::info!(
        host = %host,
        decision = ?eval.decision,
        duration_ms = start.elapsed().as_millis(),
        "sandbox_evaluate_network: exit"
    );
    Ok(IpcResponse::ok(eval))
}