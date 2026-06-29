use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{Prompt, CreatePromptRequest, UpdatePromptRequest};
use super::Database;
use super::connection::db_err;
use super::error::{json_decode, json_encode};
use crate::shared::errors::AppError;

impl Database {
    pub async fn create_prompt(&self, req: CreatePromptRequest) -> Result<Prompt, AppError> {
        Self::validate_name(&req.name, "Prompt name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags = json_encode(&req.tags, "tags")?;
        sqlx::query(
            "INSERT INTO prompts (id, name, content, category, tags, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.name).bind(&req.content).bind(&req.category)
        .bind(&tags).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(Prompt { id, name: req.name, content: req.content, category: req.category, tags: req.tags, created_at: now.clone(), updated_at: now })
    }

    pub async fn list_prompts(&self, category: Option<&str>) -> Result<Vec<Prompt>, AppError> {
        let rows = if let Some(cat) = category {
            sqlx::query(
                "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE category = ? ORDER BY updated_at DESC"
            )
            .bind(cat)
            .fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query(
                "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts ORDER BY updated_at DESC"
            )
            .fetch_all(&self.pool).await.map_err(db_err)?
        };
        rows.iter().map(|row| {
            let tags_str: String = row.get(4);
            Ok(Prompt {
                id: row.get(0), name: row.get(1), content: row.get(2),
                category: row.get(3), tags: json_decode(&tags_str, "tags")?,
                created_at: row.get(5), updated_at: row.get(6),
            })
        }).collect()
    }

    pub async fn get_prompt(&self, id: &str) -> Result<Option<Prompt>, AppError> {
        let row_opt = sqlx::query("SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool).await.map_err(db_err)?;
        match row_opt {
            None => Ok(None),
            Some(row) => {
                let tags_str: String = row.get(4);
                Ok(Some(Prompt {
                    id: row.get(0), name: row.get(1), content: row.get(2),
                    category: row.get(3), tags: json_decode(&tags_str, "tags")?,
                    created_at: row.get(5), updated_at: row.get(6),
                }))
            }
        }
    }

    pub async fn update_prompt(&self, req: UpdatePromptRequest) -> Result<Prompt, AppError> {
        let existing = self.get_prompt(&req.id).await?
            .ok_or_else(|| AppError::not_found("Prompt not found"))?;
        if let Some(ref name) = req.name {
            Self::validate_name(name, "Prompt name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let content = req.content.unwrap_or(existing.content);
        let category = req.category.unwrap_or(existing.category);
        let tags = match req.tags {
            Some(t) => json_encode(&t, "tags")?,
            None => json_encode(&existing.tags, "tags")?,
        };
        sqlx::query(
            "UPDATE prompts SET name = ?, content = ?, category = ?, tags = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&name).bind(&content).bind(&category).bind(&tags).bind(&now).bind(&req.id)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_prompt(&req.id).await?
            .ok_or_else(|| AppError::internal("Prompt not found after update"))
    }

    pub async fn delete_prompt(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM prompts WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}
