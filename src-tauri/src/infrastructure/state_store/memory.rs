use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
// MemoryEntry / MemoryType 已下沉到 shared/memory，修复 infra → core/agent 反向依赖。
// MemorySystem 仍留在 core/agent（它是 agent 内部的工具系统，不被 infra 持久化）。
use crate::shared::memory::{MemoryEntry, MemoryType};
use crate::core::agent::MemorySystem;

const DEFAULT_BUDGET: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryData {
    budget: usize,
    entries: Vec<MemoryEntry>,
}

/// 跨重启持久化的 memory store。
/// 每个 book 拥有独立的 MemorySystem，存储在 JSON 文件中。
pub struct MemoryStore {
    books: RwLock<HashMap<String, Arc<RwLock<MemorySystem>>>>,
    data_dir: PathBuf,
}

impl MemoryStore {
    pub fn new(data_dir: PathBuf) -> Arc<Self> {
        let store = Arc::new(Self {
            books: RwLock::new(HashMap::new()),
            data_dir,
        });
        store.load_all_sync();
        store
    }

    fn load_all_sync(&self) {
        let memory_dir = self.data_dir.join("memory");
        if !memory_dir.exists() {
            let _ = std::fs::create_dir_all(&memory_dir);
            return;
        }
        let entries = match std::fs::read_dir(&memory_dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let mut books = match self.books.try_write() {
            Ok(b) => b,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let book_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if book_id.is_empty() { continue; }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<MemoryData>(&content) {
                    let mut memory = MemorySystem::new(data.budget);
                    for e in data.entries { memory.archive(e); }
                    books.insert(book_id, Arc::new(RwLock::new(memory)));
                }
            }
        }
    }

    pub async fn save_book(&self, book_id: &str) {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            let entries: Vec<MemoryEntry> = mem.get_all_entries().into_iter().cloned().collect();
            drop(mem);
            let data = MemoryData { budget: DEFAULT_BUDGET, entries };
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let memory_dir = self.data_dir.join("memory");
                let _ = tokio::fs::create_dir_all(&memory_dir).await;
                let _ = tokio::fs::write(memory_dir.join(format!("{}.json", book_id)), json).await;
            }
        }
    }

    pub async fn get_or_create(&self, book_id: &str, budget: usize) -> Arc<RwLock<MemorySystem>> {
        let mut books = self.books.write().await;
        books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(budget))))
            .clone()
    }

    pub async fn get(&self, book_id: &str) -> Option<Arc<RwLock<MemorySystem>>> {
        self.books.read().await.get(book_id).cloned()
    }

    pub async fn archive_fact(&self, book_id: &str, chapter: u32, subject: &str, predicate: &str, object: &str, category: &str) {
        let entry_type = match category {
            "character" => MemoryType::Character,
            "plot" => MemoryType::Plot,
            "setting" => MemoryType::Setting,
            "dialogue" => MemoryType::Dialogue,
            "style" => MemoryType::Style,
            _ => MemoryType::Fact,
        };
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: format!("{} {} {}", subject, predicate, object),
            entry_type,
            chapter: Some(chapter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: vec![category.to_string(), subject.to_lowercase()],
        };
        {
            let mut books = self.books.write().await;
            let memory = books.entry(book_id.to_string())
                .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));
            memory.write().await.archive(entry);
        }
        self.save_book(book_id).await;
    }

    pub async fn archive_hook(&self, book_id: &str, chapter: u32, name: &str, hook_type: &str, status: &str, description: &str) {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: format!("[Hook:{}] {} - {} ({})", hook_type, name, description, status),
            entry_type: MemoryType::Plot,
            chapter: Some(chapter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: vec!["hook".to_string(), hook_type.to_string(), name.to_lowercase()],
        };
        {
            let mut books = self.books.write().await;
            let memory = books.entry(book_id.to_string())
                .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));
            memory.write().await.archive(entry);
        }
        self.save_book(book_id).await;
    }

    pub async fn archive_summary(&self, book_id: &str, chapter: u32, title: &str, characters: &[String], events: &[String]) {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content: format!("Chapter {}: {} | Characters: {} | Events: {}", chapter, title, characters.join(", "), events.join("; ")),
            entry_type: MemoryType::Fact,
            chapter: Some(chapter),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags: vec!["summary".to_string(), format!("ch{}", chapter)],
        };
        {
            let mut books = self.books.write().await;
            let memory = books.entry(book_id.to_string())
                .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));
            memory.write().await.archive(entry);
        }
        self.save_book(book_id).await;
    }

    pub async fn search(&self, book_id: &str, query: &str, top_k: usize) -> Vec<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            memory.read().await.search_memory(query, top_k).into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 列出某本书的全部 memory 条目（main_context + archival_store）
    pub async fn list_all(&self, book_id: &str) -> Vec<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            memory.read().await.get_all_entries().into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 用户手动创建 memory 条目（与 Agent 自动 archive_* 区分）
    pub async fn create_manual(
        &self,
        book_id: &str,
        content: String,
        entry_type_str: &str,
        chapter: Option<u32>,
        tags: Vec<String>,
    ) -> MemoryEntry {
        let entry_type = match entry_type_str {
            "character" => MemoryType::Character,
            "plot" => MemoryType::Plot,
            "setting" => MemoryType::Setting,
            "dialogue" => MemoryType::Dialogue,
            "style" => MemoryType::Style,
            _ => MemoryType::Fact,
        };
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            entry_type,
            chapter,
            timestamp: chrono::Utc::now().to_rfc3339(),
            tags,
        };
        {
            let mut books = self.books.write().await;
            let memory = books.entry(book_id.to_string())
                .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET))));
            memory.write().await.archive(entry.clone());
        }
        self.save_book(book_id).await;
        entry
    }

    /// 删除指定 entry_id 的 memory 条目（返回是否删除成功）
    pub async fn delete_entry(&self, book_id: &str, entry_id: &str) -> bool {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let deleted = memory.write().await.delete_entry(entry_id);
            if deleted {
                drop(books);
                self.save_book(book_id).await;
            }
            return deleted;
        }
        false
    }

    /// 更新指定 entry_id 的 memory 条目（content + tags），返回更新后的条目
    pub async fn update_entry(
        &self,
        book_id: &str,
        entry_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Option<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let updated = memory.write().await.update_entry(entry_id, content, tags);
            if updated.is_some() {
                drop(books);
                self.save_book(book_id).await;
            }
            return updated;
        }
        None
    }

    pub async fn format_context(&self, book_id: &str) -> String {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            memory.read().await.format_main_context()
        } else {
            String::new()
        }
    }

    pub async fn stats(&self, book_id: &str) -> (usize, usize) {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            let main = mem.main_context_len();
            let archival = mem.archival_store_len();
            (main, archival)
        } else {
            (0, 0)
        }
    }
}
