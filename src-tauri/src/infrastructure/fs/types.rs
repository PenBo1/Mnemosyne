//! ═══════════════════════════════════════════════════════════════════════════
//! 文件系统类型
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 文件系统错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsError {
    /// 错误信息
    pub message: String,
}

/// 文件状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    /// 文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 是否为目录
    pub is_dir: bool,
    /// 最后修改时间
    pub modified: String,
}