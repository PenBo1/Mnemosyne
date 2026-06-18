use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::domain::agents::base::{MemoryEntry, MemoryType, MemorySystem};

const DEFAULT_BUDGET: usize = 20;

/// Shared memory store that persists across pipeline runs.
/// Each book gets its own `Arc<RwLock<MemorySystem>>` so data is shared, not copied.
pub struct MemoryStore {
    books: RwLock<HashMap<String, Arc<RwLock<MemorySystem>>>>,
}

impl MemoryStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            books: RwLock::new(HashMap::new()),
        })
    }

    /// Get or create the memory system for a book.
    /// Returns the SAME Arc<RwLock<MemorySystem>> on repeated calls — data persists.
    pub async fn get_or_create(&self, book_id: &str, budget: usize) -> Arc<RwLock<MemorySystem>> {
        let mut books = self.books.write().await;
        books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(budget))))
            .clone()
    }

    /// Get a reference to the memory system for a book (returns None if not created).
    pub async fn get(&self, book_id: &str) -> Option<Arc<RwLock<MemorySystem>>> {
        let books = self.books.read().await;
        books.get(book_id).cloned()
    }

    /// Archive a fact extracted by ObserverAgent.
    pub async fn archive_fact(
        &self,
        book_id: &str,
        chapter: u32,
        subject: &str,
        predicate: &str,
        object: &str,
        category: &str,
    ) {
        let mut books = self.books.write().await;
        let memory = books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));

        let entry_type = match category {
            "character" => MemoryType::Character,
            "plot" => MemoryType::Plot,
            "setting" => MemoryType::Setting,
            "dialogue" => MemoryType::Dialogue,
            "style" => MemoryType::Style,
            _ => MemoryType::Fact,
        };

        let content = format!("{} {} {}", subject, predicate, object);
        let now = chrono::Utc::now().to_rfc3339();

        memory.write().await.archive(MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            entry_type,
            chapter: Some(chapter),
            timestamp: now,
            tags: vec![category.to_string(), subject.to_lowercase()],
        });
    }

    /// Archive a hook action.
    pub async fn archive_hook(
        &self,
        book_id: &str,
        chapter: u32,
        name: &str,
        hook_type: &str,
        status: &str,
        description: &str,
    ) {
        let mut books = self.books.write().await;
        let memory = books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));

        let content = format!("[Hook:{}] {} - {} ({})", hook_type, name, description, status);
        let now = chrono::Utc::now().to_rfc3339();

        memory.write().await.archive(MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            entry_type: MemoryType::Plot,
            chapter: Some(chapter),
            timestamp: now,
            tags: vec!["hook".to_string(), hook_type.to_string(), name.to_lowercase()],
        });
    }

    /// Archive a chapter summary.
    pub async fn archive_summary(
        &self,
        book_id: &str,
        chapter: u32,
        title: &str,
        characters: &[String],
        events: &[String],
    ) {
        let mut books = self.books.write().await;
        let memory = books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));

        let content = format!(
            "Chapter {}: {} | Characters: {} | Events: {}",
            chapter,
            title,
            characters.join(", "),
            events.join("; ")
        );
        let now = chrono::Utc::now().to_rfc3339();

        memory.write().await.archive(MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            entry_type: MemoryType::Fact,
            chapter: Some(chapter),
            timestamp: now,
            tags: vec!["summary".to_string(), format!("ch{}", chapter)],
        });
    }

    /// Search memory for a book.
    pub async fn search(&self, book_id: &str, query: &str, top_k: usize) -> Vec<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            mem.search_memory(query, top_k)
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get formatted main context for prompt injection.
    pub async fn format_context(&self, book_id: &str) -> String {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            mem.format_main_context()
        } else {
            String::new()
        }
    }

    /// Get counts for a book's memory.
    pub async fn stats(&self, book_id: &str) -> (usize, usize) {
        let books = self.books.read().await;
        if let Some(_memory) = books.get(book_id) {
            // TODO: expose main_context.len() and archival_store.len() from MemorySystem
            (0, 0)
        } else {
            (0, 0)
        }
    }
}
