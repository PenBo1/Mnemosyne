//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 仅包含参数验证和委托，业务逻辑在 validator 模块中。

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::sandbox::execpolicy::{
    evaluator::Evaluation, ExecPolicy, NetworkProtocol,
};
use crate::infrastructure::sandbox::policy::SandboxPolicy;
use crate::infrastructure::sandbox::state::SandboxState;
use crate::infrastructure::sandbox::types::SandboxStatus;
use crate::infrastructure::sandbox::validator::{
    validate_sandbox_path, validate_command_param, validate_path_param,
    validate_url_param, validate_host_param,
};
use tauri::State;
use std::time::Instant;

// ── 状态命令 ────────────────────────────────────────────────────────────────

/// 获取沙箱状态
#[tauri::command]
pub async fn sandbox_status(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<SandboxStatus>, AppError> {
    let status = state.get_status();
    Ok(IpcResponse::ok(status))
}

/// 获取沙箱策略
#[tauri::command]
pub async fn sandbox_get_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<SandboxPolicy>, AppError> {
    let policy = state.get_policy();
    Ok(IpcResponse::ok(policy))
}

// ── 验证命令 ────────────────────────────────────────────────────────────────

/// 验证路径
#[tauri::command]
pub async fn sandbox_validate_path(
    state: State<'_, SandboxState>,
    path: String,
    is_write: bool,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(path = %path, is_write, "sandbox_validate_path: enter");

    // 参数验证
    let path_buf = validate_sandbox_path(&path)?;

    // 委托给状态层
    let allowed = state.validate_path(&path_buf, is_write).map_err(|e| {
        tracing::error!(path = %path, error = %e, "sandbox_validate_path: Failed");
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

/// 验证命令
#[tauri::command]
pub async fn sandbox_validate_command(
    state: State<'_, SandboxState>,
    command: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(command = %command, "sandbox_validate_command: enter");

    // 参数验证
    validate_command_param(&command)?;

    // 委托给状态层
    let allowed = state.validate_command(&command).map_err(|e| {
        tracing::error!(command = %command, error = %e, "sandbox_validate_command: Failed");
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

/// 验证 URL
#[tauri::command]
pub async fn sandbox_validate_url(
    state: State<'_, SandboxState>,
    url: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(url = %url, "sandbox_validate_url: enter");

    // 参数验证
    validate_url_param(&url)?;

    // 委托给状态层
    let allowed = state.validate_url(&url).map_err(|e| {
        tracing::error!(url = %url, error = %e, "sandbox_validate_url: Failed");
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

// ── ExecPolicy 命令 ──────────────────────────────────────────────────────────

/// 获取执行策略
#[tauri::command]
pub async fn sandbox_get_exec_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    let policy = state.get_exec_policy();
    Ok(IpcResponse::ok(policy))
}

/// 更新执行策略
#[tauri::command]
pub async fn sandbox_update_exec_policy(
    policy: ExecPolicy,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    state.update_exec_policy(policy.clone());
    Ok(IpcResponse::updated(policy))
}

/// 重置执行策略
#[tauri::command]
pub async fn sandbox_reset_exec_policy(
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<ExecPolicy>, AppError> {
    state.reset_exec_policy();
    let policy = state.get_exec_policy();
    Ok(IpcResponse::updated(policy))
}

// ── 评估命令 ────────────────────────────────────────────────────────────────

/// 评估命令
#[tauri::command]
pub async fn sandbox_evaluate_command(
    command: String,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    // 参数验证
    validate_command_param(&command)?;

    let eval = state.evaluate_command(&command);
    Ok(IpcResponse::ok(eval))
}

/// 评估路径
#[tauri::command]
pub async fn sandbox_evaluate_path(
    path: String,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    // 参数验证
    validate_path_param(&path)?;

    let eval = state.evaluate_path(&path);
    Ok(IpcResponse::ok(eval))
}

/// 评估网络请求
#[tauri::command]
pub async fn sandbox_evaluate_network(
    host: String,
    protocol: NetworkProtocol,
    state: State<'_, SandboxState>,
) -> Result<IpcResponse<Evaluation>, AppError> {
    // 参数验证
    validate_host_param(&host)?;

    let eval = state.evaluate_network(&host, protocol);
    Ok(IpcResponse::ok(eval))
}