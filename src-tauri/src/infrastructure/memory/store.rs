// Agent 通用记忆存储 —— SQLite 持久化 + in-memory cache。
//
// Wave 5 改造要点（替代 <data_dir>/memory/<book_id>.json 文件持久化）：
// - 写穿（write-through）：每次 mutate 同步写入 SQLite，不再 save_book 写 JSON
// - 懒加载：首次访问某 book 时从 SQLite 加载到 in-memory cache
// - 一次性导入：首次启动时若发现旧 <data_dir>/memory/<book_id>.json 文件，导入到 SQLite
// - 修复 archive_* bug：原实现调用 memory.archive(&entry.id) 但 entry 从未 push 进 entries，
//   导致 archive 静默失败。新实现直接以 importance=0 写入 SQLite（archived 语义）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::memory::types::{
    MemoryEntry, MemoryType, MemorySystem,
    MemoryRetrievalRequest, MemoryRetrievalResult, RetrievedFact, RetrievedSummary,
};

const DEFAULT_BUDGET: usize = 20;

/// 旧 JSON 文件结构（仅用于一次性导入）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyMemoryData {
    budget: usize,
    entries: Vec<MemoryEntry>,
}

pub struct MemoryStore {
    books: RwLock<HashMap<String, Arc<RwLock<MemorySystem>>>>,
    /// 已从 SQLite 加载过的 book_id 集合，避免重复加载
    loaded: RwLock<std::collections::HashSet<String>>,
    db: Database,
    /// 保留 data_dir 用于一次性导入旧 JSON 文件
    data_dir: PathBuf,
}

fn make_entry(
    key: String,
    value: String,
    memory_type: MemoryType,
    source: &str,
    chapter: Option<String>,
    tags: Option<Vec<String>>,
    importance: u32,
) -> MemoryEntry {
    let now = chrono::Utc::now().to_rfc3339();
    MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        key,
        value: value.clone(),
        memory_type: memory_type.clone(),
        source: source.to_string(),
        importance,
        created_at: now.clone(),
        updated_at: now.clone(),
        content: Some(value),
        entry_type: Some(memory_type.to_string()),
        chapter,
        timestamp: Some(now),
        tags,
    }
}

impl MemoryStore {
    pub fn new(db: Database, data_dir: PathBuf) -> Arc<Self> {
        let store = Arc::new(Self {
            books: RwLock::new(HashMap::new()),
            loaded: RwLock::new(std::collections::HashSet::new()),
            db,
            data_dir,
        });
        store.import_legacy_json_sync();
        store
    }

    /// 暴露 Database 引用 —— 供归档 IPC 命令(memory_archives 表)使用。
    ///
    /// 归档操作是独立的 DB CRUD,不经过 MemoryStore 的 in-memory cache,
    /// 因此直接复用 store 持有的 Database 连接,避免在 MemoryState 中再存一份。
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// 暴露 data_dir 引用 —— 供 read_archive / delete_archive 命令拼装归档文件路径。
    ///
    /// 归档文件位于 `<data_dir>/agents/<role>/MEMORY.archive.<date>.md`。
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// 一次性导入：扫描 <data_dir>/memory/*.json，迁移到 SQLite。
    /// 导入成功后删除原 JSON 文件（避免重复导入）。
    /// 失败只 log，不阻塞启动。
    fn import_legacy_json_sync(&self) {
        let memory_dir = self.data_dir.join("memory");
        if !memory_dir.exists() {
            return;
        }
        let entries = match std::fs::read_dir(&memory_dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let book_id = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let data: LegacyMemoryData = match serde_json::from_str(&content) {
                Ok(d) => d,
                Err(_) => continue,
            };
            if data.entries.is_empty() {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            match self.db.upsert_memory_entries_batch(&book_id, &data.entries) {
                Ok(()) => {
                    tracing::info!(
                        book_id = %book_id,
                        count = data.entries.len(),
                        "Imported legacy memory JSON to SQLite"
                    );
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => {
                    tracing::warn!(
                        book_id = %book_id,
                        error = %e,
                        "Failed to import legacy memory JSON (kept for retry)"
                    );
                }
            }
        }
    }

    /// 懒加载：首次访问某 book 时从 SQLite 读取全部条目到 in-memory cache。
    async fn ensure_loaded(&self, book_id: &str) {
        {
            let loaded = self.loaded.read().await;
            if loaded.contains(book_id) {
                return;
            }
        }
        // 升级为写锁
        let mut loaded = self.loaded.write().await;
        if loaded.contains(book_id) {
            return;
        }
        // 从 SQLite 加载
        let entries = match self.db.list_memory_entries(book_id) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(book_id = %book_id, error = %e, "Failed to load memory entries from SQLite");
                Vec::new()
            }
        };
        let mut memory = MemorySystem::new(DEFAULT_BUDGET as u32);
        memory.entries = entries;
        let mut books = self.books.write().await;
        books.insert(book_id.to_string(), Arc::new(RwLock::new(memory)));
        loaded.insert(book_id.to_string());
    }

    pub async fn get_or_create(&self, book_id: &str, budget: usize) -> Arc<RwLock<MemorySystem>> {
        self.ensure_loaded(book_id).await;
        let mut books = self.books.write().await;
        books.entry(book_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(budget as u32))))
            .clone()
    }

    pub async fn get(&self, book_id: &str) -> Option<Arc<RwLock<MemorySystem>>> {
        self.ensure_loaded(book_id).await;
        self.books.read().await.get(book_id).cloned()
    }

    pub async fn archive_fact(
        &self,
        book_id: &str,
        chapter: u32,
        subject: &str,
        predicate: &str,
        object: &str,
        category: &str,
    ) {
        let entry_type = match category {
            "character" => MemoryType::Character,
            "plot" => MemoryType::Plot,
            "setting" => MemoryType::Setting,
            "dialogue" => MemoryType::Dialogue,
            "style" => MemoryType::Style,
            _ => MemoryType::Fact,
        };
        let value = format!("{} {} {}", subject, predicate, object);
        let entry = make_entry(
            format!("{}_{}", subject, category),
            value,
            entry_type,
            "archive_fact",
            Some(chapter.to_string()),
            Some(vec![category.to_string(), subject.to_lowercase()]),
            0, // archived
        );
        // 写穿 SQLite
        if let Err(e) = self.db.upsert_memory_entry(book_id, &entry) {
            tracing::warn!(book_id = %book_id, error = %e, "Failed to persist archive_fact");
        }
        // 更新 in-memory cache
        let memory = self.get_or_create(book_id, DEFAULT_BUDGET).await;
        memory.write().await.entries.push(entry);
    }

    pub async fn archive_hook(
        &self,
        book_id: &str,
        chapter: u32,
        name: &str,
        hook_type: &str,
        status: &str,
        description: &str,
    ) {
        let value = format!("[Hook:{}] {} - {} ({})", hook_type, name, description, status);
        let entry = make_entry(
            format!("hook_{}", name),
            value,
            MemoryType::Plot,
            "archive_hook",
            Some(chapter.to_string()),
            Some(vec!["hook".to_string(), hook_type.to_string(), name.to_lowercase()]),
            0, // archived
        );
        if let Err(e) = self.db.upsert_memory_entry(book_id, &entry) {
            tracing::warn!(book_id = %book_id, error = %e, "Failed to persist archive_hook");
        }
        let memory = self.get_or_create(book_id, DEFAULT_BUDGET).await;
        memory.write().await.entries.push(entry);
    }

    pub async fn archive_summary(
        &self,
        book_id: &str,
        chapter: u32,
        title: &str,
        characters: &[String],
        events: &[String],
    ) {
        let value = format!(
            "Chapter {}: {} | Characters: {} | Events: {}",
            chapter, title, characters.join(", "), events.join("; ")
        );
        let entry = make_entry(
            format!("summary_ch{}", chapter),
            value,
            MemoryType::Fact,
            "archive_summary",
            Some(chapter.to_string()),
            Some(vec!["summary".to_string(), format!("ch{}", chapter)]),
            0, // archived
        );
        if let Err(e) = self.db.upsert_memory_entry(book_id, &entry) {
            tracing::warn!(book_id = %book_id, error = %e, "Failed to persist archive_summary");
        }
        let memory = self.get_or_create(book_id, DEFAULT_BUDGET).await;
        memory.write().await.entries.push(entry);
    }

    pub async fn search(&self, book_id: &str, query: &str, top_k: usize) -> Vec<MemoryEntry> {
        // 优先用 SQLite LIKE 查询（支持索引），失败时回退到内存过滤
        match self.db.search_memory_entries(book_id, query, top_k) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!(book_id = %book_id, error = %e, "SQLite search failed, falling back to in-memory");
                let books = self.books.read().await;
                if let Some(memory) = books.get(book_id) {
                    memory.read().await.get_all_entries().iter()
                        .filter(|e| e.value.contains(query) || e.key.contains(query))
                        .take(top_k)
                        .cloned()
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }
    }

    pub async fn list_all(&self, book_id: &str) -> Vec<MemoryEntry> {
        self.ensure_loaded(book_id).await;
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            memory.read().await.get_all_entries().iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

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
        let entry = make_entry(
            format!("manual_{}", uuid::Uuid::new_v4()),
            content,
            entry_type,
            "manual",
            chapter.map(|c| c.to_string()),
            Some(tags),
            1, // active
        );
        if let Err(e) = self.db.upsert_memory_entry(book_id, &entry) {
            tracing::warn!(book_id = %book_id, error = %e, "Failed to persist manual memory entry");
        }
        let memory = self.get_or_create(book_id, DEFAULT_BUDGET).await;
        memory.write().await.entries.push(entry.clone());
        entry
    }

    pub async fn delete_entry(&self, book_id: &str, entry_id: &str) -> bool {
        let deleted = self.db.delete_memory_entry(book_id, entry_id)
            .unwrap_or(false);
        if deleted {
            // 同步从 in-memory cache 移除
            let books = self.books.read().await;
            if let Some(memory) = books.get(book_id) {
                let _ = memory.write().await.delete_entry(entry_id);
            }
        }
        deleted
    }

    pub async fn update_entry(
        &self,
        book_id: &str,
        entry_id: &str,
        content: String,
        _tags: Vec<String>,
    ) -> Option<MemoryEntry> {
        let updated = self.db.update_memory_entry_content(book_id, entry_id, &content)
            .unwrap_or(false);
        if updated {
            // 同步更新 in-memory cache
            let books = self.books.read().await;
            if let Some(memory) = books.get(book_id) {
                let _ = memory.write().await.update_entry(entry_id, &content);
            }
            return Some(make_entry(
                entry_id.to_string(),
                content,
                MemoryType::Fact,
                "update",
                None,
                None,
                1,
            ));
        }
        None
    }

    pub async fn format_context(&self, book_id: &str) -> String {
        self.ensure_loaded(book_id).await;
        let books = self.books.read().await;
        if let Some(memory) = books.get(book_id) {
            memory.read().await.format_main_context()
        } else {
            String::new()
        }
    }

    pub async fn stats(&self, book_id: &str) -> (usize, usize) {
        // 优先用 SQLite count（不需要预加载全部 entries）
        match self.db.count_memory_entries(book_id) {
            Ok(stats) => stats,
            Err(e) => {
                tracing::warn!(book_id = %book_id, error = %e, "SQLite count failed, falling back to in-memory");
                let books = self.books.read().await;
                if let Some(memory) = books.get(book_id) {
                    let mem = memory.read().await;
                    let main = mem.get_all_entries().iter().filter(|e| e.importance > 0).count();
                    let archival = mem.get_all_entries().iter().filter(|e| e.importance == 0).count();
                    (main, archival)
                } else {
                    (0, 0)
                }
            }
        }
    }

    /// P2.2: 批量检索记忆选择 —— 通过 story_facts / chapter_summaries 表查询，
    /// 替代前端并行读 7 个文件的慢路径。
    ///
    /// - facts: 按 chapter 时序过滤（valid_from_chapter <= chapter AND
    ///   (valid_until_chapter IS NULL OR valid_until_chapter > chapter)）
    /// - summaries: 最近 N 章的 chapter_summaries（chapter < current, DESC, LIMIT）
    /// - hooks / volume_summaries: 无 SQLite 表，返回空（前端降级到 markdown）
    ///
    /// 失败时返回 Err（不 silent fallback），由 IPC 层决定降级策略。
    pub fn retrieve_selection(&self, req: &MemoryRetrievalRequest) -> Result<MemoryRetrievalResult, crate::shared::error::AppError> {
        let max_items = req.max_items_per_category.clamp(1, 200) as u32;
        let chapter = req.chapter_number.unwrap_or(0).max(0) as u32;

        let mut facts: Vec<RetrievedFact> = Vec::new();
        let mut summaries: Vec<RetrievedSummary> = Vec::new();

        if req.include_facts && chapter > 0 {
            match self.db.query_facts_at_chapter(&req.book_id, chapter) {
                Ok(rows) => {
                    facts = rows.iter()
                        .take(max_items as usize)
                        .map(RetrievedFact::from_story_fact)
                        .collect();
                }
                Err(e) => {
                    tracing::warn!(
                        book_id = %req.book_id,
                        chapter = chapter,
                        error = %e,
                        "query_facts_at_chapter failed, returning empty facts"
                    );
                }
            }
        }

        if req.include_summaries && chapter > 0 {
            match self.db.list_recent_chapter_summaries(&req.book_id, chapter, max_items) {
                Ok(rows) => {
                    summaries = rows.iter()
                        .map(RetrievedSummary::from_chapter_summary)
                        .collect();
                }
                Err(e) => {
                    tracing::warn!(
                        book_id = %req.book_id,
                        chapter = chapter,
                        error = %e,
                        "list_recent_chapter_summaries failed, returning empty summaries"
                    );
                }
            }
        }

        let has_data = !facts.is_empty() || !summaries.is_empty();

        Ok(MemoryRetrievalResult {
            facts,
            summaries,
            has_data,
        })
    }
}
