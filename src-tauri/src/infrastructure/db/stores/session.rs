
use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub novel_id: Option<String>,
    pub workspace_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub novel_id: Option<String>,
    pub workspace_id: Option<String>,
    pub session_type: String,
    pub title: String,
    pub summary: Option<String>,
    pub message_count: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_results: Option<String>,
    pub token_count: Option<u32>,
    pub thinking_content: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: Option<u64>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct MessageMeta<'a> {
    pub thinking_content: Option<&'a str>,
    pub model: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: Option<u64>,
}

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

fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    let message_count: i64 = row.get(6)?;
    let input_tokens: i64 = row.get(7)?;
    let output_tokens: i64 = row.get(8)?;
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
    })
}

const MESSAGE_COLUMNS: &str = "id, session_id, role, content, tool_calls, tool_results, token_count, thinking_content, model, provider, input_tokens, output_tokens, latency_ms, created_at";

impl Database {
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
        })
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, workspace_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions WHERE id = ?",
            params![id],
            map_session_row,
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn list_sessions(&self, novel_id: Option<&str>, workspace_id: Option<&str>) -> Result<Vec<Session>, AppError> {
        let conn = self.conn()?;
        // 动态拼接 WHERE 条件：支持 novel_id / workspace_id 单独或组合过滤
        let mut sql = String::from("SELECT id, novel_id, workspace_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions");
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

    pub fn delete_session(&self, id: &str) -> Result<bool, AppError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;
        tx.execute("DELETE FROM messages WHERE session_id = ?", params![id]).map_err(db_err)?;
        let affected = tx.execute("DELETE FROM sessions WHERE id = ?", params![id]).map_err(db_err)?;
        tx.commit().map_err(db_err)?;
        Ok(affected > 0)
    }

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
}