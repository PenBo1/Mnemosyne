//! ═══════════════════════════════════════════════════════════════════════════
//! 会话存储 - 交互会话 SQLite 持久化
//! ═══════════════════════════════════════════════════════════════════════════

use rusqlite::params;

use crate::domain::interaction::types::{AutomationMode, InteractionSession, SessionKind};
use crate::infrastructure::db::connection::Database;
use crate::shared::error::AppError;

/// rusqlite::Error → AppError 映射（与 connection::db_err 等价，因 db_err 为 pub(super) 不可跨模块访问）
fn db_err(e: rusqlite::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}

/// InteractionSession 在 SQLite 中的轻量索引行（不含 messages/events payload）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionSessionRow {
    pub session_id: String,
    pub session_kind: String,
    pub automation_mode: String,
    pub active_book_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 内部：序列化 InteractionSession 为 payload_json
fn serialize_session(session: &InteractionSession) -> Result<String, AppError> {
    serde_json::to_string(session).map_err(|e| {
        AppError::internal(format!("InteractionSession 序列化失败: {}", e))
    })
}

/// 内部：从 payload_json 反序列化 InteractionSession
fn deserialize_session(payload: &str) -> Result<InteractionSession, AppError> {
    serde_json::from_str(payload).map_err(|e| {
        AppError::internal(format!("InteractionSession 反序列化失败: {}", e))
    })
}

/// 内部：派生 title（取首条 user message 前 50 字符，无则空字符串）
fn derive_title(session: &InteractionSession) -> String {
    session
        .messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| {
            let trimmed = m.content.trim();
            let len = trimmed.chars().count().min(50);
            trimmed.chars().take(len).collect::<String>()
        })
        .unwrap_or_default()
}

impl Database {
    /// 加载 InteractionSession（含完整 payload）。
    pub fn load_interaction_session(&self, session_id: &str) -> Result<InteractionSession, AppError> {
        let conn = self.conn()?;
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM interaction_sessions WHERE id = ?",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::not_found(format!("interaction_session: {}", session_id))
                }
                other => db_err(other),
            })?;
        deserialize_session(&payload)
    }

    /// Upsert InteractionSession（INSERT OR REPLACE）。
    pub fn persist_interaction_session(
        &self,
        session: &InteractionSession,
    ) -> Result<(), AppError> {
        let payload = serialize_session(session)?;
        let title = derive_title(session);
        let session_kind = session.session_kind.as_str();
        let automation_mode = session.automation_mode.as_str();
        let active_book_id = session.active_book_id.as_deref();
        let created_at = &session.created_at;
        let updated_at = &session.updated_at;

        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO interaction_sessions \
             (id, session_kind, automation_mode, active_book_id, title, payload_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                &session.session_id,
                session_kind,
                automation_mode,
                active_book_id,
                &title,
                &payload,
                created_at,
                updated_at,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    /// 列出 InteractionSession 索引行（不含 payload）。
    /// book_id 为 None 时列出全部，按 updated_at 倒序。
    pub fn list_interaction_sessions(
        &self,
        book_id: Option<&str>,
    ) -> Result<Vec<InteractionSessionRow>, AppError> {
        let start = std::time::Instant::now();
        tracing::debug!(book_id = ?book_id, "[InteractionSessionStore] Listing sessions");
        
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT id, session_kind, automation_mode, active_book_id, title, created_at, updated_at \
             FROM interaction_sessions",
        );
        let mut params_vec: Vec<String> = Vec::new();
        if let Some(bid) = book_id {
            sql.push_str(" WHERE active_book_id = ?");
            params_vec.push(bid.to_string());
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                Ok(InteractionSessionRow {
                    session_id: row.get(0)?,
                    session_kind: row.get(1)?,
                    automation_mode: row.get(2)?,
                    active_book_id: row.get(3)?,
                    title: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(db_err)?;
        let result = rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;
        
        tracing::debug!(
            count = result.len(),
            duration_ms = start.elapsed().as_millis() as u64,
            "[InteractionSessionStore] Listed sessions"
        );
        Ok(result)
    }

    /// 加载 InteractionSession 完整列表（含 payload，用于 batch 拉取）。
    pub fn list_full_interaction_sessions(
        &self,
        book_id: Option<&str>,
    ) -> Result<Vec<InteractionSession>, AppError> {
        let conn = self.conn()?;
        let mut sql = String::from("SELECT payload_json FROM interaction_sessions");
        let mut params_vec: Vec<String> = Vec::new();
        if let Some(bid) = book_id {
            sql.push_str(" WHERE active_book_id = ?");
            params_vec.push(bid.to_string());
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                let payload: String = row.get(0)?;
                Ok(payload)
            })
            .map_err(db_err)?;

        let mut out = Vec::new();
        for row in rows {
            let payload = row.map_err(db_err)?;
            out.push(deserialize_session(&payload)?);
        }
        Ok(out)
    }

    /// 删除 InteractionSession。返回是否实际删除了一行。
    pub fn delete_interaction_session(&self, session_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn
            .execute(
                "DELETE FROM interaction_sessions WHERE id = ?",
                params![session_id],
            )
            .map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 更新 automation_mode（轻量更新，不重写 payload）。
    /// 注意：调用方需在更新后同步 session.automation_mode 字段。
    pub fn update_interaction_session_automation_mode(
        &self,
        session_id: &str,
        mode: AutomationMode,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE interaction_sessions SET automation_mode = ?, updated_at = ? WHERE id = ?",
                params![mode.as_str(), &now, session_id],
            )
            .map_err(db_err)?;
        Ok(affected > 0)
    }
}

/// 创建新的 InteractionSession（生成 UUID v4 作为 session_id）。
/// 用于 interaction_run_request 当 session_id 为空时自动创建。
pub fn create_new_interaction_session(session_kind: SessionKind) -> InteractionSession {
    let session_id = uuid::Uuid::new_v4().to_string();
    InteractionSession::new(session_id, session_kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::interaction::types::{InteractionMessage, SessionKind};

    #[test]
    fn derive_title_from_first_user_message() {
        let mut session = InteractionSession::new("test-1", SessionKind::Chat);
        session.append_message(InteractionMessage {
            role: "assistant".to_string(),
            content: "hi".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        });
        session.append_message(InteractionMessage {
            role: "user".to_string(),
            content: "请帮我写一本关于时间旅行的小说".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        });
        let title = derive_title(&session);
        assert!(title.contains("时间旅行"));
    }

    #[test]
    fn derive_title_empty_when_no_user_message() {
        let session = InteractionSession::new("test-2", SessionKind::Chat);
        assert_eq!(derive_title(&session), "");
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut session = InteractionSession::new("test-3", SessionKind::Book);
        session.bind_active_book("book-1");
        session.append_message(InteractionMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        });
        let json = serialize_session(&session).unwrap();
        let restored = deserialize_session(&json).unwrap();
        assert_eq!(restored.session_id, "test-3");
        assert_eq!(restored.session_kind, SessionKind::Book);
        assert_eq!(restored.active_book_id, Some("book-1".to_string()));
        assert_eq!(restored.messages.len(), 1);
    }
}
