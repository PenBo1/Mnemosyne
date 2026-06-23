use rusqlite::{Connection, params, Row, OptionalExtension};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

use super::models::*;
use crate::errors::AppError;

const SCHEMA_SQL: &str = include_str!("sql/schema.sql");
const FEEDBACK_SCHEMA_SQL: &str = include_str!("sql/feedback_schema.sql");

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create db directory: {}", e)))?;
        let conn = Connection::open(db_path)
            .map_err(|e| AppError::internal(format!("Failed to open database: {}", e)))?;
        let db = Self { conn };
        db.conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| AppError::internal(format!("Failed to init state schema: {}", e)))?;
        Ok(db)
    }

    pub fn new_feedback(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid feedback database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create feedback db directory: {}", e)))?;
        let conn = Connection::open(db_path)
            .map_err(|e| AppError::internal(format!("Failed to open feedback database: {}", e)))?;
        let db = Self { conn };
        db.conn.execute_batch(FEEDBACK_SCHEMA_SQL)
            .map_err(|e| AppError::internal(format!("Failed to init feedback schema: {}", e)))?;
        Ok(db)
    }

    // ═══════════════════════════════════════════════════════════
    // Validation helpers
    // ═══════════════════════════════════════════════════════════

    fn validate_name(name: &str, field: &str) -> Result<(), AppError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid_input(format!("{} cannot be empty", field)));
        }
        if trimmed.len() > 255 {
            return Err(AppError::invalid_input(format!("{} too long (max 255 chars)", field)));
        }
        Ok(())
    }

    fn validate_title(title: &str) -> Result<(), AppError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid_input("Novel title cannot be empty"));
        }
        if trimmed.len() > 500 {
            return Err(AppError::invalid_input("Novel title too long (max 500 chars)"));
        }
        Ok(())
    }

    fn validate_genre(genre: &str) -> Result<(), AppError> {
        if genre.len() > 100 {
            return Err(AppError::invalid_input("Genre too long (max 100 chars)"));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════
    // Workspaces
    // ═══════════════════════════════════════════════════════════

    fn row_to_workspace(row: &Row) -> rusqlite::Result<Workspace> {
        Ok(Workspace {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    pub fn create_workspace(&self, req: CreateWorkspaceRequest) -> Result<Workspace, AppError> {
        Self::validate_name(&req.name, "Workspace name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let path = req.path.unwrap_or_default();
        self.conn.execute(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, req.name, path, now, now],
        ).map_err(|e| AppError::internal(format!("Failed to create workspace: {}", e)))?;
        Ok(Workspace { id, name: req.name, path, created_at: now.clone(), updated_at: now })
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, created_at, updated_at FROM workspaces ORDER BY created_at DESC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map([], |row| Self::row_to_workspace(row))
            .map_err(|e| AppError::internal(format!("Failed to query workspaces: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect workspaces: {}", e)))
    }

    pub fn get_workspace(&self, id: &str) -> Result<Option<Workspace>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, created_at, updated_at FROM workspaces WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query workspace: {}", e)))?;
        match rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(Self::row_to_workspace(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn update_workspace(&self, req: UpdateWorkspaceRequest) -> Result<Workspace, AppError> {
        let existing = self.get_workspace(&req.id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        if let Some(ref name) = req.name {
            Self::validate_name(name, "Workspace name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let path = req.path.unwrap_or(existing.path);
        self.conn.execute(
            "UPDATE workspaces SET name = ?1, path = ?2, updated_at = ?3 WHERE id = ?4",
            params![name, path, now, req.id],
        ).map_err(|e| AppError::internal(format!("Failed to update workspace: {}", e)))?;
        self.get_workspace(&req.id)?
            .ok_or_else(|| AppError::internal("Workspace not found after update"))
    }

    pub fn delete_workspace(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete workspace: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Novels
    // ═══════════════════════════════════════════════════════════

    fn row_to_novel(row: &Row) -> rusqlite::Result<Novel> {
        Ok(Novel {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            title: row.get(2)?,
            genre: row.get(3)?,
            platform: row.get(4)?,
            status: row.get(5)?,
            language: row.get(6)?,
            word_count: row.get(7)?,
            chapter_count: row.get(8)?,
            target_chapters: row.get(9)?,
            chapter_words: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    }

    pub fn insert_novel(&self, id: &str, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        Self::validate_title(&req.title)?;
        Self::validate_genre(&req.genre)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'drafting', ?6, 0, 0, ?7, ?8, ?9, ?9)",
            params![id, req.workspace_id, req.title, req.genre, req.platform, req.language, req.target_chapters, req.chapter_words, now],
        ).map_err(|e| AppError::internal(format!("Failed to create novel: {}", e)))?;
        self.get_novel_by_id(id)?
            .ok_or_else(|| AppError::internal("Novel not found after creation"))
    }

    pub fn create_novel(&self, req: &CreateNovelRequest) -> Result<Novel, AppError> {
        Self::validate_title(&req.title)?;
        Self::validate_genre(&req.genre)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'drafting', ?6, 0, 0, ?7, ?8, ?9, ?9)",
            params![id, req.workspace_id, req.title, req.genre, req.platform, req.language, req.target_chapters, req.chapter_words, now],
        ).map_err(|e| AppError::internal(format!("Failed to create novel: {}", e)))?;
        self.get_novel_by_id(&id)?
            .ok_or_else(|| AppError::internal("Novel not found after creation"))
    }

    pub fn get_novel_by_id(&self, id: &str) -> Result<Option<Novel>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query novel: {}", e)))?;
        match rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(Self::row_to_novel(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn list_novels(&self) -> Result<Vec<Novel>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at FROM novels ORDER BY updated_at DESC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map([], |row| Self::row_to_novel(row))
            .map_err(|e| AppError::internal(format!("Failed to query novels: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect novels: {}", e)))
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
        self.conn.execute(
            "UPDATE novels SET title = ?1, genre = ?2, platform = ?3, language = ?4, target_chapters = ?5, chapter_words = ?6, updated_at = ?7 WHERE id = ?8",
            params![title, genre, platform, language, target_chapters, chapter_words, now, id],
        ).map_err(|e| AppError::internal(format!("Failed to update novel: {}", e)))?;
        self.get_novel_by_id(id)?
            .ok_or_else(|| AppError::internal("Novel not found after update"))
    }

    pub fn delete_novel(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM novels WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete novel: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Chapters
    // ═══════════════════════════════════════════════════════════

    fn row_to_chapter(row: &Row) -> rusqlite::Result<Chapter> {
        Ok(Chapter {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            number: row.get(2)?,
            title: row.get(3)?,
            status: row.get(4)?,
            word_count: row.get(5)?,
            audit_score: row.get(6)?,
            revision_count: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }

    pub fn create_chapter(&self, novel_id: &str, number: i64, title: &str) -> Result<Chapter, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO chapters (id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'drafting', 0, NULL, 0, ?5, ?5)",
            params![id, novel_id, number, title, now],
        ).map_err(|e| AppError::internal(format!("Failed to create chapter: {}", e)))?;
        self.get_chapter_by_id(&id)?
            .ok_or_else(|| AppError::internal("Chapter not found after creation"))
    }

    pub fn get_chapter_by_id(&self, id: &str) -> Result<Option<Chapter>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query chapter: {}", e)))?;
        match rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(Self::row_to_chapter(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn list_chapters(&self, novel_id: &str) -> Result<Vec<Chapter>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, novel_id, number, title, status, word_count, audit_score, revision_count, created_at, updated_at FROM chapters WHERE novel_id = ?1 ORDER BY number ASC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let rows = stmt.query_map(params![novel_id], |row| Self::row_to_chapter(row))
            .map_err(|e| AppError::internal(format!("Failed to query chapters: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect chapters: {}", e)))
    }

    pub fn update_chapter_stats(&self, id: &str, word_count: i64, audit_score: Option<f64>, revision_count: i64) -> Result<Chapter, AppError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE chapters SET word_count = ?1, audit_score = ?2, revision_count = ?3, updated_at = ?4 WHERE id = ?5",
            params![word_count, audit_score, revision_count, now, id],
        ).map_err(|e| AppError::internal(format!("Failed to update chapter: {}", e)))?;
        self.get_chapter_by_id(id)?
            .ok_or_else(|| AppError::internal("Chapter not found after update"))
    }

    pub fn delete_chapter(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM chapters WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete chapter: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Prompts
    // ═══════════════════════════════════════════════════════════

    fn row_to_prompt(row: &Row) -> rusqlite::Result<Prompt> {
        Ok(Prompt {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
            category: row.get(3)?,
            tags: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    pub fn create_prompt(&self, req: CreatePromptRequest) -> Result<Prompt, AppError> {
        Self::validate_name(&req.name, "Prompt name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags = serde_json::to_string(&req.tags).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO prompts (id, name, content, category, tags, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, req.name, req.content, req.category, tags, now, now],
        ).map_err(|e| AppError::internal(format!("Failed to create prompt: {}", e)))?;
        Ok(Prompt { id, name: req.name, content: req.content, category: req.category, tags: req.tags, created_at: now.clone(), updated_at: now })
    }

    pub fn list_prompts(&self, category: Option<&str>) -> Result<Vec<Prompt>, AppError> {
        let sql = match category {
            Some(_) => "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE category = ?1 ORDER BY updated_at DESC",
            None => "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts ORDER BY updated_at DESC",
        };
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = match category {
            Some(cat) => vec![Box::new(cat.to_string())],
            None => vec![],
        };
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| Self::row_to_prompt(row))
            .map_err(|e| AppError::internal(format!("Failed to query prompts: {}", e)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal(format!("Failed to collect prompts: {}", e)))
    }

    pub fn get_prompt(&self, id: &str) -> Result<Option<Prompt>, AppError> {
        let mut stmt = self.conn.prepare("SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE id = ?1")
            .map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query prompt: {}", e)))?;
        match rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(Self::row_to_prompt(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn update_prompt(&self, req: UpdatePromptRequest) -> Result<Prompt, AppError> {
        let existing = self.get_prompt(&req.id)?
            .ok_or_else(|| AppError::not_found("Prompt not found"))?;
        if let Some(ref name) = req.name {
            Self::validate_name(name, "Prompt name")?;
        }
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let content = req.content.unwrap_or(existing.content);
        let category = req.category.unwrap_or(existing.category);
        let tags = req.tags.map(|t| serde_json::to_string(&t).unwrap_or_default())
            .unwrap_or_else(|| serde_json::to_string(&existing.tags).unwrap_or_default());
        self.conn.execute(
            "UPDATE prompts SET name = ?1, content = ?2, category = ?3, tags = ?4, updated_at = ?5 WHERE id = ?6",
            params![name, content, category, tags, now, req.id],
        ).map_err(|e| AppError::internal(format!("Failed to update prompt: {}", e)))?;
        self.get_prompt(&req.id)?
            .ok_or_else(|| AppError::internal("Prompt not found after update"))
    }

    pub fn delete_prompt(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete prompt: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Trends
    // ═══════════════════════════════════════════════════════════

    fn row_to_trend(row: &Row) -> rusqlite::Result<Trend> {
        Ok(Trend {
            id: row.get(0)?,
            keyword: row.get(1)?,
            platform: row.get(2)?,
            score: row.get(3)?,
            metadata: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            scanned_at: row.get(5)?,
        })
    }

    pub fn create_trend(&self, keyword: &str, platform: &str, score: f64, metadata: serde_json::Value) -> Result<Trend, AppError> {
        Self::validate_name(keyword, "Trend keyword")?;
        Self::validate_name(platform, "Trend platform")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let meta_str = serde_json::to_string(&metadata).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO trends (id, keyword, platform, score, metadata, scanned_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, keyword, platform, score, meta_str, now],
        ).map_err(|e| AppError::internal(format!("Failed to create trend: {}", e)))?;
        Ok(Trend { id, keyword: keyword.to_string(), platform: platform.to_string(), score, metadata, scanned_at: now })
    }

    pub fn list_trends(&self, platform: Option<&str>, limit: Option<i64>) -> Result<Vec<Trend>, AppError> {
        let limit = limit.unwrap_or(100).max(1).min(1000);
        let mut result = Vec::new();
        if let Some(p) = platform {
            let mut stmt = self.conn.prepare(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends WHERE platform = ?1 ORDER BY scanned_at DESC LIMIT ?2"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let rows = stmt.query_map(params![p, limit], |row| Self::row_to_trend(row))
                .map_err(|e| AppError::internal(format!("Failed to query trends: {}", e)))?;
            for row in rows {
                result.push(row.map_err(|e| AppError::internal(format!("Failed to read trend: {}", e)))?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends ORDER BY scanned_at DESC LIMIT ?1"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let rows = stmt.query_map(params![limit], |row| Self::row_to_trend(row))
                .map_err(|e| AppError::internal(format!("Failed to query trends: {}", e)))?;
            for row in rows {
                result.push(row.map_err(|e| AppError::internal(format!("Failed to read trend: {}", e)))?);
            }
        }
        Ok(result)
    }

    pub fn delete_trend(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM trends WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete trend: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Radar: Market Intelligence
    // ═══════════════════════════════════════════════════════════

    pub fn create_radar_scan(
        &self,
        market_summary: &str,
        recommendations: &[RadarRecommendation],
        raw_rankings: &[PlatformRankings],
    ) -> Result<RadarScan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let recs_json = serde_json::to_string(recommendations).unwrap_or_default();
        let raw_json = serde_json::to_string(raw_rankings).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO radar_scans (id, market_summary, recommendations_json, raw_rankings_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, market_summary, recs_json, raw_json, now],
        ).map_err(|e| AppError::internal(format!("Failed to create radar scan: {}", e)))?;
        Ok(RadarScan {
            id,
            market_summary: market_summary.to_string(),
            recommendations: recommendations.to_vec(),
            raw_rankings: raw_rankings.to_vec(),
            created_at: now,
        })
    }

    pub fn list_radar_scans(&self, limit: Option<i64>) -> Result<Vec<RadarScan>, AppError> {
        let limit = limit.unwrap_or(50).max(1).min(500);
        let mut stmt = self.conn.prepare(
            "SELECT id, market_summary, recommendations_json, raw_rankings_json, created_at FROM radar_scans ORDER BY created_at DESC LIMIT ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![limit])
            .map_err(|e| AppError::internal(format!("Failed to query radar scans: {}", e)))?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            let id: String = row.get(0)?;
            let market_summary: String = row.get(1)?;
            let recs_str: String = row.get(2)?;
            let raw_str: String = row.get(3)?;
            let created_at: String = row.get(4)?;
            let recommendations: Vec<RadarRecommendation> = serde_json::from_str(&recs_str).unwrap_or_default();
            let raw_rankings: Vec<PlatformRankings> = serde_json::from_str(&raw_str).unwrap_or_default();
            result.push(RadarScan { id, market_summary, recommendations, raw_rankings, created_at });
        }
        Ok(result)
    }

    pub fn delete_radar_scan(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM radar_scans WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete radar scan: {}", e)))?;
        Ok(affected > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Stats
    // ═══════════════════════════════════════════════════════════

    pub fn get_stats(&self) -> Result<serde_json::Value, AppError> {
        let prompt_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get(0))
            .map_err(|e| AppError::internal(format!("Failed to count prompts: {}", e)))?;
        let novel_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM novels", [], |row| row.get(0))
            .map_err(|e| AppError::internal(format!("Failed to count novels: {}", e)))?;
        let trend_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM trends", [], |row| row.get(0))
            .map_err(|e| AppError::internal(format!("Failed to count trends: {}", e)))?;
        let total_words: i64 = self.conn.query_row("SELECT COALESCE(SUM(word_count), 0) FROM novels", [], |row| row.get(0))
            .map_err(|e| AppError::internal(format!("Failed to sum words: {}", e)))?;
        Ok(serde_json::json!({ "promptCount": prompt_count, "novelCount": novel_count, "trendCount": trend_count, "totalWords": total_words }))
    }

    pub fn get_daily_activity(&self) -> Result<serde_json::Value, AppError> {
        let chat_activity: Vec<(String, i64)> = self.conn.prepare(
            "SELECT DATE(created_at) as date, COUNT(*) FROM sessions WHERE created_at >= DATE('now', '-1 year') GROUP BY DATE(created_at) ORDER BY date"
        )
        .map_err(|e| AppError::internal(format!("Failed to prepare chat activity query: {}", e)))?
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| AppError::internal(format!("Failed to query chat activity: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

        let chat_json: Vec<serde_json::Value> = chat_activity.into_iter()
            .map(|(date, count)| serde_json::json!({ "date": date, "count": count }))
            .collect();

        Ok(serde_json::json!({
            "chatActivity": chat_json,
            "novelActivity": []
        }))
    }

    // ═══════════════════════════════════════════════════════════
    // Wiki Entries
    // ═══════════════════════════════════════════════════════════

    fn row_to_wiki_entry(row: &Row) -> rusqlite::Result<crate::domain::wiki::WikiEntry> {
        let category_str: String = row.get(4)?;
        let source_type_str: String = row.get(5)?;
        let tags_json: String = row.get(7)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(crate::domain::wiki::WikiEntry {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            title: row.get(2)?,
            content: row.get(3)?,
            category: category_str.parse().unwrap_or(crate::domain::wiki::WikiCategory::General),
            source_type: source_type_str.parse().unwrap_or(crate::domain::wiki::WikiSourceType::Manual),
            source_chapter: row.get::<_, Option<i64>>(6)?.map(|n| n as u32),
            tags,
            importance: row.get::<_, i64>(8)? as u32,
            word_count: row.get::<_, i64>(9)? as u32,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }

    pub fn list_wiki_entries(
        &self,
        novel_id: &str,
        category: Option<&crate::domain::wiki::WikiCategory>,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        let sql = if category.is_some() {
            "SELECT * FROM wiki_entries WHERE novel_id = ?1 AND category = ?2 ORDER BY importance DESC, updated_at DESC"
        } else {
            "SELECT * FROM wiki_entries WHERE novel_id = ?1 ORDER BY importance DESC, updated_at DESC"
        };        
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare wiki list query: {}", e)))?;
        
        let rows = if let Some(cat) = category {
            stmt.query_map(params![novel_id, cat.to_string()], Self::row_to_wiki_entry)
        } else {
            stmt.query_map(params![novel_id], Self::row_to_wiki_entry)
        }
        .map_err(|e| AppError::internal(format!("Failed to query wiki entries: {}", e)))?;
        
        let entries = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(entries)
    }

    pub fn get_wiki_entry(&self, entry_id: &str) -> Result<Option<crate::domain::wiki::WikiEntry>, AppError> {
        let result = self.conn.query_row(
            "SELECT * FROM wiki_entries WHERE id = ?1",
            params![entry_id],
            Self::row_to_wiki_entry,
        ).optional()
        .map_err(|e| AppError::internal(format!("Failed to get wiki entry: {}", e)))?;
        Ok(result)
    }

    pub fn create_wiki_entry(&self, req: &crate::domain::wiki::CreateWikiEntryRequest) -> Result<crate::domain::wiki::WikiEntry, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());
        let importance = req.importance.unwrap_or(0);
        let word_count = count_words(&req.content);
        
        self.conn.execute(
            "INSERT INTO wiki_entries (id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'manual', ?6, ?7, ?8, ?9, ?10, ?10)",
            params![id, req.novel_id, req.title, req.content, req.category.to_string(), req.source_chapter.map(|n| n as i64), tags_json, importance as i64, word_count as i64, now],
        ).map_err(|e| AppError::internal(format!("Failed to create wiki entry: {}", e)))?;
        
        Ok(crate::domain::wiki::WikiEntry {
            id, novel_id: req.novel_id.clone(), title: req.title.clone(), content: req.content.clone(),
            category: req.category.clone(), source_type: crate::domain::wiki::WikiSourceType::Manual,
            source_chapter: req.source_chapter, tags: req.tags.clone(), importance,
            word_count, created_at: now.clone(), updated_at: now,
        })
    }

    pub fn update_wiki_entry(
        &self,
        entry_id: &str,
        req: &crate::domain::wiki::UpdateWikiEntryRequest,
    ) -> Result<crate::domain::wiki::WikiEntry, AppError> {
        let existing = self.get_wiki_entry(entry_id)?.ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
        let now = Utc::now().to_rfc3339();
        
        let title = req.title.clone().unwrap_or(existing.title);
        let content = req.content.clone().unwrap_or(existing.content);
        let category = req.category.clone().unwrap_or(existing.category);
        let tags = req.tags.clone().unwrap_or(existing.tags);
        let importance = req.importance.unwrap_or(existing.importance);
        let word_count = count_words(&content);
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        
        self.conn.execute(
            "UPDATE wiki_entries SET title = ?1, content = ?2, category = ?3, tags = ?4, importance = ?5, word_count = ?6, updated_at = ?7 WHERE id = ?8",
            params![title, content, category.to_string(), tags_json, importance as i64, word_count as i64, now, entry_id],
        ).map_err(|e| AppError::internal(format!("Failed to update wiki entry: {}", e)))?;
        
        Ok(crate::domain::wiki::WikiEntry {
            id: existing.id, novel_id: existing.novel_id, title, content, category,
            source_type: existing.source_type, source_chapter: existing.source_chapter,
            tags, importance, word_count, created_at: existing.created_at, updated_at: now,
        })
    }

    pub fn delete_wiki_entry(&self, entry_id: &str) -> Result<bool, AppError> {
        let rows = self.conn.execute("DELETE FROM wiki_entries WHERE id = ?1", params![entry_id])
            .map_err(|e| AppError::internal(format!("Failed to delete wiki entry: {}", e)))?;
        Ok(rows > 0)
    }

    pub fn search_wiki_entries(
        &self,
        novel_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        let limit_val = limit.unwrap_or(20);
        let sql = "SELECT * FROM wiki_entries WHERE novel_id = ?1 AND (title LIKE ?2 OR content LIKE ?2) ORDER BY importance DESC, updated_at DESC LIMIT ?3";
        let search_pattern = format!("%{}%", query);
        
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare wiki search: {}", e)))?;
        let rows = stmt.query_map(params![novel_id, search_pattern, limit_val as i64], Self::row_to_wiki_entry)
            .map_err(|e| AppError::internal(format!("Failed to search wiki entries: {}", e)))?;
        
        let entries = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(entries)
    }

    pub fn get_wiki_context_for_chapter(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        // Get entries with: source_chapter matches, or high importance
        let sql = "SELECT * FROM wiki_entries WHERE novel_id = ?1 AND (source_chapter = ?2 OR importance >= 5) ORDER BY importance DESC, updated_at DESC";
        
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare wiki context query: {}", e)))?;
        let rows = stmt.query_map(params![novel_id, chapter_number as i64], Self::row_to_wiki_entry)
            .map_err(|e| AppError::internal(format!("Failed to query wiki context: {}", e)))?;
        
        let entries = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(entries)
    }

    pub fn get_wiki_graph_view(
        &self,
        novel_id: &str,
        filter_category: Option<&crate::domain::wiki::WikiCategory>,
        min_importance: Option<u32>,
    ) -> Result<crate::domain::wiki::WikiGraphView, AppError> {
        // Get nodes (entries)
        let min_imp = min_importance.unwrap_or(0);
        
        // Build SQL and params based on filter
        let (node_sql, params_vec): (&str, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(cat) = filter_category {
            ("SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ?1 AND category = ?2 AND importance >= ?3",
             vec![Box::new(novel_id.to_string()), Box::new(cat.to_string()), Box::new(min_imp as i64)])
        } else {
            ("SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ?1 AND importance >= ?2",
             vec![Box::new(novel_id.to_string()), Box::new(min_imp as i64)])
        };        
        
        let mut stmt = self.conn.prepare(node_sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare wiki graph nodes: {}", e)))?;
        
        let node_rows = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| Ok(crate::domain::wiki::WikiGraphNode {
            id: row.get(0)?, title: row.get(1)?, category: row.get(2)?, importance: row.get::<_, i64>(3)? as u32,
        })).map_err(|e| AppError::internal(format!("Failed to query wiki nodes: {}", e)))?;
        
        let nodes: Vec<_> = node_rows.filter_map(|r| r.ok()).collect();
        let node_ids: Vec<_> = nodes.iter().map(|n| n.id.clone()).collect();
        
        // Get edges (links between nodes in our filtered set)
        let edge_sql = "SELECT source_entry_id, target_entry_id, relation_type, weight FROM wiki_entity_links WHERE novel_id = ?1";
        let mut stmt = self.conn.prepare(edge_sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare wiki graph edges: {}", e)))?;
        let edge_rows = stmt.query_map(params![novel_id], |row| Ok(crate::domain::wiki::WikiGraphEdge {
            source: row.get(0)?, target: row.get(1)?, relation: row.get(2)?, weight: row.get::<_, i64>(3)? as u32,
        })).map_err(|e| AppError::internal(format!("Failed to query wiki edges: {}", e)))?;
        
        // Filter edges to only those connecting our nodes
        let edges: Vec<_> = edge_rows.filter_map(|r| r.ok())
            .filter(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target))
            .collect();
        
        Ok(crate::domain::wiki::WikiGraphView { nodes, edges })
    }

    // ═══════════════════════════════════════════════════════════
    // Wiki Entity Links
    // ═══════════════════════════════════════════════════════════

    fn row_to_wiki_link(row: &Row) -> rusqlite::Result<crate::domain::wiki::WikiEntityLink> {
        Ok(crate::domain::wiki::WikiEntityLink {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            source_entry_id: row.get(2)?,
            target_entry_id: row.get(3)?,
            relation_type: row.get(4)?,
            relation_desc: row.get(5)?,
            weight: row.get::<_, i64>(6)? as u32,
            source_chapter: row.get::<_, Option<i64>>(7)?.map(|n| n as u32),
            created_at: row.get(8)?,
        })
    }

    pub fn create_wiki_link(&self, req: &crate::domain::wiki::CreateWikiLinkRequest) -> Result<crate::domain::wiki::WikiEntityLink, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let weight = req.weight.unwrap_or(1);
        
        self.conn.execute(
            "INSERT INTO wiki_entity_links (id, novel_id, source_entry_id, target_entry_id, relation_type, relation_desc, weight, source_chapter, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, req.novel_id, req.source_entry_id, req.target_entry_id, req.relation_type, req.relation_desc, weight as i64, req.source_chapter.map(|n| n as i64), now],
        ).map_err(|e| AppError::internal(format!("Failed to create wiki link: {}", e)))?;
        
        Ok(crate::domain::wiki::WikiEntityLink {
            id, novel_id: req.novel_id.clone(), source_entry_id: req.source_entry_id.clone(),
            target_entry_id: req.target_entry_id.clone(), relation_type: req.relation_type.clone(),
            relation_desc: req.relation_desc.clone(), weight, source_chapter: req.source_chapter,
            created_at: now,
        })
    }

    pub fn delete_wiki_link(&self, link_id: &str) -> Result<bool, AppError> {
        let rows = self.conn.execute("DELETE FROM wiki_entity_links WHERE id = ?1", params![link_id])
            .map_err(|e| AppError::internal(format!("Failed to delete wiki link: {}", e)))?;
        Ok(rows > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Chapter Versions
    // ═══════════════════════════════════════════════════════════

    fn row_to_chapter_version(row: &Row) -> rusqlite::Result<crate::domain::version::ChapterVersion> {
        let mode_str: String = row.get(8)?;
        Ok(crate::domain::version::ChapterVersion {
            id: row.get(0)?,
            novel_id: row.get(1)?,
            chapter_number: row.get::<_, i64>(2)? as u32,
            version_number: row.get::<_, i64>(3)? as u32,
            content: row.get(4)?,
            content_hash: row.get(5)?,
            word_count: row.get::<_, i64>(6)? as u32,
            revision_reason: row.get(7)?,
            revision_mode: mode_str.parse().unwrap_or(crate::domain::version::RevisionMode::Auto),
            created_at: row.get(9)?,
        })
    }

    pub fn list_chapter_versions(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<crate::domain::version::ChapterVersion>, AppError> {
        let sql = "SELECT * FROM chapter_versions WHERE novel_id = ?1 AND chapter_number = ?2 ORDER BY version_number DESC";
        let mut stmt = self.conn.prepare(sql)
            .map_err(|e| AppError::internal(format!("Failed to prepare version list: {}", e)))?;
        let rows = stmt.query_map(params![novel_id, chapter_number as i64], Self::row_to_chapter_version)
            .map_err(|e| AppError::internal(format!("Failed to query versions: {}", e)))?;
        let versions = rows.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(versions)
    }

    pub fn get_chapter_version(&self, version_id: &str) -> Result<Option<crate::domain::version::ChapterVersion>, AppError> {
        let result = self.conn.query_row(
            "SELECT * FROM chapter_versions WHERE id = ?1",
            params![version_id],
            Self::row_to_chapter_version,
        ).optional()
        .map_err(|e| AppError::internal(format!("Failed to get version: {}", e)))?;
        Ok(result)
    }

    pub fn get_latest_chapter_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<crate::domain::version::ChapterVersion>, AppError> {
        let sql = "SELECT * FROM chapter_versions WHERE novel_id = ?1 AND chapter_number = ?2 ORDER BY version_number DESC LIMIT 1";
        let result = self.conn.query_row(sql, params![novel_id, chapter_number as i64], Self::row_to_chapter_version)
            .optional()
        .map_err(|e| AppError::internal(format!("Failed to get latest version: {}", e)))?;
        Ok(result)
    }

    pub fn get_next_version_number(&self, novel_id: &str, chapter_number: u32) -> Result<u32, AppError> {
        let max: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version_number), 0) FROM chapter_versions WHERE novel_id = ?1 AND chapter_number = ?2",
            params![novel_id, chapter_number as i64],
            |row| row.get(0),
        ).map_err(|e| AppError::internal(format!("Failed to get max version number: {}", e)))?;
        Ok((max + 1) as u32)
    }

    pub fn create_chapter_version(
        &self,
        req: &crate::domain::version::CreateVersionRequest,
        version_number: u32,
        content_hash: &str,
        word_count: u32,
    ) -> Result<crate::domain::version::ChapterVersion, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO chapter_versions (id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, req.novel_id, req.chapter_number as i64, version_number as i64, req.content, content_hash, word_count as i64, req.revision_reason, req.revision_mode.to_string(), now],
        ).map_err(|e| AppError::internal(format!("Failed to create version: {}", e)))?;
        
        Ok(crate::domain::version::ChapterVersion {
            id, novel_id: req.novel_id.clone(), chapter_number: req.chapter_number,
            version_number, content: req.content.clone(), content_hash: content_hash.to_string(),
            word_count, revision_reason: req.revision_reason.clone(), revision_mode: req.revision_mode.clone(),
            created_at: now,
        })
    }
}

/// Count words in content (approximation)
fn count_words(content: &str) -> u32 {
    // Simplified word counting: Chinese chars + English words
    let chinese_chars = content.chars().filter(|c| !c.is_ascii()).count() as u32;
    let english_words = content.split_whitespace().count() as u32;
    chinese_chars + english_words
}
