//! ═══════════════════════════════════════════════════════════════════════════
//! 数据库状态 - 应用状态管理
//! ═══════════════════════════════════════════════════════════════════════════

use super::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

/// 数据库状态
pub struct DbState {
    /// 数据库实例
    pub db: Database,
    /// 数据目录
    pub data_dir: DataDir,
}

impl DbState {
    /// 创建新的数据库状态
    pub fn new(data_dir: DataDir) -> Result<Self, AppError> {
        let db_path = data_dir.state_db_path();
        let db_path_str = db_path.to_str()
            .ok_or_else(|| AppError::internal("State database path is not valid UTF-8"))?;
        let database = Database::new(db_path_str)?;
        tracing::info!(path = %db_path.display(), "State database initialized (migrations applied)");
        Ok(Self { db: database, data_dir })
    }
}