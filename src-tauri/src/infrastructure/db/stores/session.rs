//! ═══════════════════════════════════════════════════════════════════════════
//! 会话存储 - Session 与 Message 管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 核心能力：
//! - Session CRUD：创建、查询、更新、删除（级联删除子 session）
//! - Session 分裂：从父 session 派生新 session（Branch/Compression/Delegate）
//! - Message CRUD：创建消息（含 FTS5 全文索引）
//! - 消息全文搜索：基于 FTS5 trigram 支持中文子串匹配

use std::collections::HashSet;

use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 创建会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    /// 小说 ID
    pub novel_id: Option<String>,
    /// 工作区 ID
    pub workspace_id: Option<String>,
    /// 标题
    pub title: Option<String>,
}

/// Session 分裂类型
///
/// - Branch: 分支，从父 session 派生一条新对话线
/// - Compression: 压缩，将父 session 上下文压缩后派生新 session
/// - Delegate: 委托，将任务委托给子 session 处理
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionSplitType {
    /// 分支
    Branch,
    /// 压缩
    Compression,
    /// 委托
    Delegate,
}

impl SessionSplitType {
    /// 转为数据库存储字符串
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Branch => "branch",
            Self::Compression => "compression",
            Self::Delegate => "delegate",
        }
    }

    /// 从数据库字符串解析
    pub fn from_db_str(s: &str) -> Result<Self, AppError> {
        match s {
            "branch" => Ok(Self::Branch),
            "compression" => Ok(Self::Compression),
            "delegate" => Ok(Self::Delegate),
            other => Err(AppError::internal(format!(
                "Invalid split_type in DB: {}",
                other
            ))),
        }
    }
}

/// 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话 ID
    pub id: String,
    /// 小说 ID
    pub novel_id: Option<String>,
    /// 工作区 ID
    pub workspace_id: Option<String>,
    /// 会话类型
    pub session_type: String,
    /// 标题
    pub title: String,
    /// 摘要
    pub summary: Option<String>,
    /// 消息数
    pub message_count: u32,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 成本
    pub cost: f64,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 父会话 ID
    pub parent_session_id: Option<String>,
    /// 分裂类型
    pub split_type: Option<SessionSplitType>,
    /// 分裂原因
    pub split_reason: Option<String>,
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息 ID
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
    /// 工具调用 JSON
    pub tool_calls: Option<String>,
    /// 工具结果 JSON
    pub tool_results: Option<String>,
    /// token 数
    pub token_count: Option<u32>,
    /// 思考内容
    pub thinking_content: Option<String>,
    /// 模型
    pub model: Option<String>,
    /// 提供商
    pub provider: Option<String>,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 延迟毫秒
    pub latency_ms: Option<u64>,
    /// 创建时间
    pub created_at: String,
}

/// 消息元数据
#[derive(Debug, Clone)]
pub struct MessageMeta<'a> {
    /// 思考内容
    pub thinking_content: Option<&'a str>,
    /// 模型
    pub model: Option<&'a str>,
    /// 提供商
    pub provider: Option<&'a str>,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 延迟毫秒
    pub latency_ms: Option<u64>,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射消息行
fn map_message_row(row: &rusqlite::Row) -> rusqlite::Result<Message> {
    let token_count: Option<i64> = row.get(6)?;
    let input_tokens: i64 = row.get(10)?;
    let output_tokens: i64 = row.get(11)?;
    let latency_ms: Option<i64> = row.get(12)?;
    Ok(Message {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        tool_calls: row.get(4)?,
        tool_results: row.get(5)?,
        token_count: token_count.map(|v| v as u32),
        thinking_content: row.get(7)?,
        model: row.get(8)?,
        provider: row.get(9)?,
        input_tokens: input_tokens as u32,
        output_tokens: output_tokens as u32,
        latency_ms: latency_ms.map(|v| v as u64),
        created_at: row.get(13)?,
    })
}

/// 映射会话行
fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    let message_count: i64 = row.get(6)?;
    let input_tokens: i64 = row.get(7)?;
    let output_tokens: i64 = row.get(8)?;
    let split_type_str: Option<String> = row.get(14)?;
    let split_type = match split_type_str {
        Some(s) => Some(SessionSplitType::from_db_str(&s).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                14,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?),
        None => None,
    };
    Ok(Session {
        id: row.get(0)?,
        novel_id: row.get(1)?,
        workspace_id: row.get(2)?,
        session_type: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        message_count: message_count as u32,
        input_tokens: input_tokens as u32,
        output_tokens: output_tokens as u32,
        cost: row.get(9)?,
        status: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        parent_session_id: row.get(13)?,
        split_type,
        split_reason: row.get(15)?,
    })
}

const MESSAGE_COLUMNS: &str = "id, session_id, role, content, tool_calls, tool_results, token_count, thinking_content, model, provider, input_tokens, output_tokens, latency_ms, created_at";

// ── Session 操作 ────────────────────────────────────────────────────────────

impl Database {
    /// 创建会话
    pub fn create_session(&self, req: CreateSessionRequest) -> Result<Session, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = req.title.unwrap_or_default();
        let novel_id = req.novel_id.clone();
        let workspace_id = req.workspace_id.clone();

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO sessions (id, novel_id, workspace_id, session_type, title, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at) VALUES (?, ?, ?, 'chat', ?, 0, 0, 0, 0.0, 'active', ?, ?)",
            params![&id, &novel_id, &workspace_id, &title, &now, &now],
        ).map_err(db_err)?;

        Ok(Session {
            id,
            novel_id,
            workspace_id,
            session_type: "chat".to_string(),
            title,
            summary: None,
            message_count: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
            status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
            parent_session_id: None,
            split_type: None,
            split_reason: None,
        })
    }

    /// 获取会话
    pub fn get_session(&self, id: &str) -> Result<Option<Session>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, workspace_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at, parent_session_id, split_type, split_reason FROM sessions WHERE id = ?",
            params![id],
            map_session_row,
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    /// 列出会话
    pub fn list_sessions(&self, novel_id: Option<&str>, workspace_id: Option<&str>) -> Result<Vec<Session>, AppError> {
        let conn = self.conn()?;
        let mut sql = String::from("SELECT id, novel_id, workspace_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at, parent_session_id, split_type, split_reason FROM sessions");
        let mut conditions: Vec<&str> = Vec::new();
        let mut params_vec: Vec<String> = Vec::new();
        if let Some(nid) = novel_id {
            conditions.push("novel_id = ?");
            params_vec.push(nid.to_string());
        }
        if let Some(wid) = workspace_id {
            conditions.push("workspace_id = ?");
            params_vec.push(wid.to_string());
        }
        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), map_session_row).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// 更新会话
    pub fn update_session(&self, session: &Session) -> Result<(), AppError> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE sessions SET title = ?, summary = ?, message_count = ?, input_tokens = ?, output_tokens = ?, cost = ?, status = ?, updated_at = ? WHERE id = ?",
            params![
                &session.title,
                &session.summary,
                session.message_count as i64,
                session.input_tokens as i64,
                session.output_tokens as i64,
                session.cost,
                &session.status,
                &session.updated_at,
                &session.id,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 创建会话分裂
    ///
    /// 从父 session 派生子 session，继承 parent 的 workspace_id。
    /// 子 session 的 novel_id 为 None（分裂是新对话上下文，不继承 novel 绑定）。
    pub fn create_session_split(
        &self,
        parent_id: &str,
        split_type: SessionSplitType,
        reason: Option<&str>,
    ) -> Result<Session, AppError> {
        let parent = self.get_session(parent_id)?.ok_or_else(AppError::session_not_found)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let workspace_id = parent.workspace_id.clone();
        let split_type_str = split_type.as_db_str();

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO sessions (id, novel_id, workspace_id, session_type, title, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at, parent_session_id, split_type, split_reason) VALUES (?, NULL, ?, 'chat', '', 0, 0, 0, 0.0, 'active', ?, ?, ?, ?, ?)",
            params![&id, &workspace_id, &now, &now, parent_id, split_type_str, reason],
        ).map_err(db_err)?;

        Ok(Session {
            id,
            novel_id: None,
            workspace_id,
            session_type: "chat".to_string(),
            title: String::new(),
            summary: None,
            message_count: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
            status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
            parent_session_id: Some(parent_id.to_string()),
            split_type: Some(split_type),
            split_reason: reason.map(|s| s.to_string()),
        })
    }

    /// 删除会话（级联）
    ///
    /// 递归删除所有后代 session 及其消息。
    /// 使用 BFS 收集所有后代 ID，在同一事务内执行，保证原子性。
    pub fn delete_session(&self, id: &str) -> Result<bool, AppError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;

        // BFS 收集所有后代 session ID（含自身）
        let mut all_ids: Vec<String> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = vec![id.to_string()];

        {
            let mut stmt = tx
                .prepare("SELECT id FROM sessions WHERE parent_session_id = ?")
                .map_err(db_err)?;
            while !frontier.is_empty() {
                let mut next_frontier: Vec<String> = Vec::new();
                for sid in &frontier {
                    if !visited.insert(sid.clone()) {
                        return Err(AppError::internal(
                            "Cycle detected in session parent_session_id chain",
                        ));
                    }
                    all_ids.push(sid.clone());

                    let rows = stmt
                        .query_map(params![sid], |row| row.get::<_, String>(0))
                        .map_err(db_err)?;
                    for r in rows {
                        next_frontier.push(r.map_err(db_err)?);
                    }
                }
                frontier = next_frontier;
            }
        }

        // 删除所有相关 messages
        for sid in &all_ids {
            tx.execute("DELETE FROM messages WHERE session_id = ?", params![sid])
                .map_err(db_err)?;
        }

        // 删除所有相关 sessions
        let mut deleted = false;
        for sid in &all_ids {
            let affected = tx
                .execute("DELETE FROM sessions WHERE id = ?", params![sid])
                .map_err(db_err)?;
            if sid == id && affected > 0 {
                deleted = true;
            }
        }

        tx.commit().map_err(db_err)?;
        Ok(deleted)
    }

    // ── Message 操作 ────────────────────────────────────────────────────────

    /// 创建消息
    pub fn create_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        tool_results: Option<&str>,
    ) -> Result<Message, AppError> {
        self.create_message_with_meta(session_id, role, content, tool_calls, tool_results, None)
    }

    /// 创建消息（带元数据）
    pub fn create_message_with_meta(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        tool_results: Option<&str>,
        meta: Option<MessageMeta<'_>>,
    ) -> Result<Message, AppError> {
        let valid_roles = ["user", "assistant", "system", "tool"];
        if !valid_roles.contains(&role) {
            return Err(AppError::invalid_input(format!("Invalid message role: {}", role)));
        }
        if content.len() > 1_000_000 {
            return Err(AppError::invalid_input("Message content too long (max 1MB)"));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let (thinking, model, provider, input_tokens, output_tokens, latency_ms) = match &meta {
            Some(m) => (
                m.thinking_content,
                m.model,
                m.provider,
                m.input_tokens as i64,
                m.output_tokens as i64,
                m.latency_ms.map(|v| v as i64),
            ),
            None => (None, None, None, 0, 0, None),
        };

        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;
        tx.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, thinking_content, model, provider, input_tokens, output_tokens, latency_ms, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &id, session_id, role, content, tool_calls, tool_results,
                thinking, model, provider, input_tokens, output_tokens, latency_ms, &now,
            ],
        ).map_err(db_err)?;
        tx.execute(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ? WHERE id = ?",
            params![&now, session_id],
        ).map_err(db_err)?;
        tx.commit().map_err(db_err)?;

        Ok(Message {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: tool_calls.map(|s| s.to_string()),
            tool_results: tool_results.map(|s| s.to_string()),
            token_count: None,
            thinking_content: thinking.map(|s| s.to_string()),
            model: model.map(|s| s.to_string()),
            provider: provider.map(|s| s.to_string()),
            input_tokens: input_tokens as u32,
            output_tokens: output_tokens as u32,
            latency_ms: latency_ms.map(|v| v as u64),
            created_at: now,
        })
    }

    /// 列出消息
    pub fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, AppError> {
        let sql = format!(
            "SELECT {} FROM messages WHERE session_id = ? ORDER BY created_at ASC",
            MESSAGE_COLUMNS
        );
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt.query_map([session_id], map_message_row).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// 全文搜索消息
    ///
    /// 基于 FTS5 trigram tokenizer，支持中文子串匹配。
    /// 按 bm25 相关性排序。
    pub fn search_messages(
        &self,
        session_id: Option<&str>,
        query: &str,
        limit: u32,
    ) -> Result<Vec<MessageSearchResult>, AppError> {
        // trigram tokenizer 要求查询串至少 3 个字符
        if query.chars().count() < 3 {
            return Ok(Vec::new());
        }
        let escaped = escape_fts5_query(query);
        let limit_i64 = limit as i64;

        let conn = self.conn()?;
        let sql = if session_id.is_some() {
            "SELECT m.id, m.session_id, m.role, m.content, \
                    snippet(messages_fts, 0, '<mark>', '</mark>', '...', 10) AS snippet, \
                    bm25(messages_fts) AS score \
             FROM messages_fts f \
             JOIN messages m ON m.rowid = f.rowid \
             WHERE messages_fts MATCH ? AND m.session_id = ? \
             ORDER BY bm25(messages_fts) \
             LIMIT ?"
        } else {
            "SELECT m.id, m.session_id, m.role, m.content, \
                    snippet(messages_fts, 0, '<mark>', '</mark>', '...', 10) AS snippet, \
                    bm25(messages_fts) AS score \
             FROM messages_fts f \
             JOIN messages m ON m.rowid = f.rowid \
             WHERE messages_fts MATCH ? \
             ORDER BY bm25(messages_fts) \
             LIMIT ?"
        };
        let mut stmt = conn.prepare_cached(sql).map_err(db_err)?;
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<MessageSearchResult> {
            Ok(MessageSearchResult {
                message_id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                snippet: row.get(4)?,
                score: row.get(5)?,
            })
        };
        let rows = if let Some(sid) = session_id {
            stmt.query_map(params![&escaped, sid, limit_i64], map_row).map_err(db_err)?
        } else {
            stmt.query_map(params![&escaped, limit_i64], map_row).map_err(db_err)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }
}

// ── 搜索结果 ────────────────────────────────────────────────────────────────

/// 消息搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSearchResult {
    /// 消息 ID
    pub message_id: String,
    /// 会话 ID
    pub session_id: String,
    /// 角色
    pub role: String,
    /// 完整内容
    pub content: String,
    /// 命中片段
    pub snippet: String,
    /// bm25 相关性得分
    pub score: f64,
}

/// 转义 FTS5 查询串
///
/// 移除双引号，整体用双引号包裹为 phrase 查询。
fn escape_fts5_query(query: &str) -> String {
    let cleaned: String = query.chars().filter(|c| *c != '"').collect();
    format!("\"{}\"", cleaned)
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_message(
        db: &Database,
        role: &str,
        content: &str,
    ) -> Result<(String, String), AppError> {
        let session = db.create_session(CreateSessionRequest {
            novel_id: None,
            workspace_id: None,
            title: Some("test".to_string()),
        })?;
        let msg = db.create_message(&session.id, role, content, None, None)?;
        Ok((session.id, msg.id))
    }

    #[test]
    fn fts5_table_created_in_memory() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let conn = db.conn().expect("conn");
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='messages_fts')",
                [],
                |r| r.get(0),
            )
            .expect("query fts table");
        assert!(exists, "messages_fts 表应存在");
        for trig in ["messages_fts_ai", "messages_fts_ad", "messages_fts_au"] {
            let count: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name='{}'", trig),
                    [],
                    |r| r.get(0),
                )
                .expect("query trigger");
            assert_eq!(count, 1, "触发器 {} 应存在", trig);
        }
    }

    #[test]
    fn search_after_insert_returns_match() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let (sid, mid) = seed_message(&db, "user", "The quick brown fox jumps over the lazy dog").expect("seed");
        let results = db.search_messages(None, "brown fox", 10).expect("search");
        assert_eq!(results.len(), 1, "应命中 1 条");
        assert_eq!(results[0].message_id, mid);
        assert_eq!(results[0].session_id, sid);
        assert!(results[0].snippet.contains("<mark>"), "snippet 应包含 <mark> 标记");
    }

    #[test]
    fn search_chinese_substring() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let (_, _) = seed_message(&db, "assistant", "今天天气真好，我们去公园散步吧").expect("seed");
        let results = db.search_messages(None, "天气真好", 10).expect("search");
        assert_eq!(results.len(), 1, "中文子串应命中");
        assert!(results[0].snippet.contains("<mark>"));
    }

    #[test]
    fn search_short_query_returns_empty() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let _ = seed_message(&db, "user", "abcdefgh").expect("seed");
        let results = db.search_messages(None, "ab", 10).expect("search");
        assert!(results.is_empty(), "短于 3 字符应返回空");
    }

    #[test]
    fn search_after_delete_synced() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let (sid, _mid) = seed_message(&db, "user", "unique searchable phrase here").expect("seed");
        let deleted = db.delete_session(&sid).expect("delete session");
        assert!(deleted, "会话应已删除");
        let results = db.search_messages(None, "searchable phrase", 10).expect("search");
        assert!(results.is_empty(), "删除后搜索应为空");
    }

    #[test]
    fn search_filter_by_session_id() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let (sid1, _) = seed_message(&db, "user", "shared keyword content here").expect("seed 1");
        let (sid2, _) = seed_message(&db, "user", "shared keyword content here").expect("seed 2");
        assert_ne!(sid1, sid2, "两个会话 ID 应不同");
        let results = db.search_messages(Some(&sid1), "keyword content", 10).expect("search");
        assert_eq!(results.len(), 1, "限定 session_id 应只返回 1 条");
        assert_eq!(results[0].session_id, sid1);
        let all = db.search_messages(None, "keyword content", 10).expect("search all");
        assert_eq!(all.len(), 2, "不限定应返回 2 条");
    }

    #[test]
    fn search_escapes_special_chars() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let _ = seed_message(&db, "user", "price: $42 *special* \"quoted\" text").expect("seed");
        let escaped = escape_fts5_query(r#"a"b"#);
        assert_eq!(escaped, r#""ab""#);
        let results = db.search_messages(None, "*special*", 10).expect("search");
        assert_eq!(results.len(), 1, "特殊字符查询应命中字面内容");
    }

    #[test]
    fn search_bm25_ordering() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let _ = seed_message(&db, "user", "hello world hello world hello world").expect("seed 1");
        let _ = seed_message(&db, "user", "hello there").expect("seed 2");
        let results = db.search_messages(None, "hello world", 10).expect("search");
        assert_eq!(results.len(), 1, "phrase 查询应只命中连续出现的文档");
        assert!(results[0].score < 0.0, "bm25 应返回负值");
    }

    // Session Split 测试

    fn make_root_session(db: &Database, workspace_id: &str) -> Session {
        db.create_session(CreateSessionRequest {
            novel_id: None,
            workspace_id: Some(workspace_id.to_string()),
            title: Some("root".to_string()),
        })
        .expect("create root session")
    }

    #[test]
    fn split_type_enum_db_roundtrip() {
        assert_eq!(SessionSplitType::Branch.as_db_str(), "branch");
        assert_eq!(SessionSplitType::Compression.as_db_str(), "compression");
        assert_eq!(SessionSplitType::Delegate.as_db_str(), "delegate");

        assert_eq!(
            SessionSplitType::from_db_str("branch").unwrap(),
            SessionSplitType::Branch
        );
        assert_eq!(
            SessionSplitType::from_db_str("compression").unwrap(),
            SessionSplitType::Compression
        );
        assert_eq!(
            SessionSplitType::from_db_str("delegate").unwrap(),
            SessionSplitType::Delegate
        );

        assert!(SessionSplitType::from_db_str("invalid").is_err());
    }

    #[test]
    fn split_type_enum_serde_snake_case() {
        let json = serde_json::to_string(&SessionSplitType::Compression).unwrap();
        assert_eq!(json, "\"compression\"");

        let parsed: SessionSplitType = serde_json::from_str("\"delegate\"").unwrap();
        assert_eq!(parsed, SessionSplitType::Delegate);
    }

    #[test]
    fn create_split_sets_parent_and_inherits_workspace() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let parent = make_root_session(&db, "ws-1");

        let child = db
            .create_session_split(&parent.id, SessionSplitType::Branch, Some("test reason"))
            .expect("create split");

        assert_eq!(child.parent_session_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(child.split_type, Some(SessionSplitType::Branch));
        assert_eq!(child.split_reason.as_deref(), Some("test reason"));
        assert_eq!(child.workspace_id.as_deref(), Some("ws-1"));
        assert!(child.novel_id.is_none());

        let fetched = db.get_session(&child.id).unwrap().expect("child should exist");
        assert_eq!(fetched.parent_session_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(fetched.split_type, Some(SessionSplitType::Branch));
        assert_eq!(fetched.split_reason.as_deref(), Some("test reason"));
        assert_eq!(fetched.workspace_id.as_deref(), Some("ws-1"));
    }

    #[test]
    fn create_split_nonexistent_parent_returns_error() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let result =
            db.create_session_split("nonexistent-id", SessionSplitType::Delegate, None);
        assert!(result.is_err(), "不存在的 parent 应返回错误");
    }

    #[test]
    fn create_split_all_three_types() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let parent = make_root_session(&db, "ws-2");

        let branch = db
            .create_session_split(&parent.id, SessionSplitType::Branch, None)
            .expect("branch");
        let compression = db
            .create_session_split(&parent.id, SessionSplitType::Compression, None)
            .expect("compression");
        let delegate = db
            .create_session_split(&parent.id, SessionSplitType::Delegate, None)
            .expect("delegate");

        assert_eq!(branch.split_type, Some(SessionSplitType::Branch));
        assert_eq!(compression.split_type, Some(SessionSplitType::Compression));
        assert_eq!(delegate.split_type, Some(SessionSplitType::Delegate));

        assert_eq!(branch.parent_session_id, delegate.parent_session_id);
        assert_eq!(compression.parent_session_id, delegate.parent_session_id);
    }

    #[test]
    fn cascade_delete_removes_all_descendants() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let a = make_root_session(&db, "ws-cascade");
        let b = db
            .create_session_split(&a.id, SessionSplitType::Branch, None)
            .expect("create B");
        let c = db
            .create_session_split(&b.id, SessionSplitType::Compression, None)
            .expect("create C");

        db.create_message(&a.id, "user", "msg in A", None, None)
            .expect("msg A");
        db.create_message(&c.id, "user", "msg in C", None, None)
            .expect("msg C");

        let deleted = db.delete_session(&a.id).expect("delete A");
        assert!(deleted, "A 应被删除");

        assert!(db.get_session(&a.id).unwrap().is_none(), "A 应已删除");
        assert!(db.get_session(&b.id).unwrap().is_none(), "B 应被级联删除");
        assert!(db.get_session(&c.id).unwrap().is_none(), "C 应被级联删除");

        let msgs = db.list_messages(&c.id).expect("list C messages");
        assert!(msgs.is_empty(), "C 的消息应被级联删除");
    }

    #[test]
    fn cascade_delete_multi_branch_tree() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let a = make_root_session(&db, "ws-tree");
        let b = db
            .create_session_split(&a.id, SessionSplitType::Branch, None)
            .expect("B");
        let c = db
            .create_session_split(&a.id, SessionSplitType::Delegate, None)
            .expect("C");
        let d = db
            .create_session_split(&b.id, SessionSplitType::Compression, None)
            .expect("D");

        let deleted = db.delete_session(&a.id).expect("delete A");
        assert!(deleted);

        assert!(db.get_session(&a.id).unwrap().is_none());
        assert!(db.get_session(&b.id).unwrap().is_none());
        assert!(db.get_session(&c.id).unwrap().is_none());
        assert!(db.get_session(&d.id).unwrap().is_none());
    }

    #[test]
    fn delete_leaf_does_not_affect_parent() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let parent = make_root_session(&db, "ws-leaf");
        let child1 = db
            .create_session_split(&parent.id, SessionSplitType::Branch, None)
            .expect("child1");
        let child2 = db
            .create_session_split(&parent.id, SessionSplitType::Delegate, None)
            .expect("child2");

        let deleted = db.delete_session(&child1.id).expect("delete child1");
        assert!(deleted);

        assert!(db.get_session(&child1.id).unwrap().is_none(), "child1 应已删除");
        assert!(db.get_session(&parent.id).unwrap().is_some(), "parent 应保留");
        assert!(db.get_session(&child2.id).unwrap().is_some(), "child2 应保留");
    }

    #[test]
    fn delete_nonexistent_session_returns_false() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let deleted = db.delete_session("nonexistent").expect("delete call");
        assert!(!deleted, "不存在的 session 应返回 false");
    }

    #[test]
    fn root_session_has_null_split_fields() {
        let db = Database::connect_in_memory().expect("in-memory db");
        let session = make_root_session(&db, "ws-root");

        assert!(session.parent_session_id.is_none());
        assert!(session.split_type.is_none());
        assert!(session.split_reason.is_none());

        let fetched = db.get_session(&session.id).unwrap().expect("session exists");
        assert!(fetched.parent_session_id.is_none());
        assert!(fetched.split_type.is_none());
        assert!(fetched.split_reason.is_none());
    }
}