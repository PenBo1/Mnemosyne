use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub root_path: String,
    pub mode: String,
}