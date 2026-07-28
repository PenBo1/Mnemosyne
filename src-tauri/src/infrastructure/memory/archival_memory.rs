//! ═══════════════════════════════════════════════════════════════════════════
//! 归档记忆存储 - 内容归档与向量检索
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::archival_memory::{ArchivalMemoryRow, ArchivalSearchResult};
use crate::shared::error::AppError;

pub struct ArchivalMemoryStore {
    db: Database,
}

impl ArchivalMemoryStore {
    pub fn new(db: Database) -> Arc<Self> {
        Arc::new(Self { db })
    }

    pub fn insert(
        &self,
        id: &str,
        content: &str,
        tags: &[&str],
        citation: Option<&str>,
        embedding_vec: Option<&[f32]>,
        embedding_model: Option<&str>,
        embedding_dim: Option<usize>,
    ) -> Result<(), AppError> {
        if content.trim().is_empty() {
            return Err(AppError::invalid_input("Content cannot be empty"));
        }

        let tags_json = if tags.is_empty() {
            "[]".to_string()
        } else {
            serde_json::to_string(tags)
                .map_err(|e| AppError::internal(format!("Failed to serialize tags: {}", e)))?
        };

        let citation_json = citation.map(|c| {
            serde_json::to_string(&serde_json::json!({"raw": c}))
                .unwrap_or_else(|_| c.to_string())
        });

        self.db.upsert_archival_memory(
            id,
            content,
            &tags_json,
            citation_json.as_deref(),
            embedding_vec,
            embedding_model,
            embedding_dim,
        )?;
        Ok(())
    }

    pub fn search_by_content(
        &self,
        query: &str,
        tags_filter: Option<&[&str]>,
        limit: i64,
    ) -> Result<Vec<ArchivalMemoryRow>, AppError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, 200);
        self.db.search_archival_memory_by_content(query, tags_filter, limit)
    }

    pub fn search_by_vector(
        &self,
        query_vec: &[f32],
        embedding_model: &str,
        tags_filter: Option<&[&str]>,
        limit: i64,
    ) -> Result<Vec<ArchivalSearchResult>, AppError> {
        if query_vec.is_empty() {
            return Err(AppError::invalid_input("Query vector cannot be empty"));
        }
        let limit = limit.clamp(1, 100);
        self.db.search_archival_memory_by_vector(query_vec, embedding_model, tags_filter, limit)
    }

    pub fn list_by_tags(&self, tags: &[&str], limit: i64) -> Result<Vec<ArchivalMemoryRow>, AppError> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }
        let limit = limit.clamp(1, 200);
        self.db.list_archival_memory_by_tags(tags, limit)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<ArchivalMemoryRow>, AppError> {
        self.db.get_archival_memory_by_id(id)
    }

    pub fn delete(&self, id: &str) -> Result<bool, AppError> {
        self.db.delete_archival_memory(id)
    }
}