
use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::types::{Prompt, CreatePromptRequest, UpdatePromptRequest};
use super::super::connection::Database;
use super::super::connection::db_err;
use super::super::types::{json_decode, json_encode};
use crate::shared::error::AppError;
use crate::infrastructure::db::connection::validate_name;

impl Database {
    pub fn create_prompt(&self, req: CreatePromptRequest) -> Result<Prompt, AppError> {
        validate_name(&req.name, "Prompt name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags = json_encode(&req.tags, "tags")?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO prompts (id, name, content, category, tags, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![&id, &req.name, &req.content, &req.category, &tags, &now, &now],
        ).map_err(db_err)?;
        Ok(Prompt { id, name: req.name, content: req.content, category: req.category, tags: req.tags, created_at: now.clone(), updated_at: now })
    }

    fn map_prompt_row(row: &rusqlite::Row) -> Result<(String, String, String, String, String, String, String), rusqlite::Error> {
        Ok((
            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
            row.get(4)?, row.get(5)?, row.get(6)?,
        ))
    }

    pub fn list_prompts(&self, category: Option<&str>) -> Result<Vec<Prompt>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(if category.is_some() {
            "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE category = ? ORDER BY updated_at DESC"
        } else {
            "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts ORDER BY updated_at DESC"
        }).map_err(db_err)?;
        let rows = if let Some(cat) = category {
            stmt.query_map([cat], Self::map_prompt_row).map_err(db_err)?
        } else {
            stmt.query_map([], Self::map_prompt_row).map_err(db_err)?
        };
        rows.map(|r| {
            let (id, name, content, category, tags_str, created_at, updated_at) = r.map_err(db_err)?;
            Ok(Prompt {
                id, name, content, category,
                tags: json_decode(&tags_str, "tags")?,
                created_at, updated_at,
            })
        }).collect()
    }

    pub fn get_prompt(&self, id: &str) -> Result<Option<Prompt>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE id = ?",
            params![id],
            Self::map_prompt_row,
        );
        match result {
            Ok((id, name, content, category, tags_str, created_at, updated_at)) => {
                Ok(Some(Prompt {
                    id, name, content, category,
                    tags: json_decode(&tags_str, "tags")?,
                    created_at, updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn update_prompt(&self, req: UpdatePromptRequest) -> Result<Prompt, AppError> {
        let existing = self.get_prompt(&req.id)?
            .ok_or_else(|| AppError::not_found("Prompt not found"))?;
        if let Some(ref name) = req.name {
            validate_name(name, "Prompt name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let content = req.content.unwrap_or(existing.content);
        let category = req.category.unwrap_or(existing.category);
        let tags = match req.tags {
            Some(t) => json_encode(&t, "tags")?,
            None => json_encode(&existing.tags, "tags")?,
        };
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE prompts SET name = ?, content = ?, category = ?, tags = ?, updated_at = ? WHERE id = ?",
                params![&name, &content, &category, &tags, &now, &req.id],
            ).map_err(db_err)?;
        }
        self.get_prompt(&req.id)?
            .ok_or_else(|| AppError::internal("Prompt not found after update"))
    }

    pub fn delete_prompt(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM prompts WHERE id = ?", params![id]).map_err(db_err)?;
        Ok(affected > 0)
    }
}