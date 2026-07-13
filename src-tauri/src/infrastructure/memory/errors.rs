
use std::path::PathBuf;

#[derive(Debug)]
pub enum MemoryError {
    NotFound(String),
    StoreError(String),
    RetrievalError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "记忆不存在: {}", id),
            Self::StoreError(msg) => write!(f, "存储错误: {}", msg),
            Self::RetrievalError(msg) => write!(f, "检索错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<MemoryError> for String {
    fn from(value: MemoryError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for MemoryError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}