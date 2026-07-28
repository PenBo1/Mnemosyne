//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆存储 - Agent 通用记忆持久化
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 替代 <data_dir>/memory/<book_id>.json 文件持久化。
//! MemoryStore 仍是内存缓存 + 业务逻辑层，本模块提供 SQLite 持久化原语。

use rusqlite::params;
use chrono::Utc;

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use crate::infrastructure::memory::types::{MemoryEntry, MemoryType};

// ── SQL 语句 ────────────────────────────────────────────────────────────────

const MEMORY_ENTRY_UPSERT_SQL: &str = "\
INSERT INTO memory_entries (\
    id, book_id, key, value, memory_type, source, importance,\
    content, entry_type, chapter, timestamp, tags_json, created_at, updated_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)\
ON CONFLICT(book_id, id) DO UPDATE SET\
    key = excluded.key,\
    value = excluded.value,\
    memory_type = excluded.memory_type,\
    source = excluded.source,\
    importance = excluded.importance,\
    content = excluded.content,\
    entry_type = excluded.entry_type,\
    chapter = excluded.chapter,\
    timestamp = excluded.timestamp,\
    tags_json = excluded.tags_json,\
    updated_at = excluded.updated_at";

const MEMORY_ENTRY_SELECT_COLUMNS: &str = "\
id, key, value, memory_type, source, importance,\
content, entry_type, chapter, timestamp, tags_json, created_at, updated_at";

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 解析记忆类型字符串
fn parse_memory_type(s: &str) -> MemoryType {
    match s {
        "fact" => MemoryType::Fact,
        "context" => MemoryType::Context,
        "preference" => MemoryType::Preference,
        "lesson" => MemoryType::Lesson,
        "conversation" => MemoryType::Conversation,
        "character" => MemoryType::Character,
        "plot" => MemoryType::Plot,
        "setting" => MemoryType::Setting,
        "dialogue" => MemoryType::Dialogue,
        "style" => MemoryType::Style,
        "research" => MemoryType::Research,
        _ => MemoryType::General,
    }
}

/// 映射数据库行到记忆条目
fn map_memory_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
    let tags_json: String = row.get(10)?;
    let tags: Vec<String> = if tags_json.is_empty() || tags_json == "[]" {
        Vec::new()
    } else {
        serde_json::from_str(&tags_json).unwrap_or_default()
    };
    Ok(MemoryEntry {
        id: row.get(0)?,
        key: row.get(1)?,
        value: row.get(2)?,
        memory_type: parse_memory_type(&row.get::<_, String>(3)?),
        source: row.get(4)?,
        importance: row.get::<_, i64>(5)? as u32,
        content: row.get(6)?,
        entry_type: row.get(7)?,
        chapter: row.get(8)?,
        timestamp: row.get(9)?,
        tags: if tags.is_empty() { None } else { Some(tags) },
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 写穿单条记忆条目
    pub fn upsert_memory_entry(&self, book_id: &str, entry: &MemoryEntry) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(entry.tags.as_deref().unwrap_or(&[]))
            .map_err(|e| AppError::internal(format!("Failed to serialize tags: {}", e)))?;
        let memory_type_str = entry.memory_type.to_string();
        let conn = self.conn()?;
        conn.execute(
            MEMORY_ENTRY_UPSERT_SQL,
            params![
                &entry.id, book_id, &entry.key, &entry.value,
                &memory_type_str, &entry.source, entry.importance as i64,
                &entry.content, &entry.entry_type, &entry.chapter, &entry.timestamp,
                &tags_json, &entry.created_at, &now,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 列出某书籍的全部记忆条目
    pub fn list_memory_entries(&self, book_id: &str) -> Result<Vec<MemoryEntry>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM memory_entries WHERE book_id = ? \
                 ORDER BY importance DESC, created_at ASC",
                MEMORY_ENTRY_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map([book_id], map_memory_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 模糊搜索记忆条目
    pub fn search_memory_entries(
        &self,
        book_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryEntry>, AppError> {
        let conn = self.conn()?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM memory_entries \
                 WHERE book_id = ? AND (key LIKE ? OR value LIKE ?) \
                 ORDER BY importance DESC, created_at ASC \
                 LIMIT ?",
                MEMORY_ENTRY_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map(
            params![book_id, &pattern, &pattern, top_k as i64],
            map_memory_row,
        ).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 删除单条记忆
    pub fn delete_memory_entry(&self, book_id: &str, entry_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM memory_entries WHERE book_id = ? AND id = ?",
            params![book_id, entry_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 更新条目内容
    pub fn update_memory_entry_content(
        &self,
        book_id: &str,
        entry_id: &str,
        content: &str,
    ) -> Result<bool, AppError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE memory_entries \
             SET value = ?, content = ?, updated_at = ? \
             WHERE book_id = ? AND id = ?",
            params![content, content, &now, book_id, entry_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 归档单条记忆
    pub fn archive_memory_entry(&self, book_id: &str, entry_id: &str) -> Result<bool, AppError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE memory_entries SET importance = 0, updated_at = ? \
             WHERE book_id = ? AND id = ?",
            params![&now, book_id, entry_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 统计记忆条目
    pub fn count_memory_entries(&self, book_id: &str) -> Result<(usize, usize), AppError> {
        let conn = self.conn()?;
        let row = conn.query_row(
            "SELECT \
                SUM(CASE WHEN importance > 0 THEN 1 ELSE 0 END) AS main_count, \
                SUM(CASE WHEN importance = 0 THEN 1 ELSE 0 END) AS archival_count \
             FROM memory_entries WHERE book_id = ?",
            params![book_id],
            |row| {
                let main: Option<i64> = row.get(0)?;
                let archival: Option<i64> = row.get(1)?;
                Ok((main.unwrap_or(0) as usize, archival.unwrap_or(0) as usize))
            },
        ).map_err(db_err)?;
        Ok(row)
    }

    /// 批量写穿记忆条目
    pub fn upsert_memory_entries_batch(
        &self,
        book_id: &str,
        entries: &[MemoryEntry],
    ) -> Result<(), AppError> {
        if entries.is_empty() {
            return Ok(());
        }
        let now = Utc::now().to_rfc3339();
        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;
        for entry in entries {
            let tags_json = serde_json::to_string(entry.tags.as_deref().unwrap_or(&[]))
                .map_err(|e| AppError::internal(format!("Failed to serialize tags: {}", e)))?;
            let memory_type_str = entry.memory_type.to_string();
            tx.execute(
                MEMORY_ENTRY_UPSERT_SQL,
                params![
                    &entry.id, book_id, &entry.key, &entry.value,
                    &memory_type_str, &entry.source, entry.importance as i64,
                    &entry.content, &entry.entry_type, &entry.chapter, &entry.timestamp,
                    &tags_json, &entry.created_at, &now,
                ],
            ).map_err(db_err)?;
        }
        tx.commit().map_err(db_err)?;
        Ok(())
    }
}