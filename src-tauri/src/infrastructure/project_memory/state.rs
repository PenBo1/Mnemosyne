//! ═══════════════════════════════════════════════════════════════════════════
//! 项目记忆状态 - 状态管理
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use super::store::ProjectMemoryStore;
use crate::infrastructure::fs::data_dir::DataDir;

#[derive(Clone)]
pub struct ProjectMemoryState {
    pub store: Arc<ProjectMemoryStore>,
}

impl ProjectMemoryState {
    pub fn new(data_dir: DataDir) -> Self {
        Self {
            store: Arc::new(ProjectMemoryStore::new(data_dir)),
        }
    }
}
