//! ═══════════════════════════════════════════════════════════════════════════
//! 故事存储 - 故事事实与章节摘要
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供两类存储：
//! - StoryFact：故事事实（subject-predicate-object 三元组）
//! - ChapterSummary：章节摘要（角色、事件、状态变化、钩子活动）

use rusqlite::params;
use chrono::Utc;

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use crate::shared::story::models::{StoryFact, ChapterSummary};

// ── Story Fact 辅助函数 ─────────────────────────────────────────────────────

/// 映射故事事实行
fn map_story_fact_row(row: &rusqlite::Row) -> rusqlite::Result<(String, String, String, String, i64, Option<i64>, i64, String)> {
    Ok((
        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
        row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
    ))
}

/// 构建故事事实
fn build_story_fact(raw: (String, String, String, String, i64, Option<i64>, i64, String)) -> StoryFact {
    StoryFact {
        fact_id: raw.0,
        subject: raw.1,
        predicate: raw.2,
        object: raw.3,
        valid_from_chapter: raw.4 as u32,
        valid_until_chapter: raw.5.map(|v| v as u32),
        source_chapter: raw.6 as u32,
        created_at: raw.7,
    }
}

const STORY_FACT_UPSERT_SQL: &str = "INSERT INTO story_facts (id, novel_id, fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at, updated_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
     ON CONFLICT(novel_id, fact_id) DO UPDATE SET \
        subject = excluded.subject, \
        predicate = excluded.predicate, \
        object = excluded.object, \
        valid_from_chapter = excluded.valid_from_chapter, \
        valid_until_chapter = excluded.valid_until_chapter, \
        source_chapter = excluded.source_chapter, \
        updated_at = excluded.updated_at";

// ── Story Fact 操作 ────────────────────────────────────────────────────────

impl Database {
    /// 插入或更新故事事实
    pub fn upsert_story_fact(&self, novel_id: &str, fact: &StoryFact) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            STORY_FACT_UPSERT_SQL,
            params![
                &fact.fact_id, novel_id, &fact.fact_id,
                &fact.subject, &fact.predicate, &fact.object,
                fact.valid_from_chapter as i64,
                fact.valid_until_chapter.map(|v| v as i64),
                fact.source_chapter as i64,
                &fact.created_at, &now,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 批量插入或更新故事事实
    pub fn upsert_story_facts_batch(
        &self,
        novel_id: &str,
        facts: &[StoryFact],
    ) -> Result<(), AppError> {
        if facts.is_empty() {
            return Ok(())
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction().map_err(db_err)?;
        let now = Utc::now().to_rfc3339();
        for fact in facts {
            tx.execute(
                STORY_FACT_UPSERT_SQL,
                params![
                    &fact.fact_id, novel_id, &fact.fact_id,
                    &fact.subject, &fact.predicate, &fact.object,
                    fact.valid_from_chapter as i64,
                    fact.valid_until_chapter.map(|v| v as i64),
                    fact.source_chapter as i64,
                    &fact.created_at, &now,
                ],
            ).map_err(db_err)?;
        }
        tx.commit().map_err(db_err)?;
        Ok(())
    }

    /// 查询指定章节的事实
    pub fn query_facts_at_chapter(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<Vec<StoryFact>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at \
             FROM story_facts \
             WHERE novel_id = ? AND valid_from_chapter <= ? AND (valid_until_chapter IS NULL OR valid_until_chapter > ?) \
             ORDER BY source_chapter ASC, fact_id ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, chapter as i64, chapter as i64], map_story_fact_row).map_err(db_err)?;
        rows.map(|r| Ok(build_story_fact(r.map_err(db_err)?))).collect()
    }

    /// 按章节范围查询事实
    pub fn query_facts_by_chapter_range(
        &self,
        novel_id: &str,
        from_chapter: u32,
        to_chapter: u32,
    ) -> Result<Vec<StoryFact>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at \
             FROM story_facts \
             WHERE novel_id = ? \
               AND valid_from_chapter <= ? \
               AND (valid_until_chapter IS NULL OR valid_until_chapter > ?) \
             ORDER BY source_chapter ASC, fact_id ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, to_chapter as i64, from_chapter as i64], map_story_fact_row).map_err(db_err)?;
        rows.map(|r| Ok(build_story_fact(r.map_err(db_err)?))).collect()
    }

    /// 列出所有故事事实
    pub fn list_story_facts(&self, novel_id: &str) -> Result<Vec<StoryFact>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at \
             FROM story_facts \
             WHERE novel_id = ? \
             ORDER BY source_chapter ASC, fact_id ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map([novel_id], map_story_fact_row).map_err(db_err)?;
        rows.map(|r| Ok(build_story_fact(r.map_err(db_err)?))).collect()
    }

    /// 按主体查询事实
    pub fn query_facts_by_subject(
        &self,
        novel_id: &str,
        subject: &str,
    ) -> Result<Vec<StoryFact>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at \
             FROM story_facts \
             WHERE novel_id = ? AND subject = ? \
             ORDER BY source_chapter ASC, fact_id ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, subject], map_story_fact_row).map_err(db_err)?;
        rows.map(|r| Ok(build_story_fact(r.map_err(db_err)?))).collect()
    }

    /// 使事实在指定章节失效
    pub fn expire_fact_at_chapter(
        &self,
        novel_id: &str,
        fact_id: &str,
        expire_at_chapter: u32,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE story_facts \
             SET valid_until_chapter = ?, updated_at = ? \
             WHERE novel_id = ? AND fact_id = ? AND valid_until_chapter IS NULL",
            params![expire_at_chapter as i64, &now, novel_id, fact_id],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 删除故事事实
    pub fn delete_story_fact(
        &self,
        novel_id: &str,
        fact_id: &str,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM story_facts WHERE novel_id = ? AND fact_id = ?",
            params![novel_id, fact_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

// ── Chapter Summary 辅助函数 ────────────────────────────────────────────────

/// 映射摘要行
fn map_summary_row(row: &rusqlite::Row) -> rusqlite::Result<(i64, String, String, String, String, String, String, String, String)> {
    Ok((
        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
        row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?,
    ))
}

/// 构建摘要
fn build_summary(raw: (i64, String, String, String, String, String, String, String, String)) -> ChapterSummary {
    ChapterSummary {
        chapter: raw.0 as u32,
        title: raw.1,
        characters: parse_json_array(&raw.2),
        events: parse_json_array(&raw.3),
        state_changes: parse_json_array(&raw.4),
        hook_activity: parse_json_array(&raw.5),
        mood: raw.6,
        chapter_type: raw.7,
        created_at: raw.8,
    }
}

// ── Chapter Summary 操作 ────────────────────────────────────────────────────

impl Database {
    /// 插入或更新章节摘要
    pub fn upsert_chapter_summary(
        &self,
        novel_id: &str,
        summary: &ChapterSummary,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let characters_json = serde_json::to_string(&summary.characters)
            .map_err(|e| AppError::internal(format!("Failed to serialize characters: {}", e)))?;
        let events_json = serde_json::to_string(&summary.events)
            .map_err(|e| AppError::internal(format!("Failed to serialize events: {}", e)))?;
        let state_changes_json = serde_json::to_string(&summary.state_changes)
            .map_err(|e| AppError::internal(format!("Failed to serialize state_changes: {}", e)))?;
        let hook_activity_json = serde_json::to_string(&summary.hook_activity)
            .map_err(|e| AppError::internal(format!("Failed to serialize hook_activity: {}", e)))?;

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO chapter_summaries (id, novel_id, chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(novel_id, chapter) DO UPDATE SET \
                title = excluded.title, \
                characters_json = excluded.characters_json, \
                events_json = excluded.events_json, \
                state_changes_json = excluded.state_changes_json, \
                hook_activity_json = excluded.hook_activity_json, \
                mood = excluded.mood, \
                chapter_type = excluded.chapter_type, \
                updated_at = excluded.updated_at",
            params![
                &summary.chapter.to_string(),
                novel_id,
                summary.chapter as i64,
                &summary.title,
                &characters_json, &events_json, &state_changes_json, &hook_activity_json,
                &summary.mood, &summary.chapter_type,
                &summary.created_at, &now,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 获取章节摘要
    pub fn get_chapter_summary(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<Option<ChapterSummary>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at \
             FROM chapter_summaries \
             WHERE novel_id = ? AND chapter = ?",
            params![novel_id, chapter as i64],
            map_summary_row,
        );
        match result {
            Ok(raw) => Ok(Some(build_summary(raw))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    /// 列出所有章节摘要
    pub fn list_chapter_summaries(
        &self,
        novel_id: &str,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at \
             FROM chapter_summaries \
             WHERE novel_id = ? \
             ORDER BY chapter ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map([novel_id], map_summary_row).map_err(db_err)?;
        rows.map(|r| Ok(build_summary(r.map_err(db_err)?))).collect()
    }

    /// 列出章节范围摘要
    pub fn list_chapter_summaries_range(
        &self,
        novel_id: &str,
        from_chapter: u32,
        to_chapter: u32,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at \
             FROM chapter_summaries \
             WHERE novel_id = ? AND chapter >= ? AND chapter <= ? \
             ORDER BY chapter ASC"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, from_chapter as i64, to_chapter as i64], map_summary_row).map_err(db_err)?;
        rows.map(|r| Ok(build_summary(r.map_err(db_err)?))).collect()
    }

    /// 列出最近的章节摘要
    pub fn list_recent_chapter_summaries(
        &self,
        novel_id: &str,
        before_chapter: u32,
        limit: u32,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at \
             FROM chapter_summaries \
             WHERE novel_id = ? AND chapter < ? \
             ORDER BY chapter DESC \
             LIMIT ?"
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![novel_id, before_chapter as i64, limit as i64], map_summary_row).map_err(db_err)?;
        rows.map(|r| Ok(build_summary(r.map_err(db_err)?))).collect()
    }

    /// 删除章节摘要
    pub fn delete_chapter_summary(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM chapter_summaries WHERE novel_id = ? AND chapter = ?",
            params![novel_id, chapter as i64],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

/// 解析 JSON 数组
fn parse_json_array(json: &str) -> Vec<String> {
    if json.is_empty() || json == "[]" {
        return Vec::new()
    }
    serde_json::from_str(json).unwrap_or_default()
}