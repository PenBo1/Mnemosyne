use std::sync::Arc;
use tokio::sync::Mutex;
use crate::errors::AppError;
use crate::infra::db::Database;
use super::models::*;

pub struct WikiService {
    db: Arc<Mutex<Database>>,
}

impl WikiService {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// List all wiki entries for a novel
    pub async fn list_entries(
        &self,
        novel_id: &str,
        category: Option<&WikiCategory>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let db = self.db.lock().await;
        db.list_wiki_entries(novel_id, category)
    }

    /// Get a single wiki entry by ID
    pub async fn get_entry(&self, entry_id: &str) -> Result<Option<WikiEntry>, AppError> {
        let db = self.db.lock().await;
        db.get_wiki_entry(entry_id)
    }

    /// Create a new wiki entry
    pub async fn create_entry(
        &self,
        request: &CreateWikiEntryRequest,
    ) -> Result<WikiEntry, AppError> {
        let db = self.db.lock().await;
        db.create_wiki_entry(request)
    }

    /// Update an existing wiki entry
    pub async fn update_entry(
        &self,
        entry_id: &str,
        request: &UpdateWikiEntryRequest,
    ) -> Result<WikiEntry, AppError> {
        let db = self.db.lock().await;
        db.update_wiki_entry(entry_id, request)
    }

    /// Delete a wiki entry
    pub async fn delete_entry(&self, entry_id: &str) -> Result<bool, AppError> {
        let db = self.db.lock().await;
        db.delete_wiki_entry(entry_id)
    }

    /// Get wiki graph view for visualization
    pub async fn get_graph_view(
        &self,
        novel_id: &str,
        filter_category: Option<&WikiCategory>,
        min_importance: Option<u32>,
    ) -> Result<WikiGraphView, AppError> {
        let db = self.db.lock().await;
        db.get_wiki_graph_view(novel_id, filter_category, min_importance)
    }

    /// Create a wiki entity link
    pub async fn create_link(
        &self,
        request: &CreateWikiLinkRequest,
    ) -> Result<WikiEntityLink, AppError> {
        let db = self.db.lock().await;
        db.create_wiki_link(request)
    }

    /// Delete a wiki entity link
    pub async fn delete_link(&self, link_id: &str) -> Result<bool, AppError> {
        let db = self.db.lock().await;
        db.delete_wiki_link(link_id)
    }

    /// Search wiki entries by query
    pub async fn search_entries(
        &self,
        novel_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let db = self.db.lock().await;
        db.search_wiki_entries(novel_id, query, limit)
    }

    /// Get wiki entries relevant for a chapter (AI context integration)
    pub async fn get_context_for_chapter(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<WikiEntry>, AppError> {
        let db = self.db.lock().await;
        db.get_wiki_context_for_chapter(novel_id, chapter_number)
    }

    /// Get wiki entry summaries for AI context
    pub async fn get_entry_summaries(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<WikiEntrySummary>, AppError> {
        let entries = self.get_context_for_chapter(novel_id, chapter_number).await?;
        Ok(entries
            .into_iter()
            .map(|e| WikiEntrySummary {
                id: e.id,
                title: e.title,
                category: e.category.to_string(),
                excerpt: if e.content.len() > 500 {
                    e.content[..500].to_string()
                } else {
                    e.content
                },
                importance: e.importance,
            })
            .collect())
    }
}