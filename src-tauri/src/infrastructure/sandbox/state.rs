use std::path::PathBuf;
use std::sync::Mutex;
use super::policy::SandboxPolicy;
use super::types::SandboxStatus;

pub struct SandboxState {
    root: PathBuf,
    policy: Mutex<SandboxPolicy>,
}

impl SandboxState {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            policy: Mutex::new(SandboxPolicy::default()),
        }
    }

    pub fn get_status(&self) -> SandboxStatus {
        SandboxStatus {
            enabled: true,
            root_path: self.root.display().to_string(),
            mode: "strict".to_string(),
        }
    }

    pub fn validate_path(&self, path: &PathBuf, is_write: bool) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap();
        let canonical = std::fs::canonicalize(path)
            .map_err(|e| crate::shared::error::AppError::internal(format!("Failed to canonicalize: {}", e)))?;

        if !canonical.starts_with(&self.root) {
            return Ok(false);
        }

        if is_write && !policy.allow_write {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn validate_command(&self, command: &str) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap();

        for blocked in &policy.blocked_commands {
            if command.starts_with(blocked) {
                return Ok(false);
            }
        }

        Ok(policy.allow_exec)
    }

    pub fn validate_url(&self, url: &str) -> Result<bool, crate::shared::error::AppError> {
        let policy = self.policy.lock().unwrap();

        for blocked in &policy.blocked_domains {
            if url.contains(blocked) {
                return Ok(false);
            }
        }

        Ok(policy.allow_network)
    }

    pub fn get_policy(&self) -> SandboxPolicy {
        self.policy.lock().unwrap().clone()
    }
}