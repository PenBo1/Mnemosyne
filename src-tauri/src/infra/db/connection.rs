use rusqlite::{Connection, params, Row};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

use super::models::*;
use crate::errors::AppError;

const SCHEMA_SQL: &str = include_str!("sql/schema.sql");

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
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<(), AppError> {
        self.conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| AppError::internal(format!("Failed to init schema: {}", e)))?;
        self.run_migrations()?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), AppError> {
        let migrations: &[(&str, &str)] = &[
            ("ALTER TABLE characters ADD COLUMN role TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN age TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN gender TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN appearance TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN personality TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN backstory TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN motivation TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN fears TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN skills TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE characters ADD COLUMN custom_fields TEXT NOT NULL DEFAULT '{}'", "characters"),
            ("ALTER TABLE characters ADD COLUMN updated_at TEXT NOT NULL DEFAULT ''", "characters"),
            ("ALTER TABLE world_settings ADD COLUMN description TEXT NOT NULL DEFAULT ''", "world_settings"),
            ("ALTER TABLE world_settings ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'", "world_settings"),
            ("ALTER TABLE world_settings ADD COLUMN updated_at TEXT NOT NULL DEFAULT ''", "world_settings"),
        ];

        for (sql, _table) in migrations {
            let _ = self.conn.execute_batch(sql);
        }

        Ok(())
    }

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
        let mut result = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, created_at, updated_at FROM workspaces ORDER BY created_at DESC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query([])
            .map_err(|e| AppError::internal(format!("Failed to query workspaces: {}", e)))?;
        while let Some(row) = rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            result.push(Self::row_to_workspace(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
        }
        Ok(result)
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

    pub fn update_workspace(&self, req: UpdateWorkspaceRequest) -> Result<Option<Workspace>, AppError> {
        let existing = self.get_workspace(&req.id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        let now = Utc::now().to_rfc3339();
        let name = req.name.unwrap_or(existing.name);
        let path = req.path.unwrap_or(existing.path);
        self.conn.execute(
            "UPDATE workspaces SET name = ?1, path = ?2, updated_at = ?3 WHERE id = ?4",
            params![name, path, now, req.id],
        ).map_err(|e| AppError::internal(format!("Failed to update workspace: {}", e)))?;
        self.get_workspace(&req.id)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete workspace: {}", e)))?;
        Ok(affected > 0)
    }

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

    fn row_to_novel(row: &Row) -> rusqlite::Result<Novel> {
        Ok(Novel {
            id: row.get(0)?,
            workspace_id: row.get(1)?,
            title: row.get(2)?,
            genre: row.get(3)?,
            status: row.get(4)?,
            word_count: row.get(5)?,
            chapter_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }

    pub fn create_prompt(&self, req: CreatePromptRequest) -> Result<Prompt, AppError> {
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
        let mut result = Vec::new();
        if let Some(cat) = category {
            let mut stmt = self.conn.prepare("SELECT id, name, content, category, tags, created_at, updated_at FROM prompts WHERE category = ?1 ORDER BY updated_at DESC")
                .map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let mut rows = stmt.query(params![cat])
                .map_err(|e| AppError::internal(format!("Failed to query prompts: {}", e)))?;
            while let Some(row) = rows.next()
                .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
                result.push(Self::row_to_prompt(row)
                    .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
            }
        } else {
            let mut stmt = self.conn.prepare("SELECT id, name, content, category, tags, created_at, updated_at FROM prompts ORDER BY updated_at DESC")
                .map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let mut rows = stmt.query([])
                .map_err(|e| AppError::internal(format!("Failed to query prompts: {}", e)))?;
            while let Some(row) = rows.next()
                .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
                result.push(Self::row_to_prompt(row)
                    .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
            }
        }
        Ok(result)
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

    pub fn update_prompt(&self, req: UpdatePromptRequest) -> Result<Option<Prompt>, AppError> {
        let existing = self.get_prompt(&req.id)?
            .ok_or_else(|| AppError::not_found("Prompt not found"))?;
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
        self.get_prompt(&req.id)
    }

    pub fn delete_prompt(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM prompts WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete prompt: {}", e)))?;
        Ok(affected > 0)
    }

    pub fn create_trend(&self, keyword: &str, platform: &str, score: f64, metadata: serde_json::Value) -> Result<Trend, AppError> {
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
        let limit = limit.unwrap_or(100);
        let mut result = Vec::new();
        if let Some(plat) = platform {
            let mut stmt = self.conn.prepare(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends WHERE platform = ?1 ORDER BY scanned_at DESC LIMIT ?2"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let mut rows = stmt.query(params![plat, limit])
                .map_err(|e| AppError::internal(format!("Failed to query trends: {}", e)))?;
            while let Some(row) = rows.next()
                .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
                result.push(Self::row_to_trend(row)
                    .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, keyword, platform, score, metadata, scanned_at FROM trends ORDER BY scanned_at DESC LIMIT ?1"
            ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
            let mut rows = stmt.query(params![limit])
                .map_err(|e| AppError::internal(format!("Failed to query trends: {}", e)))?;
            while let Some(row) = rows.next()
                .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
                result.push(Self::row_to_trend(row)
                    .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
            }
        }
        Ok(result)
    }

    pub fn delete_trend(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM trends WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete trend: {}", e)))?;
        Ok(affected > 0)
    }

    pub fn create_novel_with_workspace(&self, novel_id: &str, workspace_id: &str, title: &str, genre: &str) -> Result<Novel, AppError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO novels (id, workspace_id, title, genre, status, word_count, chapter_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'draft', 0, 0, ?5, ?6)",
            params![novel_id, workspace_id, title, genre, now, now],
        ).map_err(|e| AppError::internal(format!("Failed to create novel: {}", e)))?;
        Ok(Novel { id: novel_id.to_string(), workspace_id: workspace_id.to_string(), title: title.to_string(), genre: genre.to_string(), status: "draft".to_string(), word_count: 0, chapter_count: 0, created_at: now.clone(), updated_at: now })
    }

    pub fn list_novels(&self) -> Result<Vec<Novel>, AppError> {
        let mut result = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, genre, status, word_count, chapter_count, created_at, updated_at FROM novels ORDER BY updated_at DESC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query([])
            .map_err(|e| AppError::internal(format!("Failed to query novels: {}", e)))?;
        while let Some(row) = rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            result.push(Self::row_to_novel(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
        }
        Ok(result)
    }

    pub fn delete_novel(&self, id: &str) -> Result<bool, AppError> {
        let affected = self.conn.execute("DELETE FROM novels WHERE id = ?1", params![id])
            .map_err(|e| AppError::internal(format!("Failed to delete novel: {}", e)))?;
        Ok(affected > 0)
    }

    pub fn update_novel(&self, id: &str, title: &str, genre: &str) -> Result<Novel, AppError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE novels SET title = ?1, genre = ?2, updated_at = ?3 WHERE id = ?4",
            params![title, genre, now, id],
        ).map_err(|e| AppError::internal(format!("Failed to update novel: {}", e)))?;
        let mut stmt = self.conn.prepare(
            "SELECT id, workspace_id, title, genre, status, word_count, chapter_count, created_at, updated_at FROM novels WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query novel: {}", e)))?;
        match rows.next().map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Self::row_to_novel(row).map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?),
            None => Err(AppError::internal("Novel not found after update")),
        }
    }

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
        let limit = limit.unwrap_or(50);
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
    // Agents: Configuration CRUD
    // ═══════════════════════════════════════════════════════════

    fn row_to_agent(row: &Row) -> rusqlite::Result<AgentRow> {
        Ok(AgentRow {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            model: row.get(3)?,
            system_prompt: row.get(4)?,
            temperature: row.get(5)?,
            max_tokens: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
        })
    }

    pub fn seed_default_agents(&self) -> Result<(), AppError> {
        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .map_err(|e| AppError::internal(format!("Failed to count agents: {}", e)))?;
        if count > 0 {
            return Ok(());
        }

        let now = Utc::now().to_rfc3339();
        let defaults: Vec<(&str, &str, &str, f64, i64)> = vec![
            ("architect", "建筑师 (Architect)", "建书时生成故事框架、世界观、角色、书级规则", 0.7, 4096),
            ("planner", "规划师 (Planner)", "为下一章生成章节意图（must_keep/must_avoid/focus_points）", 0.3, 2048),
            ("composer", "编排师 (Composer)", "为写手组装精简的上下文包", 0.2, 4096),
            ("writer", "写手 (Writer)", "根据上下文包和章节意图生成章节正文", 0.8, 8192),
            ("auditor", "审计员 (Auditor)", "检查章节草稿的连续性和质量（10 维度审计）", 0.2, 4096),
            ("reviser", "修订者 (Reviser)", "根据审计结果和门禁失败建议修订章节", 0.5, 8192),
            ("observer", "观察者 (Observer)", "从章节正文中提取结构化事实（9 类）", 0.1, 4096),
            ("reflector", "反射器 (Reflector)", "将观察者提取的事实更新到运行时状态", 0.2, 4096),
        ];

        for (id, name, desc, temp, max_tokens) in defaults {
            self.conn.execute(
                "INSERT INTO agents (id, name, description, model, system_prompt, temperature, max_tokens, status, created_at) VALUES (?1, ?2, ?3, 'gpt-4', '', ?4, ?5, 'active', ?6)",
                params![id, name, desc, temp, max_tokens, now],
            ).map_err(|e| AppError::internal(format!("Failed to seed agent {}: {}", id, e)))?;
        }
        Ok(())
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRow>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, model, system_prompt, temperature, max_tokens, status, created_at FROM agents ORDER BY created_at ASC"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query([])
            .map_err(|e| AppError::internal(format!("Failed to query agents: {}", e)))?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            result.push(Self::row_to_agent(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?);
        }
        Ok(result)
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<AgentRow>, AppError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, model, system_prompt, temperature, max_tokens, status, created_at FROM agents WHERE id = ?1"
        ).map_err(|e| AppError::internal(format!("Failed to prepare query: {}", e)))?;
        let mut rows = stmt.query(params![id])
            .map_err(|e| AppError::internal(format!("Failed to query agent: {}", e)))?;
        match rows.next()
            .map_err(|e| AppError::internal(format!("Failed to fetch row: {}", e)))? {
            Some(row) => Ok(Some(Self::row_to_agent(row)
                .map_err(|e| AppError::internal(format!("Failed to map row: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn update_agent(&self, req: UpdateAgentRequest) -> Result<AgentRow, AppError> {
        let existing = self.get_agent(&req.id)?
            .ok_or_else(|| AppError::not_found("Agent not found"))?;
        let name = req.name.unwrap_or(existing.name);
        let description = req.description.unwrap_or(existing.description);
        let model = req.model.unwrap_or(existing.model);
        let system_prompt = req.system_prompt.unwrap_or(existing.system_prompt);
        let temperature = req.temperature.unwrap_or(existing.temperature);
        let max_tokens = req.max_tokens.unwrap_or(existing.max_tokens);
        self.conn.execute(
            "UPDATE agents SET name = ?1, description = ?2, model = ?3, system_prompt = ?4, temperature = ?5, max_tokens = ?6 WHERE id = ?7",
            params![name, description, model, system_prompt, temperature, max_tokens, req.id],
        ).map_err(|e| AppError::internal(format!("Failed to update agent: {}", e)))?;
        self.get_agent(&req.id)?.ok_or_else(|| AppError::internal("Agent not found after update"))
    }

    pub fn toggle_agent_status(&self, id: &str) -> Result<AgentRow, AppError> {
        let existing = self.get_agent(id)?
            .ok_or_else(|| AppError::not_found("Agent not found"))?;
        let new_status = if existing.status == "active" { "inactive" } else { "active" };
        self.conn.execute(
            "UPDATE agents SET status = ?1 WHERE id = ?2",
            params![new_status, id],
        ).map_err(|e| AppError::internal(format!("Failed to toggle agent status: {}", e)))?;
        self.get_agent(id)?.ok_or_else(|| AppError::internal("Agent not found after toggle"))
    }
}
