//! ═══════════════════════════════════════════════════════════════════════════
//! 工作区错误 - 错误类型定义
//! ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub enum WorkspaceError {
    Io(std::io::Error),
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<std::io::Error> for WorkspaceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
