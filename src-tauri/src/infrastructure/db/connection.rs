//! ═══════════════════════════════════════════════════════════════════════════
//! 数据库连接 - SQLite 连接管理
//! ═══════════════════════════════════════════════════════════════════════════

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::shared::error::AppError;

/// 慢查询阈值（毫秒）
const SLOW_QUERY_THRESHOLD_MS: u64 = 1000;

// ── 数据库连接 ──────────────────────────────────────────────────────────────

/// 数据库连接封装
#[derive(Clone)]
pub struct Database {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
    /// 数据库路径
    path: String,
}

/// 验证名称字段
pub fn validate_name(name: &str, field: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        tracing::debug!(field = field, "Name validation failed: empty");
        return Err(AppError::invalid_input(format!("{} cannot be empty", field)));
    }
    if trimmed.len() > 255 {
        tracing::debug!(field = field, len = trimmed.len(), "Name validation failed: too long");
        return Err(AppError::invalid_input(format!("{} too long (max 255 chars)", field)));
    }
    Ok(())
}

impl Database {
    /// 创建新的数据库连接
    pub fn new(db_path: &str) -> Result<Self, AppError> {
        let start_time = Instant::now();
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create db directory: {}", e)))?;

        let mut conn = Connection::open(db_path)
            .map_err(db_err)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA temp_store = MEMORY;
             PRAGMA cache_size = -20000;",
        ).map_err(db_err)?;

        crate::infrastructure::db::migrate::run_migrate(&mut conn)?;
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        tracing::info!(
            path = %db_path,
            duration_ms = duration_ms,
            status = "connected",
            "[db] database connection established"
        );
        
        Ok(Self { 
            conn: Arc::new(Mutex::new(conn)),
            path: db_path.to_string(),
        })
    }

    /// 创建内存数据库（测试用）
    #[cfg(test)]
    pub fn connect_in_memory() -> Result<Self, AppError> {
        let mut conn = Connection::open_in_memory().map_err(db_err)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(db_err)?;
        crate::infrastructure::db::migrate::run_migrate(&mut conn)?;
        tracing::info!("[db] in-memory database connected (test mode)");
        Ok(Self { 
            conn: Arc::new(Mutex::new(conn)),
            path: ":memory:".to_string(),
        })
    }

    /// 获取数据库连接
    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn.lock().map_err(|e| AppError::internal(format!("Database mutex poisoned: {}", e)))
    }

    /// 执行带计时的数据库操作
    pub fn execute_with_timing<F, T>(&self, operation: &str, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let start_time = Instant::now();
        let guard = self.conn()?;
        let result = f(&guard);
        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        if duration_ms > SLOW_QUERY_THRESHOLD_MS {
            tracing::warn!(
                operation = operation,
                duration_ms = duration_ms,
                path = %self.path,
                "[db] slow query detected"
            );
        } else {
            tracing::debug!(
                operation = operation,
                duration_ms = duration_ms,
                "[db] query executed"
            );
        }
        
        result.map_err(db_err)
    }
}

/// 将 rusqlite 错误转换为应用错误
pub(super) fn db_err(e: rusqlite::Error) -> AppError {
    tracing::error!(error = %e, "[db] database error");
    AppError::internal(format!("Database error: {}", e))
}