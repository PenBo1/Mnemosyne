use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{KanbanTask, KanbanColumn, CreateKanbanTaskRequest, UpdateKanbanTaskRequest, CreateKanbanColumnRequest, UpdateKanbanColumnRequest};
use super::Database;
use super::connection::db_err;
use super::error::{json_decode, json_encode};
use crate::shared::errors::AppError;

fn map_kanban_task(row: &sqlx::sqlite::SqliteRow) -> Result<KanbanTask, AppError> {
    let tags_json: String = row.get(10);
    Ok(KanbanTask {
        id: row.get(0),
        novel_id: row.get(1),
        title: row.get(2),
        description: row.get(3),
        status: row.get(4),
        priority: row.get(5),
        assigned_agent: row.get(6),
        chapter_id: row.get(7),
        parent_task_id: row.get(8),
        tags: json_decode(&tags_json, "tags")?,
        sort_order: row.get(11),
        due_date: row.get(12),
        created_at: row.get(13),
        updated_at: row.get(14),
    })
}

fn map_kanban_column(row: &sqlx::sqlite::SqliteRow) -> KanbanColumn {
    KanbanColumn {
        id: row.get(0),
        novel_id: row.get(1),
        name: row.get(2),
        status_key: row.get(3),
        color: row.get(4),
        sort_order: row.get(5),
        wip_limit: row.get(6),
        created_at: row.get(7),
    }
}

impl Database {
    pub async fn create_kanban_task(&self, novel_id: &str, req: CreateKanbanTaskRequest) -> Result<KanbanTask, AppError> {
        Self::validate_name(&req.title, "Task title")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let status = req.status.unwrap_or_else(|| "plan".to_string());
        let priority = req.priority.unwrap_or_else(|| "medium".to_string());
        let tags_json = json_encode(&req.tags.unwrap_or_default(), "tags")?;
        let description = req.description.unwrap_or_default();

        sqlx::query(
            "INSERT INTO kanban_tasks (id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(&req.title).bind(&description).bind(&status).bind(&priority)
        .bind(&req.assigned_agent).bind(&req.chapter_id).bind(&req.parent_task_id)
        .bind(&tags_json).bind(&req.due_date).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(KanbanTask {
            id,
            novel_id: novel_id.to_string(),
            title: req.title,
            description,
            status,
            priority,
            assigned_agent: req.assigned_agent,
            chapter_id: req.chapter_id,
            parent_task_id: req.parent_task_id,
            tags: json_decode(&tags_json, "tags")?,
            sort_order: 0,
            due_date: req.due_date,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_kanban_tasks(&self, novel_id: &str, status_filter: Option<&str>) -> Result<Vec<KanbanTask>, AppError> {
        let rows = if let Some(s) = status_filter {
            sqlx::query(
                "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE novel_id = ? AND status = ? ORDER BY sort_order, created_at"
            )
            .bind(novel_id).bind(s)
            .fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query(
                "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE novel_id = ? ORDER BY sort_order, created_at"
            )
            .bind(novel_id)
            .fetch_all(&self.pool).await.map_err(db_err)?
        };
        rows.iter().map(map_kanban_task).collect()
    }

    pub async fn update_kanban_task(&self, task_id: &str, req: UpdateKanbanTaskRequest) -> Result<KanbanTask, AppError> {
        let existing = self.get_kanban_task_by_id(task_id).await?
            .ok_or_else(|| AppError::not_found("Kanban task"))?;
        let now = Utc::now().to_rfc3339();

        let title = req.title.unwrap_or(existing.title);
        let description = req.description.unwrap_or(existing.description);
        let status = req.status.unwrap_or(existing.status);
        let priority = req.priority.unwrap_or(existing.priority);
        let sort_order = req.sort_order.unwrap_or(existing.sort_order);
        let tags_json = match req.tags {
            Some(t) => json_encode(&t, "tags")?,
            None => json_encode(&existing.tags, "tags")?,
        };

        sqlx::query(
            "UPDATE kanban_tasks SET title = ?, description = ?, status = ?, priority = ?, assigned_agent = ?, chapter_id = ?, parent_task_id = ?, sort_order = ?, due_date = ?, tags = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&title).bind(&description).bind(&status).bind(&priority)
        .bind(&req.assigned_agent).bind(&req.chapter_id).bind(&req.parent_task_id)
        .bind(sort_order).bind(&req.due_date).bind(&tags_json).bind(&now).bind(task_id)
        .execute(&self.pool).await.map_err(db_err)?;

        self.get_kanban_task_by_id(task_id).await?
            .ok_or_else(|| AppError::not_found("Kanban task"))
    }

    pub async fn get_kanban_task_by_id(&self, task_id: &str) -> Result<Option<KanbanTask>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE id = ?"
        )
        .bind(task_id)
        .fetch_optional(&self.pool).await.map_err(db_err)?;
        match row_opt {
            None => Ok(None),
            Some(row) => Ok(Some(map_kanban_task(&row)?)),
        }
    }

    pub async fn delete_kanban_task(&self, task_id: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM kanban_tasks WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool).await.map_err(db_err)?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Kanban task"));
        }
        Ok(())
    }

    /// 批量重排序任务，用事务保证原子性。
    pub async fn reorder_kanban_tasks(&self, task_ids: &[String]) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        for (i, id) in task_ids.iter().enumerate() {
            sqlx::query("UPDATE kanban_tasks SET sort_order = ?, updated_at = ? WHERE id = ?")
                .bind(i as i32).bind(&now).bind(id)
                .execute(&mut *tx).await.map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn get_kanban_columns(&self, novel_id: &str) -> Result<Vec<KanbanColumn>, AppError> {
        let rows = sqlx::query(
            "SELECT id, novel_id, name, status_key, color, sort_order, wip_limit, created_at FROM kanban_columns WHERE novel_id = ? ORDER BY sort_order"
        )
        .bind(novel_id)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        Ok(rows.iter().map(map_kanban_column).collect())
    }

    pub async fn create_kanban_column(&self, novel_id: &str, req: CreateKanbanColumnRequest) -> Result<KanbanColumn, AppError> {
        Self::validate_name(&req.name, "Column name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let color = req.color.unwrap_or_else(|| "#6366f1".to_string());
        let sort_order = req.sort_order.unwrap_or(0);

        sqlx::query(
            "INSERT INTO kanban_columns (id, novel_id, name, status_key, color, sort_order, wip_limit, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(&req.name).bind(&req.status_key).bind(&color)
        .bind(sort_order).bind(req.wip_limit).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(KanbanColumn {
            id,
            novel_id: novel_id.to_string(),
            name: req.name,
            status_key: req.status_key,
            color,
            sort_order,
            wip_limit: req.wip_limit,
            created_at: now,
        })
    }

    pub async fn update_kanban_column(&self, column_id: &str, req: UpdateKanbanColumnRequest) -> Result<KanbanColumn, AppError> {
        let existing = self.get_kanban_column_by_id(column_id).await?
            .ok_or_else(|| AppError::not_found("Kanban column"))?;

        let name = req.name.unwrap_or(existing.name);
        let color = req.color.unwrap_or(existing.color);
        let sort_order = req.sort_order.unwrap_or(existing.sort_order);
        let wip_limit = req.wip_limit.or(existing.wip_limit);

        sqlx::query(
            "UPDATE kanban_columns SET name = ?, color = ?, sort_order = ?, wip_limit = ? WHERE id = ?"
        )
        .bind(&name).bind(&color).bind(sort_order).bind(wip_limit).bind(column_id)
        .execute(&self.pool).await.map_err(db_err)?;

        self.get_kanban_column_by_id(column_id).await?
            .ok_or_else(|| AppError::not_found("Kanban column"))
    }

    pub async fn get_kanban_column_by_id(&self, column_id: &str) -> Result<Option<KanbanColumn>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, novel_id, name, status_key, color, sort_order, wip_limit, created_at FROM kanban_columns WHERE id = ?"
        )
        .bind(column_id)
        .fetch_optional(&self.pool).await.map_err(db_err)?;
        Ok(row_opt.map(|row| map_kanban_column(&row)))
    }

    pub async fn delete_kanban_column(&self, column_id: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM kanban_columns WHERE id = ?")
            .bind(column_id)
            .execute(&self.pool).await.map_err(db_err)?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Kanban column"));
        }
        Ok(())
    }
}
