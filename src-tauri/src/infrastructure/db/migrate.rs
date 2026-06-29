use sqlx::SqlitePool;
use crate::shared::errors::AppError;

/// 执行所有未应用的迁移。
///
/// `sqlx::migrate!` 宏在编译期嵌入 `migrations/` 目录的 SQL 文件，
/// 运行时自动维护 `_sqlx_migrations` 表跟踪版本。
pub async fn run_migrate(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::internal(format!("Database migration failed: {}", e)))?;
    Ok(())
}
