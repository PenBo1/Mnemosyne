
use rusqlite::params;
use uuid::Uuid;
use chrono::Utc;

use super::super::types::{Novel, CreateNovelRequest, UpdateNovelRequest, Chapter};
use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

impl Database {
    pub fn insert_novel(&self, id: &str, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        Self::validate_title(&req.title)?;
        Self::validate_genre(&req.genre)?;
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.conn()?;
            conn.execute(
                "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'drafting', ?, 0, 0, ?, ?, ?, ?)",
                params![id, &req.workspace_id, &req.title, &req.genre, &req.platform, &req.language, &req.target_chapters, &req.chapter_words, &now, &now],
            ).map_err(db_err)?;
        }
        self.get_novel_by_id(id)?
            .ok_or_else(|| AppError::internal("Novel not found after creation"))
    }

    pub fn create_novel(&self, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        let id = Uuid::new_v4().to_string();
        self.insert_novel(&id, req)
    }

    pub fn get_novel_by_id(&self, id: &str) -> Result<Option<Novel>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels WHERE id = ?",
            params![id],
            |row| Ok(Novel {
                id: row.get(0)?, workspace_id: row.get(1)?, title: row.get(2)?,
                genre: row.get(3)?, platform: row.get(4)?, status: row.get(5)?,
                language: row.get(6)?, word_count: row.get(7)?, chapter_count: row.get(8)?,
                target_chapters: row.get(9)?, chapter_words: row.get(10)?,
                created_at: row.get(11)?, updated_at: row.get(12)?,
            }),
        );
        match result {
            Ok(n) => Ok(Some(n)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn list_novels(&self) -> Result<Vec<Novel>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels ORDER BY updated_at DESC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            Ok(Novel {
                id: row.get(0)?, workspace_id: row.get(1)?, title: row.get(2)?,
                genre: row.get(3)?, platform: row.get(4)?, status: row.get(5)?,
                language: row.get(6)?, word_count: row.get(7)?, chapter_count: row.get(8)?,
                target_chapters: row.get(9)?, chapter_words: row.get(10)?,
                created_at: row.get(11)?, updated_at: row.get(12)?,
            })
        }).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn update_novel(&self, id: &str, req: &UpdateNovelRequest) -> Result<Novel, AppError> {
        if let Some(ref title) = req.title {
            Self::validate_title(title)?;
        }
        if let Some(ref genre) = req.genre {
            Self::validate_genre(genre)?;
        }
        let existing = self.get_novel_by_id(id)?
            .ok_or_else(|| AppError::not_found("Novel not found"))?;
        let now = Utc::now().to_rfc3339();
        let title = req.title.clone().unwrap_or(existing.title);
        let genre = req.genre.clone().unwrap_or(existing.genre);
        let platform = req.platform.clone().unwrap_or(existing.platform);
        let language = req.language.clone().unwrap_or(existing.language);
        let target_chapters = req.target_chapters.unwrap_or(existing.target_chapters);
        let chapter_words = req.chapter_words.unwrap_or(existing.chapter_words);
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE novels SET title = ?, genre = ?, platform = ?, language = ?, target_chapters = ?, chapter_words = ?, updated_at = ? WHERE id = ?",
                params![&title, &genre, &platform, &language, target_chapters, chapter_words, &now, id],
            ).map_err(db_err)?;
        }
        self.get_novel_by_id(id)?
            .ok_or_else(|| AppError::internal("Novel not found after update"))
    }

    pub fn delete_novel(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM novels WHERE id = ?", params![id]).map_err(db_err)?;
        Ok(affected > 0)
    }

    pub(super) fn validate_title(title: &str) -> Result<(), AppError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid_input("Novel title cannot be empty"));
        }
        if trimmed.len() > 500 {
            return Err(AppError::invalid_input("Novel title too long (max 500 chars)"));
        }
        Ok(())
    }

    pub(super) fn validate_genre(genre: &str) -> Result<(), AppError> {
        if genre.len() > 100 {
            return Err(AppError::invalid_input("Genre too long (max 100 chars)"));
        }
        Ok(())
    }
}

impl Database {
    pub fn create_chapter(&self, novel_id: &str, number: i64, title: &str) -> Result<Chapter, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.conn()?;
            conn.execute(
                "INSERT INTO chapters (id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at) VALUES (?, ?, ?, ?, 'drafting', 0, NULL, 0, ?, ?)",
                params![&id, novel_id, number, title, &now, &now],
            ).map_err(db_err)?;
        }
        self.get_chapter_by_id(&id)?
            .ok_or_else(|| AppError::internal("Chapter not found after creation"))
    }

    pub fn get_chapter_by_id(&self, id: &str) -> Result<Option<Chapter>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE id = ?",
            params![id],
            |row| Ok(Chapter {
                id: row.get(0)?, novel_id: row.get(1)?, number: row.get(2)?,
                title: row.get(3)?, status: row.get(4)?, word_count: row.get(5)?,
                audit_score: row.get(6)?, revision_count: row.get(7)?,
                created_at: row.get(8)?, updated_at: row.get(9)?,
            }),
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    pub fn list_chapters(&self, novel_id: &str) -> Result<Vec<Chapter>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE novel_id = ? ORDER BY number ASC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([novel_id], |row| {
            Ok(Chapter {
                id: row.get(0)?, novel_id: row.get(1)?, number: row.get(2)?,
                title: row.get(3)?, status: row.get(4)?, word_count: row.get(5)?,
                audit_score: row.get(6)?, revision_count: row.get(7)?,
                created_at: row.get(8)?, updated_at: row.get(9)?,
            })
        }).map_err(db_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
    }

    pub fn update_chapter_stats(&self, id: &str, word_count: i64, audit_score: Option<f64>, revision_count: i64) -> Result<Chapter, AppError> {
        let now = Utc::now().to_rfc3339();
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE chapters SET word_count = ?, audit_score = ?, revision_count = ?, updated_at = ? WHERE id = ?",
                params![word_count, audit_score, revision_count, &now, id],
            ).map_err(db_err)?;
        }
        self.get_chapter_by_id(id)?
            .ok_or_else(|| AppError::internal("Chapter not found after update"))
    }

    pub fn delete_chapter(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM chapters WHERE id = ?", params![id]).map_err(db_err)?;
        Ok(affected > 0)
    }
}