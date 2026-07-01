//! S4：时序记忆库 —— StoryFact / ChapterSummary 的 SQLite 持久化层。
//!
//! ## 设计
//!
//! - `story_facts` 表：时序事实，按 (novel_id, fact_id) 唯一，带 valid_from / valid_until
//! - `chapter_summaries` 表：章节摘要，按 (novel_id, chapter) 唯一
//!
//! ## 时序查询语义
//!
//! `query_facts_at_chapter(novel_id, chapter)` 返回所有满足
//! `valid_from_chapter <= chapter AND (valid_until_chapter IS NULL OR valid_until_chapter > chapter)`
//! 的事实，即"在 chapter N 时为真的事实"。
//!
//! ## 与 StateManager 的关系
//!
//! StateManager 仍保留 state.json 作为 hooks 和元信息存储。facts/summaries
//! 双写到 SQLite，调用方可选择从 SQLite 查询（时序）或从 state.json 读取（全量）。
//! 后续阶段可逐步淘汰 state.json 中的 facts/summaries 字段。

use sqlx::Row;
use chrono::Utc;

use super::Database;
use super::connection::db_err;
use crate::shared::errors::AppError;
use crate::features::story::{StoryFact, ChapterSummary};

// ── StoryFacts ────────────────────────────────────────────────

impl Database {
    /// Upsert 单条 fact（按 novel_id + fact_id 唯一合并）。
    pub async fn upsert_story_fact(&self, novel_id: &str, fact: &StoryFact) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_facts (id, novel_id, fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(novel_id, fact_id) DO UPDATE SET
                subject = excluded.subject,
                predicate = excluded.predicate,
                object = excluded.object,
                valid_from_chapter = excluded.valid_from_chapter,
                valid_until_chapter = excluded.valid_until_chapter,
                source_chapter = excluded.source_chapter,
                updated_at = excluded.updated_at"
        )
        .bind(&fact.fact_id) // id 用 fact_id（同 novel 内唯一，便于幂等）
        .bind(novel_id)
        .bind(&fact.fact_id)
        .bind(&fact.subject)
        .bind(&fact.predicate)
        .bind(&fact.object)
        .bind(fact.valid_from_chapter)
        .bind(fact.valid_until_chapter)
        .bind(fact.source_chapter)
        .bind(&fact.created_at)
        .bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    /// 批量 upsert facts（单事务，原子性）。
    pub async fn upsert_story_facts_batch(
        &self,
        novel_id: &str,
        facts: &[StoryFact],
    ) -> Result<(), AppError> {
        if facts.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let now = Utc::now().to_rfc3339();
        for fact in facts {
            sqlx::query(
                "INSERT INTO story_facts (id, novel_id, fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(novel_id, fact_id) DO UPDATE SET
                    subject = excluded.subject,
                    predicate = excluded.predicate,
                    object = excluded.object,
                    valid_from_chapter = excluded.valid_from_chapter,
                    valid_until_chapter = excluded.valid_until_chapter,
                    source_chapter = excluded.source_chapter,
                    updated_at = excluded.updated_at"
            )
            .bind(&fact.fact_id)
            .bind(novel_id)
            .bind(&fact.fact_id)
            .bind(&fact.subject)
            .bind(&fact.predicate)
            .bind(&fact.object)
            .bind(fact.valid_from_chapter)
            .bind(fact.valid_until_chapter)
            .bind(fact.source_chapter)
            .bind(&fact.created_at)
            .bind(&now)
            .execute(&mut *tx).await.map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// 时序查询：找出在 chapter N 时有效的事实。
    ///
    /// 有效条件：`valid_from_chapter <= chapter AND (valid_until_chapter IS NULL OR valid_until_chapter > chapter)`
    pub async fn query_facts_at_chapter(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<Vec<StoryFact>, AppError> {
        sqlx::query(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at
             FROM story_facts
             WHERE novel_id = ? AND valid_from_chapter <= ? AND (valid_until_chapter IS NULL OR valid_until_chapter > ?)
             ORDER BY source_chapter ASC, fact_id ASC"
        )
        .bind(novel_id).bind(chapter).bind(chapter)
        .map(|row: sqlx::sqlite::SqliteRow| StoryFact {
            fact_id: row.get(0),
            subject: row.get(1),
            predicate: row.get(2),
            object: row.get(3),
            valid_from_chapter: row.get(4),
            valid_until_chapter: row.get(5),
            source_chapter: row.get(6),
            created_at: row.get(7),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 范围查询：找出在 [from, to] 区间内任意章节有效的事实。
    ///
    /// 用于构建多章节上下文（如回顾最近 5 章）。
    pub async fn query_facts_by_chapter_range(
        &self,
        novel_id: &str,
        from_chapter: u32,
        to_chapter: u32,
    ) -> Result<Vec<StoryFact>, AppError> {
        sqlx::query(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at
             FROM story_facts
             WHERE novel_id = ?
               AND valid_from_chapter <= ?
               AND (valid_until_chapter IS NULL OR valid_until_chapter > ?)
             ORDER BY source_chapter ASC, fact_id ASC"
        )
        .bind(novel_id).bind(to_chapter).bind(from_chapter)
        .map(|row: sqlx::sqlite::SqliteRow| StoryFact {
            fact_id: row.get(0),
            subject: row.get(1),
            predicate: row.get(2),
            object: row.get(3),
            valid_from_chapter: row.get(4),
            valid_until_chapter: row.get(5),
            source_chapter: row.get(6),
            created_at: row.get(7),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 列出 novel 的全部 facts（用于拼装完整 StoryState）。
    pub async fn list_story_facts(&self, novel_id: &str) -> Result<Vec<StoryFact>, AppError> {
        sqlx::query(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at
             FROM story_facts
             WHERE novel_id = ?
             ORDER BY source_chapter ASC, fact_id ASC"
        )
        .bind(novel_id)
        .map(|row: sqlx::sqlite::SqliteRow| StoryFact {
            fact_id: row.get(0),
            subject: row.get(1),
            predicate: row.get(2),
            object: row.get(3),
            valid_from_chapter: row.get(4),
            valid_until_chapter: row.get(5),
            source_chapter: row.get(6),
            created_at: row.get(7),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 按 subject 查询事实（用于人物/实体的状态追踪）。
    pub async fn query_facts_by_subject(
        &self,
        novel_id: &str,
        subject: &str,
    ) -> Result<Vec<StoryFact>, AppError> {
        sqlx::query(
            "SELECT fact_id, subject, predicate, object, valid_from_chapter, valid_until_chapter, source_chapter, created_at
             FROM story_facts
             WHERE novel_id = ? AND subject = ?
             ORDER BY source_chapter ASC, fact_id ASC"
        )
        .bind(novel_id).bind(subject)
        .map(|row: sqlx::sqlite::SqliteRow| StoryFact {
            fact_id: row.get(0),
            subject: row.get(1),
            predicate: row.get(2),
            object: row.get(3),
            valid_from_chapter: row.get(4),
            valid_until_chapter: row.get(5),
            source_chapter: row.get(6),
            created_at: row.get(7),
        })
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 把 fact 标记为在某章失效（重新上下文化，不删除）。
    ///
    /// 用于"事实不再成立"的场景，例如"主角持有宝剑"在 50 章后改为"主角失去宝剑"。
    pub async fn expire_fact_at_chapter(
        &self,
        novel_id: &str,
        fact_id: &str,
        expire_at_chapter: u32,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE story_facts
             SET valid_until_chapter = ?, updated_at = ?
             WHERE novel_id = ? AND fact_id = ? AND valid_until_chapter IS NULL"
        )
        .bind(expire_at_chapter).bind(&now)
        .bind(novel_id).bind(fact_id)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    /// 删除单条 fact（物理删除，慎用）。
    pub async fn delete_story_fact(
        &self,
        novel_id: &str,
        fact_id: &str,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM story_facts WHERE novel_id = ? AND fact_id = ?")
            .bind(novel_id).bind(fact_id)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

// ── ChapterSummaries ──────────────────────────────────────────

impl Database {
    /// Upsert 章节摘要（按 novel_id + chapter 唯一合并）。
    pub async fn upsert_chapter_summary(
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

        sqlx::query(
            "INSERT INTO chapter_summaries (id, novel_id, chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(novel_id, chapter) DO UPDATE SET
                title = excluded.title,
                characters_json = excluded.characters_json,
                events_json = excluded.events_json,
                state_changes_json = excluded.state_changes_json,
                hook_activity_json = excluded.hook_activity_json,
                mood = excluded.mood,
                chapter_type = excluded.chapter_type,
                updated_at = excluded.updated_at"
        )
        .bind(&summary.chapter.to_string()) // id 用 chapter 字符串（同 novel 内唯一）
        .bind(novel_id)
        .bind(summary.chapter)
        .bind(&summary.title)
        .bind(&characters_json)
        .bind(&events_json)
        .bind(&state_changes_json)
        .bind(&hook_activity_json)
        .bind(&summary.mood)
        .bind(&summary.chapter_type)
        .bind(&summary.created_at)
        .bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }

    /// 获取单章摘要。
    pub async fn get_chapter_summary(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<Option<ChapterSummary>, AppError> {
        sqlx::query(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at
             FROM chapter_summaries
             WHERE novel_id = ? AND chapter = ?"
        )
        .bind(novel_id).bind(chapter)
        .map(|row: sqlx::sqlite::SqliteRow| row_to_summary(&row))
        .fetch_optional(&self.pool).await.map_err(db_err)
    }

    /// 列出 novel 的全部章节摘要（按 chapter 升序）。
    pub async fn list_chapter_summaries(
        &self,
        novel_id: &str,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        sqlx::query(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at
             FROM chapter_summaries
             WHERE novel_id = ?
             ORDER BY chapter ASC"
        )
        .bind(novel_id)
        .map(|row: sqlx::sqlite::SqliteRow| row_to_summary(&row))
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 范围查询：列出 [from, to] 区间内的章节摘要。
    ///
    /// 用于 Composer 构建最近 N 章的上下文。
    pub async fn list_chapter_summaries_range(
        &self,
        novel_id: &str,
        from_chapter: u32,
        to_chapter: u32,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        sqlx::query(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at
             FROM chapter_summaries
             WHERE novel_id = ? AND chapter >= ? AND chapter <= ?
             ORDER BY chapter ASC"
        )
        .bind(novel_id).bind(from_chapter).bind(to_chapter)
        .map(|row: sqlx::sqlite::SqliteRow| row_to_summary(&row))
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 列出最近 N 章的摘要（不含 to_chapter 本身，便于"前 N 章回顾"）。
    pub async fn list_recent_chapter_summaries(
        &self,
        novel_id: &str,
        before_chapter: u32,
        limit: u32,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        sqlx::query(
            "SELECT chapter, title, characters_json, events_json, state_changes_json, hook_activity_json, mood, chapter_type, created_at
             FROM chapter_summaries
             WHERE novel_id = ? AND chapter < ?
             ORDER BY chapter DESC
             LIMIT ?"
        )
        .bind(novel_id).bind(before_chapter).bind(limit)
        .map(|row: sqlx::sqlite::SqliteRow| row_to_summary(&row))
        .fetch_all(&self.pool).await.map_err(db_err)
    }

    /// 删除单章摘要。
    pub async fn delete_chapter_summary(
        &self,
        novel_id: &str,
        chapter: u32,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM chapter_summaries WHERE novel_id = ? AND chapter = ?")
            .bind(novel_id).bind(chapter)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(result.rows_affected() > 0)
    }
}

// ── Row → ChapterSummary 转换 ────────────────────────────────

fn row_to_summary(row: &sqlx::sqlite::SqliteRow) -> ChapterSummary {
    let characters_json: String = row.get(2);
    let events_json: String = row.get(3);
    let state_changes_json: String = row.get(4);
    let hook_activity_json: String = row.get(5);

    ChapterSummary {
        chapter: row.get(0),
        title: row.get(1),
        characters: parse_json_array(&characters_json),
        events: parse_json_array(&events_json),
        state_changes: parse_json_array(&state_changes_json),
        hook_activity: parse_json_array(&hook_activity_json),
        mood: row.get(6),
        chapter_type: row.get(7),
        created_at: row.get(8),
    }
}

fn parse_json_array(json: &str) -> Vec<String> {
    if json.is_empty() || json == "[]" {
        return Vec::new();
    }
    serde_json::from_str(json).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::connection::Database;

    async fn setup_db() -> Database {
        let db = Database::connect_in_memory().await.unwrap();
        // connect_in_memory 已自动执行所有迁移
        // 创建测试 novel
        sqlx::query(
            "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES ('ws-1', 'test', '', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
        ).execute(&db.pool).await.unwrap();
        sqlx::query(
            "INSERT INTO novels (id, workspace_id, title, genre, platform, status, language, word_count, chapter_count, target_chapters, chapter_words, created_at, updated_at)
             VALUES ('novel-1', 'ws-1', 'Test Novel', 'general', 'local', 'drafting', 'zh', 0, 0, 100, 3000, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
        ).execute(&db.pool).await.unwrap();
        db
    }

    fn make_fact(fact_id: &str, from: u32, until: Option<u32>, source: u32) -> StoryFact {
        StoryFact {
            fact_id: fact_id.to_string(),
            subject: "主角".to_string(),
            predicate: "持有".to_string(),
            object: "宝剑".to_string(),
            valid_from_chapter: from,
            valid_until_chapter: until,
            source_chapter: source,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_summary(chapter: u32, title: &str) -> ChapterSummary {
        ChapterSummary {
            chapter,
            title: title.to_string(),
            characters: vec!["主角".to_string()],
            events: vec![format!("第{}章事件", chapter)],
            state_changes: vec![],
            hook_activity: vec![],
            mood: "严肃".to_string(),
            chapter_type: "setup".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn test_upsert_and_list_facts() {
        let db = setup_db().await;
        let f1 = make_fact("fact-1", 1, None, 1);
        let f2 = make_fact("fact-2", 5, Some(10), 5);
        db.upsert_story_facts_batch("novel-1", &[f1, f2]).await.unwrap();

        let facts = db.list_story_facts("novel-1").await.unwrap();
        assert_eq!(facts.len(), 2);
    }

    #[tokio::test]
    async fn test_query_facts_at_chapter_temporal() {
        let db = setup_db().await;
        // fact-1: 第 1 章起有效，至今仍有效
        // fact-2: 第 5 章起有效，第 10 章失效
        // fact-3: 第 8 章起有效，至今仍有效
        db.upsert_story_facts_batch("novel-1", &[
            make_fact("fact-1", 1, None, 1),
            make_fact("fact-2", 5, Some(10), 5),
            make_fact("fact-3", 8, None, 8),
        ]).await.unwrap();

        // 第 0 章：无事实
        let ch0 = db.query_facts_at_chapter("novel-1", 0).await.unwrap();
        assert_eq!(ch0.len(), 0);

        // 第 1 章：fact-1 有效
        let ch1 = db.query_facts_at_chapter("novel-1", 1).await.unwrap();
        assert_eq!(ch1.len(), 1);
        assert_eq!(ch1[0].fact_id, "fact-1");

        // 第 7 章：fact-1 + fact-2 有效（fact-3 还没出现）
        let ch7 = db.query_facts_at_chapter("novel-1", 7).await.unwrap();
        assert_eq!(ch7.len(), 2);

        // 第 10 章：fact-2 失效（valid_until=10 表示"在第 10 章失效"），fact-1 + fact-3 有效
        let ch10 = db.query_facts_at_chapter("novel-1", 10).await.unwrap();
        assert_eq!(ch10.len(), 2);
        assert!(ch10.iter().all(|f| f.fact_id != "fact-2"));

        // 第 100 章：fact-1 + fact-3 有效
        let ch100 = db.query_facts_at_chapter("novel-1", 100).await.unwrap();
        assert_eq!(ch100.len(), 2);
    }

    #[tokio::test]
    async fn test_expire_fact_at_chapter() {
        let db = setup_db().await;
        db.upsert_story_fact("novel-1", &make_fact("fact-1", 1, None, 1)).await.unwrap();

        // 在第 50 章失效
        db.expire_fact_at_chapter("novel-1", "fact-1", 50).await.unwrap();

        // 第 49 章仍有效
        let ch49 = db.query_facts_at_chapter("novel-1", 49).await.unwrap();
        assert_eq!(ch49.len(), 1);

        // 第 50 章失效
        let ch50 = db.query_facts_at_chapter("novel-1", 50).await.unwrap();
        assert_eq!(ch50.len(), 0);
    }

    #[tokio::test]
    async fn test_upsert_fact_idempotent() {
        let db = setup_db().await;
        let f1 = make_fact("fact-1", 1, None, 1);
        db.upsert_story_fact("novel-1", &f1).await.unwrap();
        // 再次 upsert 同 fact_id（不同内容）
        let f1_updated = StoryFact {
            object: "魔杖".to_string(),
            ..make_fact("fact-1", 1, None, 1)
        };
        db.upsert_story_fact("novel-1", &f1_updated).await.unwrap();

        let facts = db.list_story_facts("novel-1").await.unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].object, "魔杖");
    }

    #[tokio::test]
    async fn test_query_facts_by_subject() {
        let db = setup_db().await;
        db.upsert_story_facts_batch("novel-1", &[
            StoryFact { subject: "主角".to_string(), ..make_fact("fact-1", 1, None, 1) },
            StoryFact { subject: "反派".to_string(), ..make_fact("fact-2", 1, None, 1) },
        ]).await.unwrap();

        let protagonist_facts = db.query_facts_by_subject("novel-1", "主角").await.unwrap();
        assert_eq!(protagonist_facts.len(), 1);
        assert_eq!(protagonist_facts[0].fact_id, "fact-1");
    }

    #[tokio::test]
    async fn test_upsert_and_get_summary() {
        let db = setup_db().await;
        let s = make_summary(1, "第一章 启程");
        db.upsert_chapter_summary("novel-1", &s).await.unwrap();

        let loaded = db.get_chapter_summary("novel-1", 1).await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.title, "第一章 启程");
        assert_eq!(loaded.characters, vec!["主角"]);
        assert_eq!(loaded.events, vec!["第1章事件"]);
    }

    #[tokio::test]
    async fn test_upsert_summary_replaces_same_chapter() {
        let db = setup_db().await;
        let s1 = make_summary(1, "第一章 启程");
        db.upsert_chapter_summary("novel-1", &s1).await.unwrap();

        // 同 chapter 再次 upsert，应替换
        let s2 = make_summary(1, "第一章 启程（修订版）");
        db.upsert_chapter_summary("novel-1", &s2).await.unwrap();

        let all = db.list_chapter_summaries("novel-1").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "第一章 启程（修订版）");
    }

    #[tokio::test]
    async fn test_list_summaries_range() {
        let db = setup_db().await;
        for ch in 1..=10 {
            db.upsert_chapter_summary("novel-1", &make_summary(ch, &format!("第{}章", ch))).await.unwrap();
        }

        let range = db.list_chapter_summaries_range("novel-1", 3, 5).await.unwrap();
        assert_eq!(range.len(), 3);
        assert_eq!(range[0].chapter, 3);
        assert_eq!(range[2].chapter, 5);
    }

    #[tokio::test]
    async fn test_list_recent_summaries() {
        let db = setup_db().await;
        for ch in 1..=10 {
            db.upsert_chapter_summary("novel-1", &make_summary(ch, &format!("第{}章", ch))).await.unwrap();
        }

        // 取第 8 章之前的最近 3 章（即 5,6,7）
        let recent = db.list_recent_chapter_summaries("novel-1", 8, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
        // 按 chapter DESC 排序
        assert_eq!(recent[0].chapter, 7);
        assert_eq!(recent[1].chapter, 6);
        assert_eq!(recent[2].chapter, 5);
    }

    #[tokio::test]
    async fn test_delete_summary() {
        let db = setup_db().await;
        db.upsert_chapter_summary("novel-1", &make_summary(1, "ch1")).await.unwrap();
        let deleted = db.delete_chapter_summary("novel-1", 1).await.unwrap();
        assert!(deleted);
        let loaded = db.get_chapter_summary("novel-1", 1).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_cascade_delete_on_novel_removal() {
        let db = setup_db().await;
        db.upsert_story_facts_batch("novel-1", &[make_fact("fact-1", 1, None, 1)]).await.unwrap();
        db.upsert_chapter_summary("novel-1", &make_summary(1, "ch1")).await.unwrap();

        // 删除 novel 应级联删除 facts/summaries
        sqlx::query("DELETE FROM novels WHERE id = 'novel-1'")
            .execute(&db.pool).await.unwrap();

        let facts = db.list_story_facts("novel-1").await.unwrap();
        assert_eq!(facts.len(), 0);
        let summaries = db.list_chapter_summaries("novel-1").await.unwrap();
        assert_eq!(summaries.len(), 0);
    }
}
