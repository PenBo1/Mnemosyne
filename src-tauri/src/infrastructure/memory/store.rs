
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use crate::infrastructure::memory::types::{MemoryEntry, MemoryType, MemorySystem};

const DEFAULT_BUDGET: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryData { budget: usize, entries: Vec<MemoryEntry> }

pub struct MemoryStore {
    books: RwLock<HashMap<String, Arc<RwLock<MemorySystem>>>>,
    data_dir: PathBuf,
}

fn make_entry(key: String, value: String, memory_type: MemoryType, source: &str, chapter: Option<String>, tags: Option<Vec<String>>) -> MemoryEntry {
    MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(), key, value: value.clone(), memory_type: memory_type.clone(), source: source.to_string(),
        importance: 1, created_at: chrono::Utc::now().to_rfc3339(), updated_at: chrono::Utc::now().to_rfc3339(),
        content: Some(value), entry_type: Some(memory_type.to_string()), chapter, timestamp: Some(chrono::Utc::now().to_rfc3339()), tags,
    }
}

impl MemoryStore {
    pub fn new(data_dir: PathBuf) -> Arc<Self> {
        let store = Arc::new(Self { books: RwLock::new(HashMap::new()), data_dir });
        store.load_all_sync();
        store
    }

    fn load_all_sync(&self) {
        let memory_dir = self.data_dir.join("memory");
        if !memory_dir.exists() { let _ = std::fs::create_dir_all(&memory_dir); return; }
        let entries = match std::fs::read_dir(&memory_dir) { Ok(e) => e, Err(_) => return };
        let mut books = match self.books.try_write() { Ok(b) => b, Err(_) => return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
            let book_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if book_id.is_empty() { continue; }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<MemoryData>(&content) {
                    let mut memory = MemorySystem::new(data.budget as u32);
                    for e in data.entries { let _ = memory.archive(&e.id); }
                    books.insert(book_id, Arc::new(RwLock::new(memory)));
                }
            }
        }
    }

    pub async fn save_book(&self, book_id: &str) {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            let entries: Vec<MemoryEntry> = mem.get_all_entries().iter().cloned().collect();
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
        books.entry(book_id.to_string()).or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(budget as u32)))).clone()
    }

    pub async fn get(&self, book_id: &str) -> Option<Arc<RwLock<MemorySystem>>> { self.books.read().await.get(book_id).cloned() }

    pub async fn archive_fact(&self, book_id: &str, chapter: u32, subject: &str, predicate: &str, object: &str, category: &str) {
        let entry_type = match category { "character" => MemoryType::Character, "plot" => MemoryType::Plot, "setting" => MemoryType::Setting, "dialogue" => MemoryType::Dialogue, "style" => MemoryType::Style, _ => MemoryType::Fact };
        let value = format!("{} {} {}", subject, predicate, object);
        let entry = make_entry(format!("{}_{}", subject, category), value.clone(), entry_type, "archive_fact", Some(chapter.to_string()), Some(vec![category.to_string(), subject.to_lowercase()]));
        { let mut books = self.books.write().await; let memory = books.entry(book_id.to_string()).or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET as u32)))); let _ = memory.write().await.archive(&entry.id); }
        self.save_book(book_id).await;
    }

    pub async fn archive_hook(&self, book_id: &str, chapter: u32, name: &str, hook_type: &str, status: &str, description: &str) {
        let value = format!("[Hook:{}] {} - {} ({})", hook_type, name, description, status);
        let entry = make_entry(format!("hook_{}", name), value.clone(), MemoryType::Plot, "archive_hook", Some(chapter.to_string()), Some(vec!["hook".to_string(), hook_type.to_string(), name.to_lowercase()]));
        { let mut books = self.books.write().await; let memory = books.entry(book_id.to_string()).or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET as u32)))); let _ = memory.write().await.archive(&entry.id); }
        self.save_book(book_id).await;
    }

    pub async fn archive_summary(&self, book_id: &str, chapter: u32, title: &str, characters: &[String], events: &[String]) {
        let value = format!("Chapter {}: {} | Characters: {} | Events: {}", chapter, title, characters.join(", "), events.join("; "));
        let entry = make_entry(format!("summary_ch{}", chapter), value.clone(), MemoryType::Fact, "archive_summary", Some(chapter.to_string()), Some(vec!["summary".to_string(), format!("ch{}", chapter)]));
        { let mut books = self.books.write().await; let memory = books.entry(book_id.to_string()).or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET as u32)))); let _ = memory.write().await.archive(&entry.id); }
        self.save_book(book_id).await;
    }

    pub async fn search(&self, book_id: &str, query: &str, _top_k: usize) -> Vec<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) { memory.read().await.get_all_entries().iter().filter(|e| e.value.contains(query) || e.key.contains(query)).cloned().collect() } else { Vec::new() }
    }

    pub async fn list_all(&self, book_id: &str) -> Vec<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) { memory.read().await.get_all_entries().iter().cloned().collect() } else { Vec::new() }
    }

    pub async fn create_manual(&self, book_id: &str, content: String, entry_type_str: &str, chapter: Option<u32>, tags: Vec<String>) -> MemoryEntry {
        let entry_type = match entry_type_str { "character" => MemoryType::Character, "plot" => MemoryType::Plot, "setting" => MemoryType::Setting, "dialogue" => MemoryType::Dialogue, "style" => MemoryType::Style, _ => MemoryType::Fact };
        let entry = make_entry(format!("manual_{}", uuid::Uuid::new_v4()), content.clone(), entry_type, "manual", chapter.map(|c| c.to_string()), Some(tags));
        { let mut books = self.books.write().await; let memory = books.entry(book_id.to_string()).or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(DEFAULT_BUDGET as u32)))); let _ = memory.write().await.archive(&entry.id); }
        self.save_book(book_id).await;
        entry
    }

    pub async fn delete_entry(&self, book_id: &str, entry_id: &str) -> bool {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let deleted = memory.write().await.delete_entry(entry_id).is_ok();
            if deleted { drop(books); self.save_book(book_id).await; }
            return deleted;
        }
        false
    }

    pub async fn update_entry(&self, book_id: &str, entry_id: &str, content: String, _tags: Vec<String>) -> Option<MemoryEntry> {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let updated = memory.write().await.update_entry(entry_id, &content).is_ok();
            if updated { drop(books); self.save_book(book_id).await; return Some(make_entry(entry_id.to_string(), content, MemoryType::Fact, "update", None, None)); }
        }
        None
    }

    pub async fn format_context(&self, book_id: &str) -> String {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) { memory.read().await.format_main_context() } else { String::new() }
    }

    pub async fn stats(&self, book_id: &str) -> (usize, usize) {
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            let mem = memory.read().await;
            let main = mem.get_all_entries().iter().filter(|e| e.importance > 0).count();
            let archival = mem.get_all_entries().iter().filter(|e| e.importance == 0).count();
            (main, archival)
        } else { (0, 0) }
    }
}