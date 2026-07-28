//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱类型 - 数据类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub root_path: String,
    pub mode: String,
}