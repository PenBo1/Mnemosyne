use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::Database;
use super::connection::db_err;
use crate::shared::errors::AppError;
// ChapterVersion / CreateVersionRequest / RevisionMode 已下沉到 shared/version，
// 修复 infra → features/version 反向依赖。
use crate::shared::version::{ChapterVersion, CreateVersionRequest, RevisionMode};

impl Database {
    fn map_chapter_version(row: &sqlx::sqlite::SqliteRow) -> Result<ChapterVersion, AppError> {
        let mode_str: String = row.get(8);
        Ok(ChapterVersion {
            id: row.get(0),
            novel_id: row.get(1),
            chapter_number: row.get::<i64, usize>(2) as u32,
            version_number: row.get::<i64, usize>(3) as u32,
            content: row.get(4),
            content_hash: row.get(5),
            word_count: row.get::<i64, usize>(6) as u32,
            revision_reason: row.get(7),
            revision_mode: mode_str.parse().unwrap_or(RevisionMode::Auto),
            created_at: row.get(9),
        })
    }

    pub async fn list_chapter_versions(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<ChapterVersion>, AppError> {
        let rows = sqlx::query(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(Self::map_chapter_version).collect()
    }

    pub async fn get_chapter_version(&self, version_id: &str) -> Result<Option<ChapterVersion>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE id = ?"
        )
        .bind(version_id)
        .fetch_optional(&self.pool).await.map_err(db_err)?;
        match row_opt {
            None => Ok(None),
            Some(row) => Ok(Some(Self::map_chapter_version(&row)?)),
        }
    }

    pub async fn get_latest_chapter_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<ChapterVersion>, AppError> {
        let row_opt = sqlx::query(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC LIMIT 1"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .fetch_optional(&self.pool).await.map_err(db_err)?;
        match row_opt {
            None => Ok(None),
            Some(row) => Ok(Some(Self::map_chapter_version(&row)?)),
        }
    }

    pub async fn get_next_version_number(&self, novel_id: &str, chapter_number: u32) -> Result<u32, AppError> {
        let max: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) FROM chapter_versions WHERE novel_id = ? AND chapter_number = ?"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .fetch_one(&self.pool).await.map_err(db_err)?;
        Ok((max + 1) as u32)
    }

    pub async fn create_chapter_version(
        &self,
        req: &CreateVersionRequest,
        version_number: u32,
        content_hash: &str,
        word_count: u32,
    ) -> Result<ChapterVersion, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO chapter_versions (id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(req.chapter_number as i64).bind(version_number as i64)
        .bind(&req.content).bind(content_hash).bind(word_count as i64)
        .bind(&req.revision_reason).bind(req.revision_mode.to_string()).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(ChapterVersion {
            id, novel_id: req.novel_id.clone(), chapter_number: req.chapter_number,
            version_number, content: req.content.clone(), content_hash: content_hash.to_string(),
            word_count, revision_reason: req.revision_reason.clone(), revision_mode: req.revision_mode.clone(),
            created_at: now,
        })
    }
}
