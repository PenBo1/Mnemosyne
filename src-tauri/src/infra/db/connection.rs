use sqlx::{SqlitePool, Row};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

use super::models::*;
use crate::errors::AppError;

const SCHEMA_SQL: &str = include_str!("sql/schema.sql");
const FEEDBACK_SCHEMA_SQL: &str = include_str!("sql/feedback_schema.sql");

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

fn db_err(e: sqlx::Error) -> AppError {
    AppError::internal(format!("Database error: {}", e))
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create db directory: {}", e)))?;
        let url = format!("sqlite:{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&url).await.map_err(db_err)?;
        sqlx::raw_sql(SCHEMA_SQL).execute(&pool).await.map_err(db_err)?;
        Ok(Self { pool })
    }

    pub async fn new_feedback(db_path: &str) -> Result<Self, AppError> {
        let dir = Path::new(db_path).parent()
            .ok_or_else(|| AppError::internal("Invalid feedback database path"))?;
        std::fs::create_dir_all(dir)
            .map_err(|e| AppError::internal(format!("Failed to create feedback db directory: {}", e)))?;
        let url = format!("sqlite:{}?mode=rwc", db_path);
        let pool = SqlitePool::connect(&url).await.map_err(db_err)?;
        sqlx::raw_sql(FEEDBACK_SCHEMA_SQL).execute(&pool).await.map_err(db_err)?;
        Ok(Self { pool })
    }

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

    pub async fn delete_workspace(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Novels
    // ═══════════════════════════════════════════════════════════

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

    async fn get_novel_by_id_raw(&self, id: &str) -> Result<Option<Novel>, AppError> {
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

    pub async fn get_novel_by_id(&self, id: &str) -> Result<Option<Novel>, AppError> {
        self.get_novel_by_id_raw(id).await
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

    // ═══════════════════════════════════════════════════════════
    // Chapters
    // ═══════════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════════
    // Prompts
    // ═══════════════════════════════════════════════════════════

    pub async fn create_prompt(&self, req: CreatePromptRequest) -> Result<Prompt, AppError> {
        Self::validate_name(&req.name, "Prompt name")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags = serde_json::to_string(&req.tags).unwrap_or_default();
        sqlx::query(
            "INSERT INTO prompts (id, name, content, category, tags, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.name).bind(&req.content).bind(&req.category)
        .bind(&tags).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(Prompt { id, name: req.name, content: req.content, category: req.category, tags: req.tags, created_at: now.clone(), updated_at: now })
    }

    pub async fn list_prompts(&self, category: Option<&str>) -> Result<Vec<Prompt>, AppError> {
        if let Some(cat) = category {
            sqlx::query(
                "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE category = ? ORDER BY updated_at DESC"
            )
            .bind(cat)
            .map(|row: sqlx::sqlite::SqliteRow| {
                let tags_str: String = row.get(4);
                Prompt {
                    id: row.get(0), name: row.get(1), content: row.get(2),
                    category: row.get(3), tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    created_at: row.get(5), updated_at: row.get(6),
                }
            })
            .fetch_all(&self.pool).await.map_err(db_err)
        } else {
            sqlx::query(
                "SELECT id, name, content, category, tags, created_at, updated_at FROM prompts ORDER BY updated_at DESC"
            )
            .map(|row: sqlx::sqlite::SqliteRow| {
                let tags_str: String = row.get(4);
                Prompt {
                    id: row.get(0), name: row.get(1), content: row.get(2),
                    category: row.get(3), tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    created_at: row.get(5), updated_at: row.get(6),
                }
            })
            .fetch_all(&self.pool).await.map_err(db_err)
        }
    }

    pub async fn get_prompt(&self, id: &str) -> Result<Option<Prompt>, AppError> {
        sqlx::query("SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE id = ?")
            .bind(id)
            .map(|row: sqlx::sqlite::SqliteRow| {
                let tags_str: String = row.get(4);
                Prompt {
                    id: row.get(0), name: row.get(1), content: row.get(2),
                    category: row.get(3), tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    created_at: row.get(5), updated_at: row.get(6),
                }
            })
            .fetch_optional(&self.pool).await.map_err(db_err)
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
        let tags = req.tags.map(|t| serde_json::to_string(&t).unwrap_or_default())
            .unwrap_or_else(|| serde_json::to_string(&existing.tags).unwrap_or_default());
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

    // ═══════════════════════════════════════════════════════════
    // Trends
    // ═══════════════════════════════════════════════════════════

    pub async fn create_trend(&self, keyword: &str, platform: &str, score: f64, metadata: serde_json::Value) -> Result<Trend, AppError> {
        Self::validate_name(keyword, "Trend keyword")?;
        Self::validate_name(platform, "Trend platform")?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let meta_str = serde_json::to_string(&metadata).unwrap_or_default();
        sqlx::query(
            "INSERT INTO trends (id, keyword, platform, score, metadata, scanned_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(keyword).bind(platform).bind(score).bind(&meta_str).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(Trend { id, keyword: keyword.to_string(), platform: platform.to_string(), score, metadata, scanned_at: now })
    }

    pub async fn list_trends(&self, platform: Option<&str>, limit: Option<i64>) -> Result<Vec<Trend>, AppError> {
        let limit = limit.unwrap_or(100).max(1).min(1000);
        let map_trend = |row: sqlx::sqlite::SqliteRow| -> Trend {
            let meta_str: String = row.get(4);
            Trend {
                id: row.get(0), keyword: row.get(1), platform: row.get(2),
                score: row.get(3), metadata: serde_json::from_str(&meta_str).unwrap_or_default(),
                scanned_at: row.get(5),
            }
        };
        if let Some(p) = platform {
            sqlx::query(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends WHERE platform = ? ORDER BY scanned_at DESC LIMIT ?"
            )
            .bind(p).bind(limit)
            .map(map_trend)
            .fetch_all(&self.pool).await.map_err(db_err)
        } else {
            sqlx::query(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends ORDER BY scanned_at DESC LIMIT ?"
            )
            .bind(limit)
            .map(map_trend)
            .fetch_all(&self.pool).await.map_err(db_err)
        }
    }

    pub async fn delete_trend(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM trends WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Radar: Market Intelligence
    // ═══════════════════════════════════════════════════════════

    pub async fn create_radar_scan(
        &self,
        market_summary: &str,
        recommendations: &[RadarRecommendation],
        raw_rankings: &[PlatformRankings],
    ) -> Result<RadarScan, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let recs_json = serde_json::to_string(recommendations).unwrap_or_default();
        let raw_json = serde_json::to_string(raw_rankings).unwrap_or_default();
        sqlx::query(
            "INSERT INTO radar_scans (id, market_summary, recommendations_json, raw_rankings_json, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(market_summary).bind(&recs_json).bind(&raw_json).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(RadarScan {
            id,
            market_summary: market_summary.to_string(),
            recommendations: recommendations.to_vec(),
            raw_rankings: raw_rankings.to_vec(),
            created_at: now,
        })
    }

    pub async fn list_radar_scans(&self, limit: Option<i64>) -> Result<Vec<RadarScan>, AppError> {
        let limit = limit.unwrap_or(50).max(1).min(500);
        sqlx::query(
            "SELECT id, market_summary, recommendations_json, raw_rankings_json, created_at FROM radar_scans ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .map(|row: sqlx::sqlite::SqliteRow| {
            let recs_str: String = row.get(2);
            let raw_str: String = row.get(3);
            RadarScan {
                id: row.get(0), market_summary: row.get(1),
                recommendations: serde_json::from_str(&recs_str).unwrap_or_default(),
                raw_rankings: serde_json::from_str(&raw_str).unwrap_or_default(),
                created_at: row.get(4),
            }
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn delete_radar_scan(&self, id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM radar_scans WHERE id = ?")
            .bind(id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Stats
    // ═══════════════════════════════════════════════════════════

    pub async fn get_stats(&self) -> Result<serde_json::Value, AppError> {
        let prompt_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM prompts")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let novel_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM novels")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let trend_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trends")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        let total_words: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(word_count), 0) FROM novels")
            .fetch_one(&self.pool).await.map_err(db_err)?;
        Ok(serde_json::json!({ "promptCount": prompt_count, "novelCount": novel_count, "trendCount": trend_count, "totalWords": total_words }))
    }

    pub async fn get_daily_activity(&self) -> Result<serde_json::Value, AppError> {
        let chat_activity: Vec<(String, i64)> = sqlx::query_as(
            "SELECT DATE(created_at) as date, COUNT(*) FROM sessions WHERE created_at >= DATE('now', '-1 year') GROUP BY DATE(created_at) ORDER BY date"
        )
        .fetch_all(&self.pool).await.map_err(db_err)?;

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

    fn map_wiki_entry(row: sqlx::sqlite::SqliteRow) -> crate::domain::wiki::WikiEntry {
        let category_str: String = row.get(4);
        let source_type_str: String = row.get(5);
        let tags_json: String = row.get(7);
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        crate::domain::wiki::WikiEntry {
            id: row.get(0),
            novel_id: row.get(1),
            title: row.get(2),
            content: row.get(3),
            category: category_str.parse().unwrap_or(crate::domain::wiki::WikiCategory::General),
            source_type: source_type_str.parse().unwrap_or(crate::domain::wiki::WikiSourceType::Manual),
            source_chapter: row.try_get::<Option<i64>, usize>(6).unwrap_or(None).map(|n| n as u32),
            tags,
            importance: row.get::<i64, usize>(8) as u32,
            word_count: row.get::<i64, usize>(9) as u32,
            created_at: row.get(10),
            updated_at: row.get(11),
        }
    }

    pub async fn list_wiki_entries(
        &self,
        novel_id: &str,
        category: Option<&crate::domain::wiki::WikiCategory>,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        if let Some(cat) = category {
            sqlx::query(
                "SELECT * FROM wiki_entries WHERE novel_id = ? AND category = ? ORDER BY importance DESC, updated_at DESC"
            )
            .bind(novel_id).bind(cat.to_string())
            .map(Self::map_wiki_entry)
            .fetch_all(&self.pool).await.map_err(db_err)
        } else {
            sqlx::query(
                "SELECT * FROM wiki_entries WHERE novel_id = ? ORDER BY importance DESC, updated_at DESC"
            )
            .bind(novel_id)
            .map(Self::map_wiki_entry)
            .fetch_all(&self.pool).await.map_err(db_err)
        }
    }

    pub async fn get_wiki_entry(&self, entry_id: &str) -> Result<Option<crate::domain::wiki::WikiEntry>, AppError> {
        sqlx::query("SELECT * FROM wiki_entries WHERE id = ?")
            .bind(entry_id)
            .map(Self::map_wiki_entry)
            .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn create_wiki_entry(&self, req: &crate::domain::wiki::CreateWikiEntryRequest) -> Result<crate::domain::wiki::WikiEntry, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());
        let importance = req.importance.unwrap_or(0);
        let word_count = count_words(&req.content);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        sqlx::query(
            "INSERT INTO wiki_entries (id, novel_id, title, content, category, source_type, source_chapter, tags, importance, word_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'manual', ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(&req.title).bind(&req.content)
        .bind(req.category.to_string()).bind(source_chapter_i64).bind(&tags_json)
        .bind(importance as i64).bind(word_count as i64).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(crate::domain::wiki::WikiEntry {
            id, novel_id: req.novel_id.clone(), title: req.title.clone(), content: req.content.clone(),
            category: req.category.clone(), source_type: crate::domain::wiki::WikiSourceType::Manual,
            source_chapter: req.source_chapter, tags: req.tags.clone(), importance,
            word_count, created_at: now.clone(), updated_at: now,
        })
    }

    pub async fn update_wiki_entry(
        &self,
        entry_id: &str,
        req: &crate::domain::wiki::UpdateWikiEntryRequest,
    ) -> Result<crate::domain::wiki::WikiEntry, AppError> {
        let existing = self.get_wiki_entry(entry_id).await?.ok_or_else(|| AppError::not_found("Wiki entry not found"))?;
        let now = Utc::now().to_rfc3339();

        let title = req.title.clone().unwrap_or(existing.title);
        let content = req.content.clone().unwrap_or(existing.content);
        let category = req.category.clone().unwrap_or(existing.category);
        let tags = req.tags.clone().unwrap_or(existing.tags);
        let importance = req.importance.unwrap_or(existing.importance);
        let word_count = count_words(&content);
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "UPDATE wiki_entries SET title = ?, content = ?, category = ?, tags = ?, importance = ?, word_count = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&title).bind(&content).bind(category.to_string()).bind(&tags_json)
        .bind(importance as i64).bind(word_count as i64).bind(&now).bind(entry_id)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(crate::domain::wiki::WikiEntry {
            id: existing.id, novel_id: existing.novel_id, title, content, category,
            source_type: existing.source_type, source_chapter: existing.source_chapter,
            tags, importance, word_count, created_at: existing.created_at, updated_at: now,
        })
    }

    pub async fn delete_wiki_entry(&self, entry_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM wiki_entries WHERE id = ?")
            .bind(entry_id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn search_wiki_entries(
        &self,
        novel_id: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        let limit_val = limit.unwrap_or(20);
        let search_pattern = format!("%{}%", query);
        sqlx::query(
            "SELECT * FROM wiki_entries WHERE novel_id = ? AND (title LIKE ? OR content LIKE ?) ORDER BY importance DESC, updated_at DESC LIMIT ?"
        )
        .bind(novel_id).bind(&search_pattern).bind(&search_pattern).bind(limit_val as i64)
        .map(Self::map_wiki_entry)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_wiki_context_for_chapter(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<crate::domain::wiki::WikiEntry>, AppError> {
        sqlx::query(
            "SELECT * FROM wiki_entries WHERE novel_id = ? AND (source_chapter = ? OR importance >= 5) ORDER BY importance DESC, updated_at DESC"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .map(Self::map_wiki_entry)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_wiki_graph_view(
        &self,
        novel_id: &str,
        filter_category: Option<&crate::domain::wiki::WikiCategory>,
        min_importance: Option<u32>,
    ) -> Result<crate::domain::wiki::WikiGraphView, AppError> {
        let min_imp = min_importance.unwrap_or(0);

        let nodes: Vec<crate::domain::wiki::WikiGraphNode> = if let Some(cat) = filter_category {
            sqlx::query(
                "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND category = ? AND importance >= ?"
            )
            .bind(novel_id).bind(cat.to_string()).bind(min_imp as i64)
            .map(|row: sqlx::sqlite::SqliteRow| crate::domain::wiki::WikiGraphNode {
                id: row.get(0), title: row.get(1), category: row.get(2),
                importance: row.get::<i64, usize>(3) as u32,
            })
            .fetch_all(&self.pool).await.map_err(db_err)?
        } else {
            sqlx::query(
                "SELECT id, title, category, importance FROM wiki_entries WHERE novel_id = ? AND importance >= ?"
            )
            .bind(novel_id).bind(min_imp as i64)
            .map(|row: sqlx::sqlite::SqliteRow| crate::domain::wiki::WikiGraphNode {
                id: row.get(0), title: row.get(1), category: row.get(2),
                importance: row.get::<i64, usize>(3) as u32,
            })
            .fetch_all(&self.pool).await.map_err(db_err)?
        };

        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        let all_edges: Vec<crate::domain::wiki::WikiGraphEdge> = sqlx::query(
            "SELECT source_entry_id, target_entry_id, relation_type, weight FROM wiki_entity_links WHERE novel_id = ?"
        )
        .bind(novel_id)
        .map(|row: sqlx::sqlite::SqliteRow| crate::domain::wiki::WikiGraphEdge {
            source: row.get(0), target: row.get(1), relation: row.get(2),
            weight: row.get::<i64, usize>(3) as u32,
        })
        .fetch_all(&self.pool).await.map_err(db_err)?;

        let edges: Vec<_> = all_edges.into_iter()
            .filter(|e| node_ids.contains(&e.source) && node_ids.contains(&e.target))
            .collect();

        Ok(crate::domain::wiki::WikiGraphView { nodes, edges })
    }

    // ═══════════════════════════════════════════════════════════
    // Wiki Entity Links
    // ═══════════════════════════════════════════════════════════

    pub async fn create_wiki_link(&self, req: &crate::domain::wiki::CreateWikiLinkRequest) -> Result<crate::domain::wiki::WikiEntityLink, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let weight = req.weight.unwrap_or(1);
        let source_chapter_i64 = req.source_chapter.map(|n| n as i64);

        sqlx::query(
            "INSERT INTO wiki_entity_links (id, novel_id, source_entry_id, target_entry_id, relation_type, relation_desc, weight, source_chapter, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(&req.source_entry_id).bind(&req.target_entry_id)
        .bind(&req.relation_type).bind(&req.relation_desc).bind(weight as i64)
        .bind(source_chapter_i64).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(crate::domain::wiki::WikiEntityLink {
            id, novel_id: req.novel_id.clone(), source_entry_id: req.source_entry_id.clone(),
            target_entry_id: req.target_entry_id.clone(), relation_type: req.relation_type.clone(),
            relation_desc: req.relation_desc.clone(), weight, source_chapter: req.source_chapter,
            created_at: now,
        })
    }

    pub async fn delete_wiki_link(&self, link_id: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM wiki_entity_links WHERE id = ?")
            .bind(link_id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // Chapter Versions
    // ═══════════════════════════════════════════════════════════

    fn map_chapter_version(row: sqlx::sqlite::SqliteRow) -> crate::domain::version::ChapterVersion {
        let mode_str: String = row.get(8);
        crate::domain::version::ChapterVersion {
            id: row.get(0),
            novel_id: row.get(1),
            chapter_number: row.get::<i64, usize>(2) as u32,
            version_number: row.get::<i64, usize>(3) as u32,
            content: row.get(4),
            content_hash: row.get(5),
            word_count: row.get::<i64, usize>(6) as u32,
            revision_reason: row.get(7),
            revision_mode: mode_str.parse().unwrap_or(crate::domain::version::RevisionMode::Auto),
            created_at: row.get(9),
        }
    }

    pub async fn list_chapter_versions(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Vec<crate::domain::version::ChapterVersion>, AppError> {
        sqlx::query(
            "SELECT * FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .map(Self::map_chapter_version)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_chapter_version(&self, version_id: &str) -> Result<Option<crate::domain::version::ChapterVersion>, AppError> {
        sqlx::query("SELECT * FROM chapter_versions WHERE id = ?")
            .bind(version_id)
            .map(Self::map_chapter_version)
            .fetch_optional(&self.pool).await.map_err(db_err)
    }

    pub async fn get_latest_chapter_version(
        &self,
        novel_id: &str,
        chapter_number: u32,
    ) -> Result<Option<crate::domain::version::ChapterVersion>, AppError> {
        sqlx::query(
            "SELECT * FROM chapter_versions WHERE novel_id = ? AND chapter_number = ? ORDER BY version_number DESC LIMIT 1"
        )
        .bind(novel_id).bind(chapter_number as i64)
        .map(Self::map_chapter_version)
        .fetch_optional(&self.pool).await.map_err(db_err)
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
        req: &crate::domain::version::CreateVersionRequest,
        version_number: u32,
        content_hash: &str,
        word_count: u32,
    ) -> Result<crate::domain::version::ChapterVersion, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO chapter_versions (id, novel_id, chapter_number, version_number, content, content_hash, word_count, revision_reason, revision_mode, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&req.novel_id).bind(req.chapter_number as i64).bind(version_number as i64)
        .bind(&req.content).bind(content_hash).bind(word_count as i64)
        .bind(&req.revision_reason).bind(req.revision_mode.to_string()).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(crate::domain::version::ChapterVersion {
            id, novel_id: req.novel_id.clone(), chapter_number: req.chapter_number,
            version_number, content: req.content.clone(), content_hash: content_hash.to_string(),
            word_count, revision_reason: req.revision_reason.clone(), revision_mode: req.revision_mode.clone(),
            created_at: now,
        })
    }
}

/// Count words in content — delegates to shared implementation.
fn count_words(content: &str) -> u32 {
    crate::domain::utils::text_utils::count_words(content)
}

// ═══════════════════════════════════════════════════════════
// Kanban
// ═══════════════════════════════════════════════════════════

fn map_kanban_task(row: sqlx::sqlite::SqliteRow) -> KanbanTask {
    let tags_json: String = row.get(10);
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    KanbanTask {
        id: row.get(0),
        novel_id: row.get(1),
        title: row.get(2),
        description: row.get(3),
        status: row.get(4),
        priority: row.get(5),
        assigned_agent: row.get(6),
        chapter_id: row.get(7),
        parent_task_id: row.get(8),
        tags,
        sort_order: row.get(11),
        due_date: row.get(12),
        created_at: row.get(13),
        updated_at: row.get(14),
    }
}

fn map_kanban_column(row: sqlx::sqlite::SqliteRow) -> KanbanColumn {
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
        let tags = serde_json::to_string(&req.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
        let description = req.description.unwrap_or_default();

        sqlx::query(
            "INSERT INTO kanban_tasks (id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(&req.title).bind(&description).bind(&status).bind(&priority)
        .bind(&req.assigned_agent).bind(&req.chapter_id).bind(&req.parent_task_id)
        .bind(&tags).bind(&req.due_date).bind(&now).bind(&now)
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
            tags: serde_json::from_str(&tags).unwrap_or_default(),
            sort_order: 0,
            due_date: req.due_date,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_kanban_tasks(&self, novel_id: &str, status_filter: Option<&str>) -> Result<Vec<KanbanTask>, AppError> {
        if let Some(s) = status_filter {
            sqlx::query(
                "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE novel_id = ? AND status = ? ORDER BY sort_order, created_at"
            )
            .bind(novel_id).bind(s)
            .map(map_kanban_task)
            .fetch_all(&self.pool).await.map_err(db_err)
        } else {
            sqlx::query(
                "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE novel_id = ? ORDER BY sort_order, created_at"
            )
            .bind(novel_id)
            .map(map_kanban_task)
            .fetch_all(&self.pool).await.map_err(db_err)
        }
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
        let tags = req.tags.map(|t| serde_json::to_string(&t).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| serde_json::to_string(&existing.tags).unwrap_or_else(|_| "[]".to_string()));

        sqlx::query(
            "UPDATE kanban_tasks SET title = ?, description = ?, status = ?, priority = ?, assigned_agent = ?, chapter_id = ?, parent_task_id = ?, sort_order = ?, due_date = ?, tags = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&title).bind(&description).bind(&status).bind(&priority)
        .bind(&req.assigned_agent).bind(&req.chapter_id).bind(&req.parent_task_id)
        .bind(sort_order).bind(&req.due_date).bind(&tags).bind(&now).bind(task_id)
        .execute(&self.pool).await.map_err(db_err)?;

        self.get_kanban_task_by_id(task_id).await?
            .ok_or_else(|| AppError::not_found("Kanban task"))
    }

    pub async fn get_kanban_task_by_id(&self, task_id: &str) -> Result<Option<KanbanTask>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, title, description, status, priority, assigned_agent, chapter_id, parent_task_id, tags, sort_order, due_date, created_at, updated_at FROM kanban_tasks WHERE id = ?"
        )
        .bind(task_id)
        .map(map_kanban_task)
        .fetch_optional(&self.pool).await.map_err(db_err)
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

    pub async fn reorder_kanban_tasks(&self, task_ids: &[String]) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        for (i, id) in task_ids.iter().enumerate() {
            sqlx::query("UPDATE kanban_tasks SET sort_order = ?, updated_at = ? WHERE id = ?")
                .bind(i as i32).bind(&now).bind(id)
                .execute(&self.pool).await.map_err(db_err)?;
        }
        Ok(())
    }

    pub async fn get_kanban_columns(&self, novel_id: &str) -> Result<Vec<KanbanColumn>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, name, status_key, color, sort_order, wip_limit, created_at FROM kanban_columns WHERE novel_id = ? ORDER BY sort_order"
        )
        .bind(novel_id)
        .map(map_kanban_column)
        .fetch_all(&self.pool).await.map_err(db_err)
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
        sqlx::query(
            "SELECT id, novel_id, name, status_key, color, sort_order, wip_limit, created_at FROM kanban_columns WHERE id = ?"
        )
        .bind(column_id)
        .map(map_kanban_column)
        .fetch_optional(&self.pool).await.map_err(db_err)
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

// ═══════════════════════════════════════════════════════════
// Loop Engineering
// ═══════════════════════════════════════════════════════════

fn map_loop_state(row: sqlx::sqlite::SqliteRow) -> LoopState {
    let payload_json: String = row.get(5);
    let config_json: String = row.get(6);
    let last_result_json: Option<String> = row.get(10);
    LoopState {
        id: row.get(0),
        novel_id: row.get(1),
        pattern_id: row.get(2),
        status: row.get(3),
        readiness_level: row.get(4),
        state_payload: serde_json::from_str(&payload_json).unwrap_or(serde_json::json!({})),
        config: serde_json::from_str(&config_json).unwrap_or(serde_json::json!({})),
        token_usage_today: row.get(7),
        token_cap_daily: row.get(8),
        last_run_at: row.get(9),
        last_run_result: last_result_json.and_then(|j| serde_json::from_str(&j).ok()),
        created_at: row.get(11),
        updated_at: row.get(12),
    }
}

fn map_loop_run_log(row: sqlx::sqlite::SqliteRow) -> LoopRunLog {
    let phase_json: String = row.get(4);
    let findings_json: String = row.get(7);
    let actions_json: String = row.get(8);
    let escalations_json: String = row.get(9);
    LoopRunLog {
        id: row.get(0),
        loop_state_id: row.get(1),
        pattern_id: row.get(2),
        status: row.get(3),
        phase_results: serde_json::from_str(&phase_json).unwrap_or_default(),
        tokens_used: row.get(5),
        duration_ms: row.get(6),
        findings: serde_json::from_str(&findings_json).unwrap_or_default(),
        actions_taken: serde_json::from_str(&actions_json).unwrap_or_default(),
        escalations: serde_json::from_str(&escalations_json).unwrap_or_default(),
        error_message: row.get(10),
        created_at: row.get(11),
    }
}

fn map_loop_pattern(row: sqlx::sqlite::SqliteRow) -> LoopPattern {
    let phases_json: String = row.get(5);
    let gates_json: String = row.get(6);
    let cost_json: String = row.get(7);
    let skills_json: String = row.get(8);
    let schema_json: String = row.get(9);
    LoopPattern {
        id: row.get(0),
        name: row.get(1),
        description: row.get(2),
        goal: row.get(3),
        cadence: row.get(4),
        risk_level: row.get(10),
        phases: serde_json::from_str(&phases_json).unwrap_or_default(),
        human_gates: serde_json::from_str(&gates_json).unwrap_or_default(),
        cost_config: serde_json::from_str(&cost_json).unwrap_or(serde_json::json!({})),
        skills_required: serde_json::from_str(&skills_json).unwrap_or_default(),
        state_schema: serde_json::from_str(&schema_json).unwrap_or(serde_json::json!({})),
        is_active: row.get(11),
        created_at: row.get(12),
        updated_at: row.get(13),
    }
}

impl Database {
    pub async fn create_loop_state(&self, novel_id: &str, req: CreateLoopStateRequest) -> Result<LoopState, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let readiness = req.readiness_level.unwrap_or_else(|| "L0".to_string());
        let config = req.config.unwrap_or(serde_json::json!({}));
        let cap = req.token_cap_daily.unwrap_or(50000);
        let config_str = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());

        sqlx::query(
            "INSERT INTO loop_states (id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, created_at, updated_at) VALUES (?, ?, ?, 'idle', ?, '{}', ?, 0, ?, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(&req.pattern_id).bind(&readiness)
        .bind(&config_str).bind(cap).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(LoopState {
            id,
            novel_id: novel_id.to_string(),
            pattern_id: req.pattern_id,
            status: "idle".to_string(),
            readiness_level: readiness,
            state_payload: serde_json::json!({}),
            config,
            token_usage_today: 0,
            token_cap_daily: cap,
            last_run_at: None,
            last_run_result: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_loop_states(&self, novel_id: &str) -> Result<Vec<LoopState>, AppError> {
        sqlx::query(
            "SELECT id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at FROM loop_states WHERE novel_id = ? ORDER BY created_at DESC"
        )
        .bind(novel_id)
        .map(map_loop_state)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_loop_state_by_id(&self, state_id: &str) -> Result<LoopState, AppError> {
        sqlx::query(
            "SELECT id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at FROM loop_states WHERE id = ?"
        )
        .bind(state_id)
        .map(map_loop_state)
        .fetch_optional(&self.pool).await.map_err(db_err)?
        .ok_or_else(|| AppError::not_found("Loop state"))
    }

    pub async fn update_loop_state(&self, state_id: &str, req: UpdateLoopStateRequest) -> Result<LoopState, AppError> {
        let existing = self.get_loop_state_by_id(state_id).await?;
        let now = Utc::now().to_rfc3339();

        let status = req.status.unwrap_or(existing.status);
        let readiness_level = req.readiness_level.unwrap_or(existing.readiness_level);
        let config = req.config.unwrap_or(existing.config);
        let token_cap_daily = req.token_cap_daily.unwrap_or(existing.token_cap_daily);
        let last_run_at = req.last_run_at;
        let last_run_result = req.last_run_result;

        let config_str = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
        let result_str = last_run_result.as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "{}".to_string()));

        sqlx::query(
            "UPDATE loop_states SET status = ?, readiness_level = ?, config = ?, token_cap_daily = ?, last_run_at = ?, last_run_result = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&status).bind(&readiness_level).bind(&config_str)
        .bind(token_cap_daily).bind(&last_run_at).bind(&result_str)
        .bind(&now).bind(state_id)
        .execute(&self.pool).await.map_err(db_err)?;

        self.get_loop_state_by_id(state_id).await
    }

    pub async fn delete_loop_state(&self, state_id: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM loop_states WHERE id = ?")
            .bind(state_id)
            .execute(&self.pool).await.map_err(db_err)?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Loop state"));
        }
        Ok(())
    }

    pub async fn create_loop_run_log(&self, log: &LoopRunLog) -> Result<LoopRunLog, AppError> {
        let phase_json = serde_json::to_string(&log.phase_results).unwrap_or_else(|_| "[]".to_string());
        let findings_json = serde_json::to_string(&log.findings).unwrap_or_else(|_| "[]".to_string());
        let actions_json = serde_json::to_string(&log.actions_taken).unwrap_or_else(|_| "[]".to_string());
        let escalations_json = serde_json::to_string(&log.escalations).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO loop_run_logs (id, loop_state_id, pattern_id, status, phase_results, tokens_used, duration_ms, findings, actions_taken, escalations, error_message, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&log.id).bind(&log.loop_state_id).bind(&log.pattern_id).bind(&log.status)
        .bind(&phase_json).bind(log.tokens_used).bind(log.duration_ms)
        .bind(&findings_json).bind(&actions_json).bind(&escalations_json)
        .bind(&log.error_message).bind(&log.created_at)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(log.clone())
    }

    pub async fn get_loop_run_logs(&self, state_id: &str, limit: i64) -> Result<Vec<LoopRunLog>, AppError> {
        sqlx::query(
            "SELECT id, loop_state_id, pattern_id, status, phase_results, tokens_used, duration_ms, findings, actions_taken, escalations, error_message, created_at FROM loop_run_logs WHERE loop_state_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(state_id).bind(limit)
        .map(map_loop_run_log)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn get_loop_patterns(&self) -> Result<Vec<LoopPattern>, AppError> {
        sqlx::query(
            "SELECT id, name, description, goal, cadence, phases, human_gates, cost_config, skills_required, state_schema, risk_level, is_active, created_at, updated_at FROM loop_patterns ORDER BY name"
        )
        .map(map_loop_pattern)
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    pub async fn upsert_loop_pattern(&self, id: Option<&str>, req: UpsertLoopPatternRequest) -> Result<LoopPattern, AppError> {
        let now = Utc::now().to_rfc3339();
        let pattern_id = id.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let phases = req.phases.map(|p| serde_json::to_string(&p).unwrap_or_else(|_| "[]".to_string())).unwrap_or_else(|| "[]".to_string());
        let gates = req.human_gates.map(|g| serde_json::to_string(&g).unwrap_or_else(|_| "[]".to_string())).unwrap_or_else(|| "[]".to_string());
        let cost = req.cost_config.map(|c| serde_json::to_string(&c).unwrap_or_else(|_| "{}".to_string())).unwrap_or_else(|| "{}".to_string());
        let skills = req.skills_required.map(|s| serde_json::to_string(&s).unwrap_or_else(|_| "[]".to_string())).unwrap_or_else(|| "[]".to_string());
        let schema = req.state_schema.map(|s| serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string())).unwrap_or_else(|| "{}".to_string());
        let desc = req.description.unwrap_or_default();
        let goal = req.goal.unwrap_or_default();
        let cadence = req.cadence.unwrap_or_else(|| "1d".to_string());
        let risk = req.risk_level.unwrap_or_else(|| "low".to_string());
        let active = req.is_active.unwrap_or(true);

        sqlx::query(
            "INSERT INTO loop_patterns (id, name, description, goal, cadence, risk_level, phases, human_gates, cost_config, skills_required, state_schema, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description, goal=excluded.goal, cadence=excluded.cadence, risk_level=excluded.risk_level, phases=excluded.phases, human_gates=excluded.human_gates, cost_config=excluded.cost_config, skills_required=excluded.skills_required, state_schema=excluded.state_schema, is_active=excluded.is_active, updated_at=excluded.updated_at"
        )
        .bind(&pattern_id).bind(&req.name).bind(&desc).bind(&goal).bind(&cadence).bind(&risk)
        .bind(&phases).bind(&gates).bind(&cost).bind(&skills).bind(&schema)
        .bind(active).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(LoopPattern {
            id: pattern_id,
            name: req.name,
            description: desc,
            goal,
            cadence,
            risk_level: risk,
            phases: serde_json::from_str(&phases).unwrap_or_default(),
            human_gates: serde_json::from_str(&gates).unwrap_or_default(),
            cost_config: serde_json::from_str(&cost).unwrap_or(serde_json::json!({})),
            skills_required: serde_json::from_str(&skills).unwrap_or_default(),
            state_schema: serde_json::from_str(&schema).unwrap_or(serde_json::json!({})),
            is_active: active,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    async fn test_db() -> Database {
        let dir = env::temp_dir().join("mnemosyne_test_db");
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test_async.sqlite");
        let _ = std::fs::remove_file(&db_path);
        Database::new(db_path.to_str().unwrap()).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_list_workspaces() {
        let db = test_db().await;
        let req = CreateWorkspaceRequest {
            name: "Test Workspace".into(),
            path: Some("/tmp/test".into()),
        };
        let ws = db.create_workspace(req).await.unwrap();
        assert_eq!(ws.name, "Test Workspace");
        assert!(!ws.id.is_empty());

        let all = db.list_workspaces().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, ws.id);
    }

    #[tokio::test]
    async fn test_workspace_not_found() {
        let db = test_db().await;
        let result = db.get_workspace("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_workspace() {
        let db = test_db().await;
        let req = CreateWorkspaceRequest {
            name: "To Delete".into(),
            path: Some("/tmp/test".into()),
        };
        let ws = db.create_workspace(req).await.unwrap();
        let deleted = db.delete_workspace(&ws.id).await.unwrap();
        assert!(deleted);
        assert!(db.get_workspace(&ws.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_create_and_list_novels() {
        let db = test_db().await;
        let ws_req = CreateWorkspaceRequest {
            name: "WS".into(),
            path: Some("/tmp/test".into()),
        };
        let ws = db.create_workspace(ws_req).await.unwrap();

        let novel_req = CreateNovelRequest {
            workspace_id: ws.id,
            title: "Test Novel".into(),
            genre: "fantasy".into(),
            platform: "local".into(),
            language: "zh".into(),
            target_chapters: 100,
            chapter_words: 3000,
        };
        let novel = db.create_novel(&novel_req).await.unwrap();
        assert_eq!(novel.title, "Test Novel");

        let all = db.list_novels().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_create_and_get_prompt() {
        let db = test_db().await;
        let req = CreatePromptRequest {
            name: "Test Prompt".into(),
            content: "You are a helpful assistant".into(),
            category: "system".into(),
            tags: vec!["test".into()],
        };
        let prompt = db.create_prompt(req).await.unwrap();
        assert_eq!(prompt.name, "Test Prompt");

        let fetched = db.get_prompt(&prompt.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Test Prompt");
    }

    #[tokio::test]
    async fn test_create_and_list_trends() {
        let db = test_db().await;
        let trend = db.create_trend("AI", "twitter", 0.95, serde_json::json!({})).await.unwrap();
        assert_eq!(trend.keyword, "AI");

        let all = db.list_trends(None, None).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_workspace_validation() {
        let db = test_db().await;

        let empty_name = CreateWorkspaceRequest {
            name: "".into(),
            path: Some("/tmp".into()),
        };
        assert!(db.create_workspace(empty_name).await.is_err());

        let long_name = CreateWorkspaceRequest {
            name: "a".repeat(256).into(),
            path: Some("/tmp".into()),
        };
        assert!(db.create_workspace(long_name).await.is_err());
    }

    #[tokio::test]
    async fn test_novel_validation() {
        let db = test_db().await;
        let ws_req = CreateWorkspaceRequest {
            name: "WS".into(),
            path: Some("/tmp/test".into()),
        };
        let ws = db.create_workspace(ws_req).await.unwrap();

        let empty_title = CreateNovelRequest {
            workspace_id: ws.id.clone(),
            title: "".into(),
            genre: "fantasy".into(),
            platform: "local".into(),
            language: "zh".into(),
            target_chapters: 100,
            chapter_words: 3000,
        };
        assert!(db.create_novel(&empty_title).await.is_err());
    }
}
