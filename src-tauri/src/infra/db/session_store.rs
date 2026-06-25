use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}

impl Database {
    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<Session, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let title = req.title.unwrap_or_default();
        let novel_id = req.novel_id.clone();

        sqlx::query(
            "INSERT INTO sessions (id, novel_id, session_type, title, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at) VALUES (?, ?, 'chat', ?, 0, 0, 0, 0.0, 'active', ?, ?)"
        )
        .bind(&id).bind(&novel_id).bind(&title).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

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

    pub async fn get_session(&self, id: &str) -> Result<Option<Session>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions WHERE id = ?"
        )
        .bind(id)
        .map(|row: sqlx::sqlite::SqliteRow| Session {
            id: row.get(0usize), novel_id: row.get(1usize), session_type: row.get(2usize),
            title: row.get(3usize), summary: row.get(4usize), message_count: row.get(5usize),
            input_tokens: row.get(6usize), output_tokens: row.get(7usize), cost: row.get(8usize),
            status: row.get(9usize), created_at: row.get(10usize), updated_at: row.get(11usize),
        })
        .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn list_sessions(&self, novel_id: Option<&str>) -> Result<Vec<Session>, AppError> {
        let map_session = |row: sqlx::sqlite::SqliteRow| Session {
            id: row.get(0usize), novel_id: row.get(1usize), session_type: row.get(2usize),
            title: row.get(3usize), summary: row.get(4usize), message_count: row.get(5usize),
            input_tokens: row.get(6usize), output_tokens: row.get(7usize), cost: row.get(8usize),
            status: row.get(9usize), created_at: row.get(10usize), updated_at: row.get(11usize),
        };

        if let Some(nid) = novel_id {
            sqlx::query(
                "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions WHERE novel_id = ? ORDER BY updated_at DESC"
            )
            .bind(nid)
            .map(map_session)
            .fetch_all(&self.pool).await.map_err(db_err)
        } else {
            sqlx::query(
                "SELECT id, novel_id, session_type, title, summary, message_count, input_tokens, output_tokens, cost, status, created_at, updated_at FROM sessions ORDER BY updated_at DESC"
            )
            .map(map_session)
            .fetch_all(&self.pool).await.map_err(db_err)
        }
    }

    pub async fn update_session(&self, session: &Session) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sessions SET title = ?, summary = ?, message_count = ?, input_tokens = ?, output_tokens = ?, cost = ?, status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&session.title).bind(&session.summary).bind(session.message_count)
        .bind(session.input_tokens).bind(session.output_tokens).bind(session.cost)
        .bind(&session.status).bind(&session.updated_at).bind(&session.id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    pub async fn delete_session(&self, id: &str) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(id)
            .execute(&mut *tx).await.map_err(db_err)?;
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&mut *tx).await.map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_message(&self, session_id: &str, role: &str, content: &str, tool_calls: Option<&str>, tool_results: Option<&str>) -> Result<Message, AppError> {
        let valid_roles = ["user", "assistant", "system", "tool"];
        if !valid_roles.contains(&role) {
            return Err(AppError::invalid_input(format!("Invalid message role: {}", role)));
        }
        if content.len() > 1_000_000 {
            return Err(AppError::invalid_input("Message content too long (max 1MB)"));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(session_id).bind(role).bind(content).bind(tool_calls).bind(tool_results).bind(&now)
        .execute(&mut *tx).await.map_err(db_err)?;

        sqlx::query(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ? WHERE id = ?"
        )
        .bind(&now).bind(session_id)
        .execute(&mut *tx).await.map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;

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

    pub async fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, AppError> {
        sqlx::query(
            "SELECT id, session_id, role, content, tool_calls, tool_results, token_count, created_at FROM messages WHERE session_id = ? ORDER BY created_at ASC"
        )
        .bind(session_id)
        .map(|row: sqlx::sqlite::SqliteRow| Message {
            id: row.get(0usize), session_id: row.get(1usize), role: row.get(2usize),
            content: row.get(3usize), tool_calls: row.get(4usize), tool_results: row.get(5usize),
            token_count: row.get(6usize), created_at: row.get(7usize),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }
}
