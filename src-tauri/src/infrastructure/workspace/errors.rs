// Workspace 授权错误 —— WorkspaceRegistry::authorize 的返回错误类型。
//
// 仅保留实际使用的 Io 变体；先前的 NotAuthorized / PathTraversal / InvalidPath
// 变体从未被构造（死代码），已移除。
// `From<WorkspaceError> for String` 也已移除（无消费方）。

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
