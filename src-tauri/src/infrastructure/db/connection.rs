use sqlx::SqlitePool;
use std::path::Path;

use crate::shared::errors::AppError;

/// 应用核心数据库。所有 store 共享同一个连接池。
///
/// schema 通过 `sqlx::migrate!` 在 `new()` 时自动应用，无需手动加载 SQL。
#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    /// 打开/创建 state.sqlite 并执行所有未应用的迁移。
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create db directory: {}", e)))?;

        let url = format!("sqlite:{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&url).await.map_err(db_err)?;

        // 应用迁移（每次启动都执行，sqlx 自动跳过已应用的）
        crate::infrastructure::db::migrate::run_migrate(&pool).await?;

        Ok(Self { pool })
    }
}

/// 将 sqlx::Error 转换为 AppError。
pub(super) fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}
