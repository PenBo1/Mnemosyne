//! ═══════════════════════════════════════════════════════════════════════════
//! 回忆记忆存储 - 对话历史检索
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::recall_memory::RecallMemoryRow;
use crate::shared::error::AppError;

pub struct RecallMemoryStore {
    db: Database,
}

impl RecallMemoryStore {
    pub fn new(db: Database) -> Arc<Self> {
        Arc::new(Self { db })
    }

    pub fn insert(
        &self,
        id: &str,
        session_id: &str,
        message_index: i64,
        role: &str,
        content: &str,
    ) -> Result<(), AppError> {
        if content.trim().is_empty() {
            return Err(AppError::invalid_input("Content cannot be empty"));
        }

        let valid_roles = ["user", "assistant", "system"];
        if !valid_roles.contains(&role) {
            return Err(AppError::invalid_input(format!(
                "Invalid role: {} (allowed: {:?})",
                role, valid_roles
            )));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let row = RecallMemoryRow {
            id: id.to_string(),
            session_id: session_id.to_string(),
            message_index,
            role: role.to_string(),
            content: content.to_string(),
            created_at: now,
        };

        self.db.upsert_recall_memory(&row)?;
        Ok(())
    }

    pub fn search(
        &self,
        session_id: Option<&str>,
        role: Option<&str>,
        query: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RecallMemoryRow>, AppError> {
        let limit = limit.clamp(1, 500);
        self.db.search_recall_memory(session_id, role, query, limit)
    }

    pub fn list_by_session(&self, session_id: &str, limit: i64) -> Result<Vec<RecallMemoryRow>, AppError> {
        let limit = limit.clamp(1, 500);
        self.db.list_recall_by_session(session_id, limit)
    }

    pub fn delete_by_session(&self, session_id: &str) -> Result<u64, AppError> {
        self.db.delete_recall_by_session(session_id)
    }
}