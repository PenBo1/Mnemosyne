//! ═══════════════════════════════════════════════════════════════════════════
//! 回忆记忆存储 - Agent 历史对话检索
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 用于 Agent 检索历史对话记录，支持按 session/role/时间范围搜索。
//! 与 memory_short_term 的区别：
//! - memory_short_term 存储 session 摘要（LLM 生成的总结）
//! - recall_memory 存储原始对话消息（user/assistant 交互记录）

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── SQL 语句 ────────────────────────────────────────────────────────────────

const UPSERT_SQL: &str = "\
INSERT INTO recall_memory (\
    id, session_id, message_index, role, content, created_at\
) VALUES (?, ?, ?, ?, ?, ?) \
ON CONFLICT(session_id, message_index) DO UPDATE SET \
    role = excluded.role, \
    content = excluded.content, \
    created_at = excluded.created_at";

const SELECT_COLUMNS: &str = "\
id, session_id, message_index, role, content, created_at";

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 回忆记忆行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallMemoryRow {
    /// 记录 ID
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 消息索引
    pub message_index: i64,
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
    /// 创建时间
    pub created_at: String,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到回忆记忆行
fn map_row(row: &rusqlite::Row) -> rusqlite::Result<RecallMemoryRow> {
    Ok(RecallMemoryRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        message_index: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 插入或更新回忆记忆
    pub fn upsert_recall_memory(&self, row: &RecallMemoryRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        conn.execute(
            UPSERT_SQL,
            params![
                &row.id,
                &row.session_id,
                row.message_index,
                &row.role,
                &row.content,
                &row.created_at,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 搜索回忆记忆
    pub fn search_recall_memory(
        &self,
        session_id: Option<&str>,
        role: Option<&str>,
        query: Option<&str>,
        limit: i64,
    ) -> Result<Vec<RecallMemoryRow>, AppError> {
        let limit = limit.clamp(1, 500);
        let conn = self.conn()?;

        let mut sql = format!(
            "SELECT {} FROM recall_memory WHERE 1=1",
            SELECT_COLUMNS
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(sid) = session_id {
            sql.push_str(" AND session_id = ?");
            params_vec.push(Box::new(sid.to_string()));
        }

        if let Some(r) = role {
            sql.push_str(" AND role = ?");
            params_vec.push(Box::new(r.to_string()));
        }

        if let Some(q) = query {
            if !q.trim().is_empty() {
                sql.push_str(" AND content LIKE ?");
                let pattern = format!("%{}%", q.trim());
                params_vec.push(Box::new(pattern));
            }
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        params_vec.push(Box::new(limit));

        let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt.query_map(params.as_slice(), map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出会话的回忆记忆
    pub fn list_recall_by_session(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<RecallMemoryRow>, AppError> {
        self.search_recall_memory(Some(session_id), None, None, limit)
    }

    /// 删除会话的回忆记忆
    pub fn delete_recall_by_session(&self, session_id: &str) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM recall_memory WHERE session_id = ?",
            params![session_id],
        ).map_err(db_err)?;
        Ok(affected as u64)
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    fn make_row(id: &str, session_id: &str, idx: i64, role: &str, content: &str) -> RecallMemoryRow {
        RecallMemoryRow {
            id: id.to_string(),
            session_id: session_id.to_string(),
            message_index: idx,
            role: role.to_string(),
            content: content.to_string(),
            created_at: "2026-07-14T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn upsert_inserts_then_updates() {
        let db = make_in_memory_db();
        let row1 = make_row("id-1", "sess-1", 0, "user", "Hello");
        db.upsert_recall_memory(&row1).unwrap();

        let fetched = db.list_recall_by_session("sess-1", 10).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].content, "Hello");

        let mut row2 = row1.clone();
        row2.content = "Hello updated".to_string();
        db.upsert_recall_memory(&row2).unwrap();

        let fetched = db.list_recall_by_session("sess-1", 10).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].content, "Hello updated");
    }

    #[test]
    fn search_by_role_filters_correctly() {
        let db = make_in_memory_db();
        db.upsert_recall_memory(&make_row("id-1", "sess-1", 0, "user", "Q1")).unwrap();
        db.upsert_recall_memory(&make_row("id-2", "sess-1", 1, "assistant", "A1")).unwrap();

        let user_msgs = db.search_recall_memory(None, Some("user"), None, 10).unwrap();
        assert_eq!(user_msgs.len(), 1);

        let asst_msgs = db.search_recall_memory(None, Some("assistant"), None, 10).unwrap();
        assert_eq!(asst_msgs.len(), 1);
    }

    #[test]
    fn search_by_query_matches_content() {
        let db = make_in_memory_db();
        db.upsert_recall_memory(&make_row("id-1", "sess-1", 0, "user", "What is the plot?")).unwrap();
        db.upsert_recall_memory(&make_row("id-2", "sess-1", 1, "assistant", "The plot involves...")).unwrap();

        let hits = db.search_recall_memory(None, None, Some("plot"), 10).unwrap();
        assert_eq!(hits.len(), 2);

        let no_hits = db.search_recall_memory(None, None, Some("nonexistent"), 10).unwrap();
        assert!(no_hits.is_empty());
    }

    #[test]
    fn delete_by_session_removes_rows() {
        let db = make_in_memory_db();
        db.upsert_recall_memory(&make_row("id-1", "sess-1", 0, "user", "Q1")).unwrap();
        db.upsert_recall_memory(&make_row("id-2", "sess-2", 0, "user", "Q2")).unwrap();

        let deleted = db.delete_recall_by_session("sess-1").unwrap();
        assert_eq!(deleted, 1);

        let remaining = db.list_recall_by_session("sess-2", 10).unwrap();
        assert_eq!(remaining.len(), 1);
    }
}