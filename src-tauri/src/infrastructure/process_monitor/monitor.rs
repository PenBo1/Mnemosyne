//! ═══════════════════════════════════════════════════════════════════════════
//! 进程监控实现 - sysinfo 实现
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::OnceLock;
use sysinfo::{Process, System};
use crate::shared::error::AppError;

static SYSTEM: OnceLock<std::sync::Mutex<System>> = OnceLock::new();

fn get_system() -> std::sync::MutexGuard<'static, System> {
    let sys = SYSTEM.get_or_init(|| {
        let mut s = System::new_all();
        s.refresh_all();
        std::sync::Mutex::new(s)
    });
    sys.lock().unwrap_or_else(|e| e.into_inner())
}

/// Process type classification based on command line arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessType {
    Main,
    Renderer,
    Gpu,
    Utility,
    Unknown,
}

impl ProcessType {
    /// Classify process type from command line arguments.
    /// Tauri/Electron processes use --type= argument to identify process type.
    pub fn from_cmd(cmd: &[String]) -> Self {
        for arg in cmd {
            let arg_lower = arg.to_lowercase();
            
            // Check for --type= argument
            if arg_lower.starts_with("--type=") {
                let type_name = arg_lower.split('=').nth(1).unwrap_or("");
                return match type_name {
                    "renderer" => Self::Renderer,
                    "gpu-process" | "gpu" => Self::Gpu,
                    "utility" => Self::Utility,
                    _ => Self::Unknown,
                };
            }
            
            // Also check for renderer-specific arguments
            if arg_lower.contains("renderer") {
                return Self::Renderer;
            }
            if arg_lower.contains("gpu") {
                return Self::Gpu;
            }
        }
        
        // If no --type argument, it's likely the main process
        Self::Main
    }
}

/// Process information for frontend display.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    /// Process name with type suffix (e.g., "mnemosyne", "mnemosyne (renderer)")
    pub name: String,
    /// Process ID
    pub pid: u32,
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_mb: f64,
    /// Process type classification
    pub process_type: ProcessType,
}

impl ProcessInfo {
    /// Create ProcessInfo from sysinfo Process.
    fn from_process(process: &Process) -> Self {
        let cmd: Vec<String> = process.cmd().iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        
        let process_type = ProcessType::from_cmd(&cmd);
        
        // Build display name based on process type
        let base_name = process.name().to_string_lossy().to_string();
        let name = match process_type {
            ProcessType::Main => base_name,
            ProcessType::Renderer => {
                // Try to identify which renderer from URL
                let url_arg = cmd.iter().find(|a| a.starts_with("http://") || a.starts_with("https://"));
                if let Some(url) = url_arg {
                    if url.contains("process-monitor") {
                        format!("{} (Process Monitor)", base_name)
                    } else {
                        format!("{} (Renderer)", base_name)
                    }
                } else {
                    format!("{} (Renderer)", base_name)
                }
            }
            ProcessType::Gpu => format!("{} (GPU)", base_name),
            ProcessType::Utility => {
                // Try to identify utility type
                let utility_name = cmd.iter()
                    .find(|a| a.contains("network") || a.contains("audio") || a.contains("cron"))
                    .map(|s| {
                        if s.contains("network") { "Network" }
                        else if s.contains("audio") { "Audio" }
                        else if s.contains("cron") { "Cron" }
                        else { "Utility" }
                    })
                    .unwrap_or("Utility");
                format!("{} ({})", base_name, utility_name)
            }
            ProcessType::Unknown => format!("{} (Unknown)", base_name),
        };
        
        // sysinfo memory() returns bytes
        let memory_bytes = process.memory();
        Self {
            name,
            pid: process.pid().as_u32(),
            cpu_usage: process.cpu_usage() as f64,
            memory_mb: memory_bytes as f64 / 1024.0 / 1024.0,
            process_type,
        }
    }
}

/// Get all processes related to the application.
///
/// Filters processes by name containing app name (case-insensitive).
/// Uses command line arguments to identify process type.
pub fn get_app_processes(app_name: &str) -> Result<Vec<ProcessInfo>, AppError> {
    let mut sys = get_system();

    // Refresh process information including CPU and memory
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::everything(),
    );

    let app_name_lower = app_name.to_lowercase();
    
    // Filter processes containing app name
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .filter(|(_, process)| {
            let name = process.name().to_string_lossy().to_lowercase();
            name.contains(&app_name_lower)
        })
        .map(|(_, process)| ProcessInfo::from_process(process))
        .collect();

    // Sort by type, then by PID
    processes.sort_by(|a, b| {
        let type_order = |t: ProcessType| match t {
            ProcessType::Main => 0,
            ProcessType::Renderer => 1,
            ProcessType::Gpu => 2,
            ProcessType::Utility => 3,
            ProcessType::Unknown => 4,
        };
        match type_order(a.process_type).cmp(&type_order(b.process_type)) {
            std::cmp::Ordering::Equal => a.pid.cmp(&b.pid),
            other => other,
        }
    });

    Ok(processes)
}

/// Get system-wide resource usage summary.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResourceSummary {
    /// Total CPU usage percentage (0.0 - 100.0)
    pub total_cpu_usage: f64,
    /// Total memory used in MB
    pub total_memory_mb: f64,
    /// Total memory available in MB
    pub total_memory_available_mb: f64,
    /// Number of app-related processes
    pub process_count: usize,
}

/// Get system resource summary for app processes.
pub fn get_system_resource_summary(app_name: &str) -> Result<SystemResourceSummary, AppError> {
    let processes = get_app_processes(app_name)?;

    let mut sys = get_system();
    sys.refresh_memory();

    let total_cpu_usage: f64 = processes.iter().map(|p| p.cpu_usage).sum();
    let total_memory_mb: f64 = processes.iter().map(|p| p.memory_mb).sum();
    let total_memory_available_mb = sys.available_memory() as f64 / 1024.0 / 1024.0;

    Ok(SystemResourceSummary {
        total_cpu_usage,
        total_memory_mb,
        total_memory_available_mb,
        process_count: processes.len(),
    })
}