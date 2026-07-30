//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆存储 - Agent 通用记忆持久化
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::memory::types::{MemoryEntry, MemoryType, MemorySystem};
use crate::infrastructure::memory::dto::{
    MemoryRetrievalRequest, MemoryRetrievalResult, RetrievedFact, RetrievedSummary,
};

const DEFAULT_BUDGET: usize = 20;
/// in-memory cache 中最多保留多少个 book 的记忆条目；超出后淘汰最久未访问的 book。
/// book 数据本身持久化在 SQLite，淘汰后下次访问会通过 ensure_loaded() 重新加载。
const MAX_BOOKS_IN_CACHE: usize = 32;

pub struct MemoryStore {
    books: RwLock<HashMap<String, Arc<RwLock<MemorySystem>>>>,
    /// 已从 SQLite 加载过的 book_id 集合，避免重复加载
    loaded: RwLock<std::collections::HashSet<String>>,
    /// 每个 book 上次被访问的时间戳，用于 LRU 淘汰
    last_access: RwLock<HashMap<String, Instant>>,
    db: Database,
    /// 保留 data_dir 供归档命令拼装归档文件路径
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
        
        Arc::new(Self {
            books: RwLock::new(HashMap::new()),
            loaded: RwLock::new(std::collections::HashSet::new()),
            last_access: RwLock::new(HashMap::new()),
            db,
            data_dir,
        })
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

    /// 懒加载：首次访问某 book 时从 SQLite 读取全部条目到 in-memory cache。
    /// 同时执行 LRU 淘汰：当 books 数量超过 MAX_BOOKS_IN_CACHE 时，
    /// 移除最久未访问的 book（含其 books/loaded/last_access 三处记录）。
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
        let mut last_access = self.last_access.write().await;
        // LRU 淘汰：超过上限时移除最久未访问的 book
        if books.len() >= MAX_BOOKS_IN_CACHE {
            // 找出当前 books 中最久未访问的 book_id。
            // 缺失 last_access 的 book 视为"极旧"（24h 前），避免新加入但 last_access
            // 缺失的 book 被当作"最新永不淘汰"（原 bug 用 Instant::now() 作为
            // 回退值，反而把它当作最新）。
            let stale_fallback = Instant::now() - std::time::Duration::from_secs(86400);
            if let Some(evict_id) = books
                .keys()
                .min_by_key(|bid| last_access.get(*bid).copied().unwrap_or(stale_fallback))
                .cloned()
            {
                books.remove(&evict_id);
                loaded.remove(&evict_id);
                last_access.remove(&evict_id);
                tracing::debug!(book_id = %evict_id, "Evicted memory cache entry (LRU)");
            }
        }
        books.insert(book_id.to_string(), Arc::new(RwLock::new(memory)));
        // 补写 last_access（即使 or_insert_with 路径也保证有记录）
        last_access.insert(book_id.to_string(), Instant::now());
        loaded.insert(book_id.to_string());
    }

    pub async fn get_or_create(&self, book_id: &str, budget: usize) -> Arc<RwLock<MemorySystem>> {
        self.ensure_loaded(book_id).await;
        let arc = {
            let mut books = self.books.write().await;
            books
                .entry(book_id.to_string())
                .or_insert_with(|| Arc::new(RwLock::new(MemorySystem::new(budget as u32))))
                .clone()
        };
        // 在 books 锁释放后再写 last_access，避免双重持锁跨 await
        self.last_access.write().await.insert(book_id.to_string(), Instant::now());
        arc
    }

    pub async fn get(&self, book_id: &str) -> Option<Arc<RwLock<MemorySystem>>> {
        self.ensure_loaded(book_id).await;
        let arc = self.books.read().await.get(book_id).cloned();
        if arc.is_some() {
            self.last_access.write().await.insert(book_id.to_string(), Instant::now());
        }
        arc
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
        let start_time = Instant::now();
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
            tracing::error!(
                book_id = %book_id,
                chapter = chapter,
                subject = %subject,
                error = %e,
                "[memory] archive_fact failed"
            );
        }
        // 更新 in-memory cache
        let memory = self.get_or_create(book_id, DEFAULT_BUDGET).await;
        memory.write().await.entries.push(entry);
        
        let duration_ms = start_time.elapsed().as_millis() as u64;
        tracing::info!(
            book_id = %book_id,
            chapter = chapter,
            subject = %subject,
            category = %category,
            duration_ms = duration_ms,
            "[memory] fact archived"
        );
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
            memory.read().await.get_all_entries().to_vec()
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

    pub async fn delete_entry(&self, book_id: &str, entry_id: &str) -> Result<bool, crate::shared::error::AppError> {
        // DB 调用失败直接 ? 传播（不静默吞错为 false）。
        let deleted = self.db.delete_memory_entry(book_id, entry_id)?;
        if deleted {
            // 同步从 in-memory cache 移除。先 clone Arc 出来再释放 books 读锁，
            // 避免外层 RwLock 读守卫跨 await。
            let memory_arc = {
                let books = self.books.read().await;
                books.get(book_id).cloned()
            };
            if let Some(memory) = memory_arc {
                let _ = memory.write().await.delete_entry(entry_id);
            }
        }
        Ok(deleted)
    }

    pub async fn update_entry(
        &self,
        book_id: &str,
        entry_id: &str,
        content: String,
        _tags: Vec<String>,
    ) -> Result<Option<MemoryEntry>, crate::shared::error::AppError> {
        // DB 调用失败直接 ? 传播（不静默吞错为 false）。
        let updated = self.db.update_memory_entry_content(book_id, entry_id, &content)?;
        if !updated {
            return Ok(None);
        }

        // 同步更新 in-memory cache 并返回更新后的 entry。
        // 关键：保留原 entry 的 id/type/source/chapter/tags/created_at/timestamp，
        // 仅替换 value/content/updated_at —— 不用 make_entry 重建避免丢失元数据。
        let memory_arc = {
            let books = self.books.read().await;
            books.get(book_id).cloned()
        };
        let mut updated_entry: Option<MemoryEntry> = None;
        if let Some(memory) = memory_arc {
            let mut mem = memory.write().await;
            // 找到原 entry 并 in-place 修改，保留所有非 value 字段
            if let Some(entry) = mem.entries.iter_mut().find(|e| e.id == entry_id) {
                entry.value = content.clone();
                entry.content = Some(content.clone());
                entry.updated_at = chrono::Utc::now().to_rfc3339();
                updated_entry = Some(entry.clone());
            }
        }

        // 如果 cache 中没找到（book 未加载或 entry 不在 cache），从 DB 读一次返回。
        Ok(match updated_entry {
            Some(e) => Some(e),
            None => {
                // fallback：仅用 DB 已知字段构造返回值。
                // 由于 update_memory_entry_content 只更新 value/content/updated_at，
                // 其他字段必须由调用方或后续 retrieve 命令从 DB 读取。
                let now = chrono::Utc::now().to_rfc3339();
                Some(MemoryEntry {
                    id: entry_id.to_string(),
                    key: String::new(),
                    value: content,
                    memory_type: MemoryType::General,
                    source: String::new(),
                    importance: 0,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    content: None,
                    entry_type: None,
                    chapter: None,
                    timestamp: None,
                    tags: None,
                })
            }
        })
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
    /// 失败策略（对齐 "no silent fallback"）：DB 查询失败时直接返回 Err，
    /// 由 IPC 层决定降级策略（前端可以转而读 markdown 文件，但不能让本命令
    /// 静默返回空数据导致用户误以为无数据）。
    pub fn retrieve_selection(&self, req: &MemoryRetrievalRequest) -> Result<MemoryRetrievalResult, crate::shared::error::AppError> {
        let max_items = req.max_items_per_category.clamp(1, 200) as u32;
        let chapter = req.chapter_number.unwrap_or(0).max(0) as u32;

        let mut facts: Vec<RetrievedFact> = Vec::new();
        let mut summaries: Vec<RetrievedSummary> = Vec::new();

        if req.include_facts && chapter > 0 {
            let rows = self.db.query_facts_at_chapter(&req.book_id, chapter)?;
            facts = rows.iter()
                .take(max_items as usize)
                .map(RetrievedFact::from_story_fact)
                .collect();
        }

        if req.include_summaries && chapter > 0 {
            let rows = self.db.list_recent_chapter_summaries(&req.book_id, chapter, max_items)?;
            summaries = rows.iter()
                .map(RetrievedSummary::from_chapter_summary)
                .collect();
        }

        let has_data = !facts.is_empty() || !summaries.is_empty();

        Ok(MemoryRetrievalResult {
            facts,
            summaries,
            has_data,
        })
    }
}
