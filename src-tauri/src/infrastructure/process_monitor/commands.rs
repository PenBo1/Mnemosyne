//! ═══════════════════════════════════════════════════════════════════════════
//! 进程监控命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use super::monitor::{get_app_processes, get_system_resource_summary, ProcessInfo, SystemResourceSummary};

/// Get list of app-related processes.
///
/// Returns process name, PID, CPU usage, memory usage, and type classification.
#[tauri::command]
pub async fn process_monitor_list() -> Result<IpcResponse<Vec<ProcessInfo>>, AppError> {
    let processes = get_app_processes("mnemosyne")?;
    Ok(IpcResponse::ok(processes))
}

/// Get system resource summary for app processes.
///
/// Returns total CPU, memory usage, and process count.
#[tauri::command]
pub async fn process_monitor_summary() -> Result<IpcResponse<SystemResourceSummary>, AppError> {
    let summary = get_system_resource_summary("mnemosyne")?;
    Ok(IpcResponse::ok(summary))
}