
use std::path::PathBuf;

#[derive(Debug)]
pub enum SkillError {
    NotFound(String),
    DiscoveryError(String),
    LoadError(String),
    ExecuteError(String),
    InvalidInput(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for SkillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "技能不存在: {}", id),
            Self::DiscoveryError(msg) => write!(f, "技能发现错误: {}", msg),
            Self::LoadError(msg) => write!(f, "技能加载错误: {}", msg),
            Self::ExecuteError(msg) => write!(f, "技能执行错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<SkillError> for String {
    fn from(value: SkillError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for SkillError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}