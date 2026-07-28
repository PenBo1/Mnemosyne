//! ═══════════════════════════════════════════════════════════════════════════
//! 反馈错误 - 反馈模块错误类型
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;

// ── 错误类型 ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum FeedbackError {
    NotFound(String),
    StoreError(String),
    RetrievalError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

// ── 错误实现 ────────────────────────────────────────────────────────────────

impl std::fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "反馈不存在: {}", id),
            Self::StoreError(msg) => write!(f, "存储错误: {}", msg),
            Self::RetrievalError(msg) => write!(f, "检索错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<FeedbackError> for String {
    fn from(value: FeedbackError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for FeedbackError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}