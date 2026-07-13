use std::path::PathBuf;

#[derive(Debug)]
pub enum WorkspaceError {
    NotAuthorized(PathBuf),
    PathTraversal,
    InvalidPath(String),
    Io(std::io::Error),
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAuthorized(p) => write!(f, "路径未授权: {}", p.display()),
            Self::PathTraversal => write!(f, "路径穿越检测"),
            Self::InvalidPath(msg) => write!(f, "路径无效: {}", msg),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<std::io::Error> for WorkspaceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<WorkspaceError> for String {
    fn from(value: WorkspaceError) -> Self {
        value.to_string()
    }
}