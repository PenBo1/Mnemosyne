
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::shared::error::AppError;

/// 数据库句柄。内部 `Arc<Mutex<Connection>>`,克隆廉价且共享同一连接,
/// 允许多个模块(如 AgentEngine)持有副本以写入指标。
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

pub fn validate_name(name: &str, field: &str) -> Result<(), AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input(format!("{} cannot be empty", field)));
    }
    if trimmed.len() > 255 {
        return Err(AppError::invalid_input(format!("{} too long (max 255 chars)", field)));
    }
    Ok(())
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create db directory: {}", e)))?;

        let mut conn = Connection::open(db_path)
            .map_err(db_err)?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;\
             PRAGMA foreign_keys = ON;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA busy_timeout = 5000;\
             PRAGMA temp_store = MEMORY;\
             PRAGMA cache_size = -20000;",
        ).map_err(db_err)?;

        crate::infrastructure::db::migrate::run_migrate(&mut conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    #[cfg(test)]
    pub fn connect_in_memory() -> Result<Self, AppError> {
        let mut conn = Connection::open_in_memory().map_err(db_err)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(db_err)?;
        crate::infrastructure::db::migrate::run_migrate(&mut conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn.lock().map_err(|e| AppError::internal(format!("Database mutex poisoned: {}", e)))
    }
}

pub(super) fn db_err(e: rusqlite::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}