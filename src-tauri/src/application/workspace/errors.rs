
//! ═══════════════════════════════════════════════════════════════════════════
//! Errors - 工作区错误定义
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;

#[derive(Debug)]
pub enum WorkspaceError {
    NotFound(String),
    AlreadyExists(String),
    IsolationError(String),
    InvalidState(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "工作区不存在: {}", id),
            Self::AlreadyExists(id) => write!(f, "工作区已存在: {}", id),
            Self::IsolationError(msg) => write!(f, "隔离错误: {}", msg),
            Self::InvalidState(msg) => write!(f, "工作区状态无效: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<WorkspaceError> for String {
    fn from(value: WorkspaceError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for WorkspaceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}