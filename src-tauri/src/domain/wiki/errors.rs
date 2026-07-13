
use std::path::PathBuf;

#[derive(Debug)]
pub enum WikiError {
    NotFound(String),
    InvalidTitle(String),
    InvalidContent(String),
    LinkError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for WikiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Wiki 条目不存在: {}", id),
            Self::InvalidTitle(msg) => write!(f, "标题无效: {}", msg),
            Self::InvalidContent(msg) => write!(f, "内容无效: {}", msg),
            Self::LinkError(msg) => write!(f, "链接错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<WikiError> for String {
    fn from(value: WikiError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for WikiError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}