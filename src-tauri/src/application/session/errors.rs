
use std::path::PathBuf;

#[derive(Debug)]
pub enum SessionError {
    NotFound(String),
    InvalidState(String),
    InvalidTransition(String),
    InvalidInput(String),
    MessageError(String),
    ApprovalError(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "会话不存在: {}", id),
            Self::InvalidState(msg) => write!(f, "会话状态无效: {}", msg),
            Self::InvalidTransition(msg) => write!(f, "状态转换无效: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::MessageError(msg) => write!(f, "消息错误: {}", msg),
            Self::ApprovalError(msg) => write!(f, "审批错误: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<SessionError> for String {
    fn from(value: SessionError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for SessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}