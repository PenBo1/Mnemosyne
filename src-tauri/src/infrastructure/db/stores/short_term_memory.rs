// 短期记忆 CRUD —— memory_short_term 表。
//
// 用途:
// - 每次 session 结束(commit)时,AgentEngine 调用 LLM 生成摘要并 upsert 到本表
// - 每晚 GC 任务调用 list_by_date_range() 合并到 MEMORY.md(长期记忆)
// - 用户可在 UI 中按日期浏览短期记忆(回溯能力)
//
// 架构约束:
// - infrastructure 层只依赖 shared/,不依赖 core/agent/
// - 因此定义 ShortTermMemoryRow DTO,agent_role 用 String
// - 业务层在 core/agent 中调用 store 并转换(类比 loop_run)

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const UPSERT_SQL: &str = "\
INSERT INTO memory_short_term (\
    id, session_id, book_id, entry_date, summary, key_topics, \
    agent_role, token_count, message_count, created_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
ON CONFLICT(session_id, entry_date) DO UPDATE SET \
    summary = excluded.summary, \
    key_topics = excluded.key_topics, \
    token_count = excluded.token_count, \
    message_count = excluded.message_count, \
    created_at = excluded.created_at";

const SELECT_COLUMNS: &str = "\
id, session_id, book_id, entry_date, summary, key_topics, \
agent_role, token_count, message_count, created_at";

/// 短期记忆行(对应 memory_short_term 表)
///
/// key_topics 为 JSON 字符串(数组),由业务层用 serde_json::to_string 序列化。
/// 这样保持 infrastructure 层不依赖业务类型,只处理原始字符串。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryRow {
    pub id: String,
    pub session_id: String,
    pub book_id: Option<String>,
    /// YYYY-MM-DD
    pub entry_date: String,
    pub summary: String,
    /// JSON 数组字符串,如 `["chapter-3","dialogue"]`
    pub key_topics: String,
    pub agent_role: Option<String>,
    pub token_count: u64,
    pub message_count: u32,
    pub created_at: String,
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ShortTermMemoryRow> {
    Ok(ShortTermMemoryRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        book_id: row.get(2)?,
        entry_date: row.get(3)?,
        summary: row.get(4)?,
        key_topics: row.get(5)?,
        agent_role: row.get(6)?,
        token_count: row.get::<_, i64>(7)? as u64,
        message_count: row.get::<_, i64>(8)? as u32,
        created_at: row.get(9)?,
    })
}

impl Database {
    /// 插入或更新短期记忆。
    ///
    /// UNIQUE(session_id, entry_date) 约束保证同 session 同一天只保留最新摘要。
    /// 当天 session 持续追加消息时,每次 commit 会覆盖当天记录。
    pub fn upsert_short_term_memory(&self, row: &ShortTermMemoryRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        let book_id = row.book_id.as_deref();
        let agent_role = row.agent_role.as_deref();
        conn.execute(
            UPSERT_SQL,
            params![
                &row.id,
                &row.session_id,
                book_id,
                &row.entry_date,
                &row.summary,
                &row.key_topics,
                agent_role,
                row.token_count as i64,
                row.message_count as i64,
                &row.created_at,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 按 session_id 获取最新摘要(若有)
    ///
    /// 用于 session 恢复时回填"上次进展"上下文。
    pub fn get_short_term_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ShortTermMemoryRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_short_term WHERE session_id = ?1 \
             ORDER BY entry_date DESC LIMIT 1",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let mut rows = stmt.query_map([session_id], map_row).map_err(db_err)?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(db_err)?))
        } else {
            Ok(None)
        }
    }

    /// 按日期获取所有 session 摘要(用于每日回顾 UI)
    ///
    /// entry_date 格式:YYYY-MM-DD
    pub fn list_short_term_by_date(
        &self,
        entry_date: &str,
    ) -> Result<Vec<ShortTermMemoryRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_short_term WHERE entry_date = ?1 \
             ORDER BY created_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([entry_date], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按日期范围列出(用于每晚合并短期→长期)
    ///
    /// start_date / end_date 格式:YYYY-MM-DD(闭区间)
    pub fn list_short_term_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<ShortTermMemoryRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_short_term \
             WHERE entry_date >= ?1 AND entry_date <= ?2 \
             ORDER BY entry_date ASC, created_at ASC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([start_date, end_date], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按 book_id 获取该书的 session 摘要(用于小说创作的上下文回顾)
    pub fn list_short_term_by_book(
        &self,
        book_id: &str,
        limit: i64,
    ) -> Result<Vec<ShortTermMemoryRow>, AppError> {
        let limit = limit.clamp(1, 500);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_short_term WHERE book_id = ?1 \
             ORDER BY entry_date DESC LIMIT ?2",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map(params![book_id, limit], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// GC:删除 created_at 早于 cutoff_iso 的短期记忆(30 天滚动保留)
    pub fn delete_short_term_before(&self, cutoff_iso: &str) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM memory_short_term WHERE created_at < ?1",
            params![cutoff_iso],
        ).map_err(db_err)?;
        Ok(affected as u64)
    }

    /// 统计 memory_short_term 表总行数(用于 stats 命令的 total 字段)。
    ///
    /// SQLite COUNT(*) 走索引,性能足够(表本身有 30 天 GC 收缩)。
    pub fn count_all_short_term(&self) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_short_term", [], |row| row.get(0))
            .map_err(db_err)?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    fn make_row(id: &str, session_id: &str, date: &str, summary: &str) -> ShortTermMemoryRow {
        ShortTermMemoryRow {
            id: id.to_string(),
            session_id: session_id.to_string(),
            book_id: Some("book-1".to_string()),
            entry_date: date.to_string(),
            summary: summary.to_string(),
            key_topics: r#"["topic-a","topic-b"]"#.to_string(),
            agent_role: Some("main".to_string()),
            token_count: 1500,
            message_count: 8,
            created_at: format!("{}T12:00:00Z", date),
        }
    }

    #[test]
    fn upsert_inserts_then_updates() {
        let db = make_in_memory_db();
        let row1 = make_row("id-1", "sess-1", "2026-07-13", "first summary");
        db.upsert_short_term_memory(&row1).unwrap();

        let fetched = db.get_short_term_for_session("sess-1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().summary, "first summary");

        // 同 session 同天再次 upsert → 覆盖
        let mut row2 = row1.clone();
        row2.id = "id-2".to_string();
        row2.summary = "updated summary".to_string();
        db.upsert_short_term_memory(&row2).unwrap();

        let fetched = db.get_short_term_for_session("sess-1").unwrap();
        assert_eq!(fetched.unwrap().summary, "updated summary");

        // 应该只有 1 条记录(UPSERT 覆盖)
        let all = db.list_short_term_by_date("2026-07-13").unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn list_by_date_returns_all_sessions() {
        let db = make_in_memory_db();
        db.upsert_short_term_memory(&make_row("id-1", "sess-1", "2026-07-13", "session 1")).unwrap();
        db.upsert_short_term_memory(&make_row("id-2", "sess-2", "2026-07-13", "session 2")).unwrap();
        db.upsert_short_term_memory(&make_row("id-3", "sess-3", "2026-07-14", "session 3")).unwrap();

        let daily = db.list_short_term_by_date("2026-07-13").unwrap();
        assert_eq!(daily.len(), 2);
        assert!(daily.iter().any(|r| r.session_id == "sess-1"));
        assert!(daily.iter().any(|r| r.session_id == "sess-2"));

        let next_day = db.list_short_term_by_date("2026-07-14").unwrap();
        assert_eq!(next_day.len(), 1);
    }

    #[test]
    fn list_by_date_range_inclusive() {
        let db = make_in_memory_db();
        db.upsert_short_term_memory(&make_row("id-1", "sess-1", "2026-07-10", "d1")).unwrap();
        db.upsert_short_term_memory(&make_row("id-2", "sess-2", "2026-07-13", "d2")).unwrap();
        db.upsert_short_term_memory(&make_row("id-3", "sess-3", "2026-07-15", "d3")).unwrap();

        let range = db.list_short_term_by_date_range("2026-07-10", "2026-07-13").unwrap();
        assert_eq!(range.len(), 2); // 7-10 和 7-13 都在区间
    }

    #[test]
    fn list_by_book_returns_recent() {
        let db = make_in_memory_db();
        db.upsert_short_term_memory(&make_row("id-1", "sess-1", "2026-07-13", "s1")).unwrap();
        db.upsert_short_term_memory(&make_row("id-2", "sess-2", "2026-07-14", "s2")).unwrap();
        db.upsert_short_term_memory(&make_row("id-3", "sess-3", "2026-07-15", "s3")).unwrap();

        let book_entries = db.list_short_term_by_book("book-1", 10).unwrap();
        assert_eq!(book_entries.len(), 3);
        // 最新在前
        assert_eq!(book_entries[0].entry_date, "2026-07-15");
    }

    #[test]
    fn delete_before_gcs_old() {
        let db = make_in_memory_db();
        db.upsert_short_term_memory(&make_row("id-1", "sess-1", "2026-06-01", "old")).unwrap();
        db.upsert_short_term_memory(&make_row("id-2", "sess-2", "2026-07-13", "new")).unwrap();

        let deleted = db.delete_short_term_before("2026-07-01T00:00:00Z").unwrap();
        assert_eq!(deleted, 1);

        let remaining = db.list_short_term_by_date_range("2026-01-01", "2026-12-31").unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].entry_date, "2026-07-13");
    }
}
