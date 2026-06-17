use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::Database;
use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub novel_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub novel_id: Option<String>,
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
    pub created_at: String,
}

impl Database {
    pub fn create_session(&self, req: CreateSessionRequest) -> Result<Session, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = req.title.unwrap_or_default();
        let novel_id = req.novel_id;

        self.conn.execute(
            "INSERT INTO sessions (id, novel_id, session_type, title, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at) VALUES (?1, ?2, 'chat', ?3, 0, 0, 0, 0.0, 'active', ?4, ?4)",
            params![id, novel_id, title, now],
        ).map_err(|e| AppError::internal(format!("Failed to create session: {}", e)))?;

        Ok(Session {
            id,
            novel_id,
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
        let mut stmt = self.conn.prepare(
            "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query session: {}", e)))?;
        match rows.next().map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(self.row_to_session(row)?)),
            None => Ok(None),
        }
    }

    pub fn list_sessions(&self, novel_id: Option<&str>) -> Result<Vec<Session>, AppError> {
        let mut result = Vec::new();
        if let Some(nid) = novel_id {
            let mut stmt = self.conn.prepare(
                "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions WHERE novel_id = ?1 ORDER BY updated_at DESC"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let rows = stmt.query_map(params![nid], |row| self.row_to_session(row))
                .map_err(|e| AppError::internal(format!("Failed to query sessions: {}", e)))?;
            for row in rows {
                result.push(row.map_err(|e| AppError::internal(format!("Failed to read session: {}", e)))?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions ORDER BY updated_at DESC"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let rows = stmt.query_map([], |row| self.row_to_session(row))
                .map_err(|e| AppError::internal(format!("Failed to query sessions: {}", e)))?;
            for row in rows {
                result.push(row.map_err(|e| AppError::internal(format!("Failed to read session: {}", e)))?);
            }
        }
        Ok(result)
    }

    pub fn update_session(&self, session: &Session) -> Result<(), AppError> {
        self.conn.execute(
            "UPDATE sessions SET title = ?1, summary = ?2, message_count = ?3, input_tokens = ?4, output_tokens = ?5, cost = ?6, status = ?7, updated_at = ?8 WHERE id = ?9",
            params![session.title, session.summary, session.message_count, session.input_tokens, session.output_tokens, session.cost, session.status, session.updated_at, session.id],
        ).map_err(|e| AppError::internal(format!("Failed to update session: {}", e)))?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<bool, AppError> {
        let tx = self.conn.unchecked_transaction()
            .map_err(|e| AppError::internal(format!("Failed to start delete session tx: {}", e)))?;
        tx.execute("DELETE FROM messages WHERE session_id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete messages: {}", e)))?;
        let affected = tx.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete session: {}", e)))?;
        tx.commit()
            .map_err(|e| AppError::internal(format!("Failed to commit delete session: {}", e)))?;
        Ok(affected > 0)
    }

    pub fn create_message(&self, session_id: &str, role: &str, content: &str, tool_calls: Option<&str>, tool_results: Option<&str>) -> Result<Message, AppError> {
        let valid_roles = ["user", "assistant", "system", "tool"];
        if !valid_roles.contains(&role) {
            return Err(AppError::invalid_input(format!("Invalid message role: {}", role)));
        }
        if content.len() > 1_000_000 {
            return Err(AppError::invalid_input("Message content too long (max 1MB)"));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let tx = self.conn.unchecked_transaction()
            .map_err(|e| AppError::internal(format!("Failed to start create message tx: {}", e)))?;

        tx.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, session_id, role, content, tool_calls, tool_results, now],
        ).map_err(|e| AppError::internal(format!("Failed to create message: {}", e)))?;

        tx.execute(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
            params![now, session_id],
        ).map_err(|e| AppError::internal(format!("Failed to update session count: {}", e)))?;

        tx.commit()
            .map_err(|e| AppError::internal(format!("Failed to commit create message: {}", e)))?;

        Ok(Message {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: tool_calls.map(|s| s.to_string()),
            tool_results: tool_results.map(|s| s.to_string()),
            token_count: None,
            created_at: now,
        })
    }

    pub fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, tool_calls, tool_results, token_count, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls: row.get(4)?,
                tool_results: row.get(5)?,
                token_count: row.get(6)?,
                created_at: row.get(7)?,
            })
        }).map_err(|e| AppError::internal(format!("Failed to query messages: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect messages: {}", e)))
    }

    fn row_to_session(&self, row: &rusqlite::Row) -> rusqlite::Result<Session> {
        Ok(Session {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            session_type: row.get(2)?,
            title: row.get(3)?,
            summary: row.get(4)?,
            message_count: row.get(5)?,
            input_tokens: row.get(6)?,
            output_tokens: row.get(7)?,
            cost: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }
}
