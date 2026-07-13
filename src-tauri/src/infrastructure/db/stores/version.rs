
use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use crate::shared::version::types::{ChapterVersion, CreateVersionRequest, RevisionMode};

impl Database {
    fn map_chapter_version(row: &rusqlite::Row) -> Result<ChapterVersion, rusqlite::Error> {
        let mode_str: String = row.get(8)?;
        Ok(ChapterVersion {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            chapter_number: row.get::<_, i64>(2)? as u32,
            version_number: row.get::<_, i64>(3)? as u32,
            content: row.get(4)?,
            content_hash: row.get(5)?,
            word_count: row.get::<_, i64>(6)? as u32,
            revision_reason: row.get(7)?,
            revision_mode: mode_str.parse().unwrap_or(RevisionMode::Auto),
            created_at: row.get(9)?,
        })
    }

    pub fn list_chapter_versions(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<ChapterVersion>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, chapter_number as i64], Self::map_chapter_version).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn get_chapter_version(&self, version_id: &str) -> Result<Option<ChapterVersion>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE id = ?",
            params![version_id],
            Self::map_chapter_version,
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn get_latest_chapter_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<ChapterVersion>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC LIMIT 1",
            params![novel_id, chapter_number as i64],
            Self::map_chapter_version,
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn get_next_version_number(&self, novel_id: &str, chapter_number: u32) -> Result<u32, AppError> {
        let conn = self.conn()?;
        let max: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version_number), 0) FROM chapter_versions WHERE novel_id = ? AND chapter_number = ?",
            params![novel_id, chapter_number as i64],
            |row| row.get::<_, i64>(0),
        ).map_err(db_err)?;
        Ok((max + 1) as u32)
    }

    pub fn create_chapter_version(
        &self,
        req: &CreateVersionRequest,
        version_number: u32,
        content_hash: &str,
        word_count: u32,
    ) -> Result<ChapterVersion, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO chapter_versions (id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![&id, &req.novel_id, req.chapter_number as i64, version_number as i64, &req.content, content_hash, word_count as i64, &req.revision_reason, req.revision_mode.to_string(), &now],
        ).map_err(db_err)?;

        Ok(ChapterVersion {
            id, novel_id: req.novel_id.clone(), chapter_number: req.chapter_number,
            version_number, content: req.content.clone(), content_hash: content_hash.to_string(),
            word_count, revision_reason: req.revision_reason.clone(), revision_mode: req.revision_mode.clone(),
            created_at: now,
        })
    }
}