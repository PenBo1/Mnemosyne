use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_exec: bool,
    pub allow_network: bool,
    pub blocked_commands: Vec<String>,
    pub blocked_domains: Vec<String>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            allow_read: true,
            allow_write: true,
            allow_exec: false,
            allow_network: true,
            blocked_commands: vec![
                "rm".to_string(), "del".to_string(), "format".to_string(),
                "fdisk".to_string(), "mkfs".to_string(), "shutdown".to_string(),
                "reboot".to_string(), "halt".to_string(), "poweroff".to_string(),
                "init".to_string(), "systemctl".to_string(),
            ],
            blocked_domains: vec![
                "metadata.google.internal".to_string(),
                "metadata.azure.com".to_string(),
                "169.254.169.254".to_string(),
            ],
        }
    }
}