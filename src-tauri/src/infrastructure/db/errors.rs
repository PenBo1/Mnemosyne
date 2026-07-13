
use std::path::PathBuf;

#[derive(Debug)]
pub enum DbError {
    NotFound(String),
    ConnectionError(String),
    QueryError(String),
    MigrationError(String),
    TransactionError(String),
    InvalidInput(String),
    ConstraintViolation(String),
    PermissionDenied(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "记录不存在: {}", id),
            Self::ConnectionError(msg) => write!(f, "数据库连接错误: {}", msg),
            Self::QueryError(msg) => write!(f, "查询错误: {}", msg),
            Self::MigrationError(msg) => write!(f, "迁移错误: {}", msg),
            Self::TransactionError(msg) => write!(f, "事务错误: {}", msg),
            Self::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            Self::ConstraintViolation(msg) => write!(f, "约束违反: {}", msg),
            Self::PermissionDenied(path) => write!(f, "权限拒绝: {}", path.display()),
            Self::Io(e) => write!(f, "IO 错误: {}", e),
        }
    }
}

impl From<DbError> for String {
    fn from(value: DbError) -> Self {
        value.to_string()
    }
}

impl From<std::io::Error> for DbError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}