use rusqlite::{Connection, params, Row};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

use super::models::*;
use crate::errors::AppError;

const SCHEMA_SQL: &str = include_str!("sql/schema.sql");
const FEEDBACK_SCHEMA_SQL: &str = include_str!("sql/feedback_schema.sql");

/// Migrations by version number. Each entry: (version, sql).
/// Version 1 is implicit (initial schema.sql). Version 2+ are incremental.
const MIGRATIONS: &[(i64, &str)] = &[
    (2, "ALTER TABLE agents ADD COLUMN updated_at TEXT NOT NULL DEFAULT ''"),
];

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
        db.init_with_schema(SCHEMA_SQL)?;
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
        db.init_with_schema(FEEDBACK_SCHEMA_SQL)?;
        Ok(db)
    }

    fn init_with_schema(&self, schema_sql: &str) -> Result<(), AppError> {
        self.conn.execute_batch(schema_sql)
            .map_err(|e| AppError::internal(format!("Failed to init schema: {}", e)))?;
        self.run_migrations()?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), AppError> {
        let current: i64 = self.conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        for &(version, sql) in MIGRATIONS {
            if version <= current {
                continue;
            }
            let tx = self.conn.unchecked_transaction()
                .map_err(|e| AppError::internal(format!("Failed to start migration tx: {}", e)))?;
            tx.execute_batch(sql)
                .map_err(|e| AppError::db_migration(format!("Migration v{} failed: {}", version, e)))?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, Utc::now().to_rfc3339()],
            )
            .map_err(|e| AppError::db_migration(format!("Failed to record migration v{}: {}", version, e)))?;
            tx.commit()
                .map_err(|e| AppError::db_migration(format!("Failed to commit migration v{}: {}", version, e)))?;
        }
        Ok(())
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
}
