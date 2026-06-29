use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{Novel, CreateNovelRequest, UpdateNovelRequest, Chapter};
use super::Database;
use super::connection::db_err;
use crate::shared::errors::AppError;

impl Database {
    pub async fn insert_novel(&self, id: &str, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        Self::validate_title(&req.title)?;
        Self::validate_genre(&req.genre)?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'drafting', ?, 0, 0, ?, ?, ?, ?)"
        )
        .bind(id).bind(&req.workspace_id).bind(&req.title).bind(&req.genre)
        .bind(&req.platform).bind(&req.language).bind(&req.target_chapters)
        .bind(&req.chapter_words).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_novel_by_id(id).await?
            .ok_or_else(|| AppError::internal("Novel not found after creation"))
    }

    pub async fn create_novel(&self, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        Self::validate_title(&req.title)?;
        Self::validate_genre(&req.genre)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'drafting', ?, 0, 0, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.workspace_id).bind(&req.title).bind(&req.genre)
        .bind(&req.platform).bind(&req.language).bind(&req.target_chapters)
        .bind(&req.chapter_words).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_novel_by_id(&id).await?
            .ok_or_else(|| AppError::internal("Novel not found after creation"))
    }

    pub async fn get_novel_by_id(&self, id: &str) -> Result<Option<Novel>, AppError> {
        sqlx::query(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels WHERE id = ?"
        )
        .bind(id)
        .map(|row: sqlx::sqlite::SqliteRow| Novel {
            id: row.get(0), workspace_id: row.get(1), title: row.get(2),
            genre: row.get(3), platform: row.get(4), status: row.get(5),
            language: row.get(6), word_count: row.get(7), chapter_count: row.get(8),
            target_chapters: row.get(9), chapter_words: row.get(10),
            created_at: row.get(11), updated_at: row.get(12),
        })
        .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn list_novels(&self) -> Result<Vec<Novel>, AppError> {
        sqlx::query(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels ORDER BY updated_at DESC"
        )
        .map(|row: sqlx::sqlite::SqliteRow| Novel {
            id: row.get(0), workspace_id: row.get(1), title: row.get(2),
            genre: row.get(3), platform: row.get(4), status: row.get(5),
            language: row.get(6), word_count: row.get(7), chapter_count: row.get(8),
            target_chapters: row.get(9), chapter_words: row.get(10),
            created_at: row.get(11), updated_at: row.get(12),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn update_novel(&self, id: &str, req: &UpdateNovelRequest) -> Result<Novel, AppError> {
        if let Some(ref title) = req.title {
            Self::validate_title(title)?;
        }
        if let Some(ref genre) = req.genre {
            Self::validate_genre(genre)?;
        }
        let existing = self.get_novel_by_id(id).await?
            .ok_or_else(|| AppError::not_found("Novel not found"))?;
        let now = Utc::now().to_rfc3339();
        let title = req.title.clone().unwrap_or(existing.title);
        let genre = req.genre.clone().unwrap_or(existing.genre);
        let platform = req.platform.clone().unwrap_or(existing.platform);
        let language = req.language.clone().unwrap_or(existing.language);
        let target_chapters = req.target_chapters.unwrap_or(existing.target_chapters);
        let chapter_words = req.chapter_words.unwrap_or(existing.chapter_words);
        sqlx::query(
            "UPDATE novels SET title = ?, genre = ?, platform = ?, language = ?, target_chapters = ?, chapter_words = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&title).bind(&genre).bind(&platform).bind(&language)
        .bind(target_chapters).bind(chapter_words).bind(&now).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_novel_by_id(id).await?
            .ok_or_else(|| AppError::internal("Novel not found after update"))
    }

    pub async fn delete_novel(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM novels WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
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

// ═══════════════════════════════════════════════════════════
// Chapters
// ═══════════════════════════════════════════════════════════

impl Database {
    pub async fn create_chapter(&self, novel_id: &str, number: i64, title: &str) -> Result<Chapter, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO chapters (id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at) VALUES (?, ?, ?, ?, 'drafting', 0, NULL, 0, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(number).bind(title).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_chapter_by_id(&id).await?
            .ok_or_else(|| AppError::internal("Chapter not found after creation"))
    }

    pub async fn get_chapter_by_id(&self, id: &str) -> Result<Option<Chapter>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE id = ?"
        )
        .bind(id)
        .map(|row: sqlx::sqlite::SqliteRow| Chapter {
            id: row.get(0), novel_id: row.get(1), number: row.get(2),
            title: row.get(3), status: row.get(4), word_count: row.get(5),
            audit_score: row.get(6), revision_count: row.get(7),
            created_at: row.get(8), updated_at: row.get(9),
        })
        .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn list_chapters(&self, novel_id: &str) -> Result<Vec<Chapter>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE novel_id = ? ORDER BY number ASC"
        )
        .bind(novel_id)
        .map(|row: sqlx::sqlite::SqliteRow| Chapter {
            id: row.get(0), novel_id: row.get(1), number: row.get(2),
            title: row.get(3), status: row.get(4), word_count: row.get(5),
            audit_score: row.get(6), revision_count: row.get(7),
            created_at: row.get(8), updated_at: row.get(9),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn update_chapter_stats(&self, id: &str, word_count: i64, audit_score: Option<f64>, revision_count: i64) -> Result<Chapter, AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE chapters SET word_count = ?, audit_score = ?, revision_count = ?, updated_at = ? WHERE id = ?"
        )
        .bind(word_count).bind(audit_score).bind(revision_count).bind(&now).bind(id)
        .execute(&self.pool).await.map_err(db_err)?;
        self.get_chapter_by_id(id).await?
            .ok_or_else(|| AppError::internal("Chapter not found after update"))
    }

    pub async fn delete_chapter(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM chapters WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}
