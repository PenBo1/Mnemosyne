//! ═══════════════════════════════════════════════════════════════════════════
//! 工作区存储 - Workspace 管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供 Workspace CRUD：
//! - 创建、列出、获取、更新、删除工作区
//! - 更新最近打开时间（用于恢复上次活动工作区）
//! - 级联删除关联会话及其消息

use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::types::{Workspace, CreateWorkspaceRequest, UpdateWorkspaceRequest};
use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use crate::infrastructure::db::connection::validate_name;

impl Database {
    /// 创建工作区
    pub fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace, AppError> {
        validate_name(&req.name, "Workspace name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let path = req.path.unwrap_or_default();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at, last_opened_at) VALUES (?, ?, ?, ?, ?, NULL)",
            params![&id, &req.name, &path, &now, &now],
        ).map_err(db_err)?;
        Ok(Workspace { id, name: req.name, path, created_at: now.clone(), updated_at: now, last_opened_at: None })
    }

    /// 列出工作区
    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, name, path, created_at, updated_at, last_opened_at FROM workspaces ORDER BY last_opened_at DESC NULLS LAST, created_at DESC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?, name: row.get(1)?, path: row.get(2)?,
                created_at: row.get(3)?, updated_at: row.get(4)?,
                last_opened_at: row.get(5)?,
            })
        }).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    /// 获取工作区
    pub fn get_workspace(&self, id: &str) -> Result<Option<Workspace>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, name, path, created_at, updated_at, last_opened_at FROM workspaces WHERE id = ?",
            params![id],
            |row| Ok(Workspace {
                id: row.get(0)?, name: row.get(1)?, path: row.get(2)?,
                created_at: row.get(3)?, updated_at: row.get(4)?,
                last_opened_at: row.get(5)?,
            }),
        );
        match result {
            Ok(ws) => Ok(Some(ws)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    /// 更新工作区
    pub fn update_workspace(&self, req: UpdateWorkspaceRequest) -> Result<Workspace, AppError> {
        let existing = self.get_workspace(&req.id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        if let Some(ref name) = req.name {
            validate_name(name, "Workspace name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let path = req.path.unwrap_or(existing.path);
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE workspaces SET name = ?, path = ?, updated_at = ? WHERE id = ?",
                params![&name, &path, &now, &req.id],
            ).map_err(db_err)?;
        }
        self.get_workspace(&req.id)?
            .ok_or_else(|| AppError::internal("Workspace not found after update"))
    }

    /// 更新工作区最近打开时间
    pub fn touch_workspace(&self, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE workspaces SET last_opened_at = ? WHERE id = ?",
            params![&now, id],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 删除工作区
    ///
    /// 级联删除关联会话及其消息。
    /// 使用临时表批量删除，避免 N+1 问题。
    pub fn delete_workspace(&self, id: &str) -> Result<bool, AppError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;

        // 创建临时表存储要删除的 session ID
        tx.execute(
            "CREATE TEMP TABLE IF NOT EXISTS _delete_session_ids (id TEXT PRIMARY KEY)",
            [],
        )
        .map_err(db_err)?;
        tx.execute("DELETE FROM _delete_session_ids", []).map_err(db_err)?;

        // 插入所有关联的 session ID
        tx.execute(
            "INSERT INTO _delete_session_ids SELECT id FROM sessions WHERE workspace_id = ?",
            [id],
        )
        .map_err(db_err)?;

        // 批量删除 messages
        let messages_deleted = tx
            .execute(
                "DELETE FROM messages WHERE session_id IN (SELECT id FROM _delete_session_ids)",
                [],
            )
            .map_err(db_err)?;

        // 批量删除 sessions
        let sessions_deleted = tx
            .execute("DELETE FROM sessions WHERE workspace_id = ?", [id])
            .map_err(db_err)?;

        // 删除 workspace
        let affected = tx
            .execute("DELETE FROM workspaces WHERE id = ?", [id])
            .map_err(db_err)?;

        // 清理临时表
        tx.execute("DROP TABLE _delete_session_ids", [])
            .map_err(db_err)?;

        tx.commit().map_err(db_err)?;

        tracing::debug!(
            workspace_id = id,
            workspaces_deleted = affected,
            sessions_deleted,
            messages_deleted,
            "delete_workspace completed"
        );

        Ok(affected > 0)
    }
}