//! ═══════════════════════════════════════════════════════════════════════════
//! 数据库迁移 - Schema 初始化
//! ═══════════════════════════════════════════════════════════════════════════

use rusqlite::Connection;

use crate::shared::error::AppError;

/// 初始化数据库 schema。
///
/// 开发阶段采用单一最终态 schema 模式：所有表/索引/触发器合并到 `schema.sql`，
/// 通过 `execute_batch` 一次性应用。所有 CREATE 语句均使用 `IF NOT EXISTS`，
/// 保证重复调用幂等。`PRAGMA user_version = 1` 标记 schema 已初始化。
pub fn run_migrate(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(include_str!("schema.sql"))
        .map_err(|e| AppError::internal(format!("Schema init failed: {}", e)))?;
    conn.execute_batch("PRAGMA user_version = 1")
        .map_err(|e| AppError::internal(format!("Set user_version failed: {}", e)))?;
    Ok(())
}