//! ═══════════════════════════════════════════════════════════════════════════
//! 用户错误 - 错误类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;

#[derive(Debug)]
pub enum UserError {
    NotFound(String),
    ProfileError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "用户不存在: {}", id),
            Self::ProfileError(msg) => write!(f, "用户配置错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<UserError> for String {
    fn from(value: UserError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for UserError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}