//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆状态 - 记忆系统状态管理
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use super::store::MemoryStore;
use crate::infrastructure::db::connection::Database;
use std::path::PathBuf;

#[derive(Clone)]
pub struct MemoryState {
    pub store: Arc<MemoryStore>,
}

impl MemoryState {
    pub fn new(db: Database, data_dir: PathBuf) -> Self {
        Self { store: MemoryStore::new(db, data_dir) }
    }
}
