use std::sync::OnceLock;
use sysinfo::{Process, System};
use super::quota::{ResourceQuota, ResourceUsage};
use crate::shared::error::AppError;

static SYSTEM: OnceLock<std::sync::Mutex<System>> = OnceLock::new();

fn get_system() -> std::sync::MutexGuard<'static, System> {
    let sys = SYSTEM.get_or_init(|| std::sync::Mutex::new(System::new_all()));
    sys.lock().expect("Failed to lock system monitor")
}

pub struct ResourceMonitor {
    pid: Option<u32>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self { pid: None }
    }

    pub fn with_pid(pid: u32) -> Self {
        Self { pid: Some(pid) }
    }

    pub fn current_process() -> Self {
        Self::with_pid(std::process::id())
    }

    pub fn get_current_usage(&self) -> Result<ResourceUsage, AppError> {
        let cpu = self.get_process_cpu_usage()?;
        let memory = self.get_process_memory_usage()?;
        let disk = self.get_disk_usage()?;
        let network = self.get_network_usage()?;

        Ok(ResourceUsage {
            cpu,
            memory,
            disk,
            network,
            token: 0,
            cost: 0.0,
        })
    }

    pub fn get_system_cpu_usage(&self) -> Result<f64, AppError> {
        let mut sys = get_system();
        sys.refresh_all();

        let cpus = sys.cpus();
        if cpus.is_empty() {
            return Ok(0.0);
        }

        let total_usage: f64 = cpus.iter().map(|c| c.cpu_usage() as f64).sum();
        let avg_usage = total_usage / cpus.len() as f64;

        Ok(avg_usage)
    }

    pub fn get_system_memory_usage(&self) -> Result<u64, AppError> {
        let mut sys = get_system();
        sys.refresh_memory();

        let used = sys.used_memory();
        Ok(used / 1024)
    }

    pub fn get_system_memory_total(&self) -> Result<u64, AppError> {
        let sys = get_system();
        let total = sys.total_memory();
        Ok(total / 1024)
    }

    pub fn get_process_memory_usage(&self) -> Result<u64, AppError> {
        let pid = match self.pid {
            Some(p) => p,
            None => return Ok(0),
        };

        let mut sys = get_system();
        sys.refresh_memory();
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            false,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );

        let pid_obj = sysinfo::Pid::from_u32(pid);
        let process: Option<&Process> = sys.process(pid_obj);

        match process {
            Some(p) => Ok(p.memory() / 1024),
            None => Ok(0),
        }
    }

    pub fn get_process_cpu_usage(&self) -> Result<f64, AppError> {
        let pid = match self.pid {
            Some(p) => p,
            None => return Ok(0.0),
        };

        let mut sys = get_system();
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            false,
            sysinfo::ProcessRefreshKind::nothing().with_cpu(),
        );

        let pid_obj = sysinfo::Pid::from_u32(pid);
        let process: Option<&Process> = sys.process(pid_obj);

        match process {
            Some(p) => Ok(p.cpu_usage() as f64),
            None => Ok(0.0),
        }
    }

    pub fn get_disk_usage(&self) -> Result<u64, AppError> {
        Ok(0)
    }

    pub fn get_network_usage(&self) -> Result<u64, AppError> {
        Ok(0)
    }

    pub fn check_available(&self, quota: &ResourceQuota) -> Result<(), AppError> {
        let usage = self.get_current_usage()?;

        if let Some(exceeded) = usage.exceeds_quota(quota) {
            return Err(AppError::resource_quota_exceeded(exceeded.to_string()));
        }

        Ok(())
    }

    pub fn get_system_info(&self) -> SystemInfo {
        let sys = get_system();
        let total_memory = sys.total_memory() / 1024;
        let total_swap = sys.total_swap() / 1024;
        let cpu_count = sys.cpus().len();

        SystemInfo {
            total_memory_mb: total_memory,
            total_swap_mb: total_swap,
            cpu_count,
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub total_memory_mb: u64,
    pub total_swap_mb: u64,
    pub cpu_count: usize,
}