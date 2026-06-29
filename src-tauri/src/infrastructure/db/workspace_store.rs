use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{Workspace, CreateWorkspaceRequest, UpdateWorkspaceRequest};
use super::Database;
use super::connection::db_err;
use crate::shared::errors::AppError;

impl Database {
    pub async fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace, AppError> {
        Self::validate_name(&req.name, "Workspace name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let path = req.path.unwrap_or_default();
        sqlx::query("INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&id).bind(&req.name).bind(&path).bind(&now).bind(&now)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(Workspace { id, name: req.name, path, created_at: now.clone(), updated_at: now })
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>, AppError> {
        sqlx::query("SELECT id, name, path, created_at, updated_at FROM workspaces ORDER BY created_at DESC")
            .map(|row: sqlx::sqlite::SqliteRow| Workspace {
                id: row.get(0), name: row.get(1), path: row.get(2),
                created_at: row.get(3), updated_at: row.get(4),
            })
            .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>, AppError> {
        sqlx::query("SELECT id, name, path, created_at, updated_at FROM workspaces WHERE id = ?")
            .bind(id)
            .map(|row: sqlx::sqlite::SqliteRow| Workspace {
                id: row.get(0), name: row.get(1), path: row.get(2),
                created_at: row.get(3), updated_at: row.get(4),
            })
            .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn update_workspace(&self, req: UpdateWorkspaceRequest) -> Result<Workspace, AppError> {
        let existing = self.get_workspace(&req.id).await?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        if let Some(ref name) = req.name {
            Self::validate_name(name, "Workspace name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let path = req.path.unwrap_or(existing.path);
        sqlx::query("UPDATE workspaces SET name = ?, path = ?, updated_at = ? WHERE id = ?")
            .bind(&name).bind(&path).bind(&now).bind(&req.id)
            .execute(&self.pool).await.map_err(db_err)?;
        self.get_workspace(&req.id).await?
            .ok_or_else(|| AppError::internal("Workspace not found after update"))
    }

    /// 删除 workspace 并级联清理关联数据（novels/chapters/sessions 等通过 FK CASCADE）。
    pub async fn delete_workspace(&self, id: &str) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let result = sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id)
            .execute(&mut *tx).await.map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    pub(super) fn validate_name(name: &str, field: &str) -> Result<(), AppError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid_input(format!("{} cannot be empty", field)));
        }
        if trimmed.len() > 255 {
            return Err(AppError::invalid_input(format!("{} too long (max 255 chars)", field)));
        }
        Ok(())
    }
}
