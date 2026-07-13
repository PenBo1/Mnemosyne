
use std::path::PathBuf;

#[derive(Debug)]
pub enum GitError {
    NotFound(String),
    RepositoryError(String),
    BranchError(String),
    CommitError(String),
    MergeConflict(String),
    PushError(String),
    PullError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Git 资源不存在: {}", id),
            Self::RepositoryError(msg) => write!(f, "仓库错误: {}", msg),
            Self::BranchError(msg) => write!(f, "分支错误: {}", msg),
            Self::CommitError(msg) => write!(f, "提交错误: {}", msg),
            Self::MergeConflict(msg) => write!(f, "合并冲突: {}", msg),
            Self::PushError(msg) => write!(f, "推送错误: {}", msg),
            Self::PullError(msg) => write!(f, "拉取错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<GitError> for String {
    fn from(value: GitError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for GitError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}