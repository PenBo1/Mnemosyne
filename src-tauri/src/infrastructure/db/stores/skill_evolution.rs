//! ═══════════════════════════════════════════════════════════════════════════
//! 技能进化存储 - 技能使用统计与候选提案
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 两个核心能力：
//! 1. 已存在 skill 的使用统计（每次工具调用时增量更新）
//! 2. 从成功任务中提取 skill 候选（等待人工审核）

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── Skill 使用统计 ──────────────────────────────────────────────────────────

const USAGE_UPSERT_SQL: &str = "\
INSERT INTO skill_usage_stats (\
    skill_name, used_count, success_count, failure_count, user_feedback_score, \
    first_used_at, last_used_at, last_session_id\
) VALUES (?1, 1, ?2, ?3, 0.0, ?4, ?5, ?6) \
ON CONFLICT(skill_name) DO UPDATE SET \
    used_count = used_count + 1, \
    success_count = success_count + ?2, \
    failure_count = failure_count + ?3, \
    last_used_at = excluded.last_used_at, \
    last_session_id = excluded.last_session_id";

const USAGE_SELECT_COLUMNS: &str = "\
skill_name, used_count, success_count, failure_count, user_feedback_score, \
first_used_at, last_used_at, last_session_id";

/// Skill 使用统计行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsageStatsRow {
    /// 技能名称
    pub skill_name: String,
    /// 使用次数
    pub used_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 用户反馈分
    pub user_feedback_score: f64,
    /// 首次使用时间
    pub first_used_at: String,
    /// 最后使用时间
    pub last_used_at: String,
    /// 最后会话 ID
    pub last_session_id: Option<String>,
}

/// Skill 成熟度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillMaturity {
    /// 新生（1-2 次）
    Emerging,
    /// 开发中（3-9 次）
    Developing,
    /// 成熟（10+ 次）
    Mature,
    /// 弃用（90 天未用）
    Deprecated,
}

impl SkillMaturity {
    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Emerging => "emerging",
            Self::Developing => "developing",
            Self::Mature => "mature",
            Self::Deprecated => "deprecated",
        }
    }
}

impl SkillUsageStatsRow {
    /// 计算成熟度
    pub fn maturity(&self) -> SkillMaturity {
        if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&self.last_used_at) {
            let now = chrono::Utc::now();
            if (now - last.with_timezone(&chrono::Utc)).num_days() > 90 {
                return SkillMaturity::Deprecated;
            }
        }
        match self.used_count {
            0 => SkillMaturity::Emerging,
            1..=2 => SkillMaturity::Emerging,
            3..=9 => SkillMaturity::Developing,
            _ => SkillMaturity::Mature,
        }
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        if self.used_count == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.used_count as f64
    }
}

/// 映射使用统计行
fn map_usage_row(row: &rusqlite::Row) -> rusqlite::Result<SkillUsageStatsRow> {
    Ok(SkillUsageStatsRow {
        skill_name: row.get(0)?,
        used_count: row.get::<_, i64>(1)? as u64,
        success_count: row.get::<_, i64>(2)? as u64,
        failure_count: row.get::<_, i64>(3)? as u64,
        user_feedback_score: row.get(4)?,
        first_used_at: row.get(5)?,
        last_used_at: row.get(6)?,
        last_session_id: row.get(7)?,
    })
}

// ── Skill 候选提案 ────────────────────────────────────────────────────────

const CANDIDATE_INSERT_SQL: &str = "\
INSERT INTO skill_candidate_proposals (\
    id, candidate_name, candidate_description, source_session_id, source_summary, \
    candidate_content, status, reviewer_notes, reviewed_at, created_at\
) VALUES (?, ?, ?, ?, ?, ?, 'pending', NULL, NULL, ?)";

const CANDIDATE_SELECT_COLUMNS: &str = "\
id, candidate_name, candidate_description, source_session_id, source_summary, \
candidate_content, status, reviewer_notes, reviewed_at, created_at";

/// Skill 候选行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCandidateRow {
    /// 候选 ID
    pub id: String,
    /// 候选名称
    pub candidate_name: String,
    /// 候选描述
    pub candidate_description: String,
    /// 来源会话 ID
    pub source_session_id: String,
    /// 来源摘要
    pub source_summary: String,
    /// 候选内容
    pub candidate_content: String,
    /// 状态
    pub status: String,
    /// 审核者备注
    pub reviewer_notes: Option<String>,
    /// 审核时间
    pub reviewed_at: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 映射候选行
fn map_candidate_row(row: &rusqlite::Row) -> rusqlite::Result<SkillCandidateRow> {
    Ok(SkillCandidateRow {
        id: row.get(0)?,
        candidate_name: row.get(1)?,
        candidate_description: row.get(2)?,
        source_session_id: row.get(3)?,
        source_summary: row.get(4)?,
        candidate_content: row.get(5)?,
        status: row.get(6)?,
        reviewer_notes: row.get(7)?,
        reviewed_at: row.get(8)?,
        created_at: row.get(9)?,
    })
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 记录一次 skill 使用
    pub fn record_skill_usage(
        &self,
        skill_name: &str,
        success: bool,
        session_id: Option<&str>,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now();
        let now_str = now.to_rfc3339();
        let success_inc = if success { 1 } else { 0 };
        let failure_inc = if success { 0 } else { 1 };
        let conn = self.conn()?;
        conn.execute(
            USAGE_UPSERT_SQL,
            params![
                skill_name,
                success_inc,
                failure_inc,
                &now_str,
                &now_str,
                session_id,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 列出所有 skill 使用统计
    pub fn list_skill_usage_stats(&self) -> Result<Vec<SkillUsageStatsRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM skill_usage_stats ORDER BY used_count DESC",
            USAGE_SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([], map_usage_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 获取 skill 使用统计
    pub fn get_skill_usage_stats(
        &self,
        skill_name: &str,
    ) -> Result<Option<SkillUsageStatsRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM skill_usage_stats WHERE skill_name = ?1",
            USAGE_SELECT_COLUMNS
        )).map_err(db_err)?;
        let mut rows = stmt.query_map([skill_name], map_usage_row).map_err(db_err)?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(db_err)?))
        } else {
            Ok(None)
        }
    }

    /// 调整用户反馈分
    pub fn adjust_skill_feedback(
        &self,
        skill_name: &str,
        delta: f64,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE skill_usage_stats SET user_feedback_score = user_feedback_score + ?1 \
             WHERE skill_name = ?2",
            params![delta, skill_name],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 插入 skill 候选
    pub fn insert_skill_candidate(
        &self,
        candidate_name: &str,
        candidate_description: &str,
        source_session_id: &str,
        source_summary: &str,
        candidate_content: &str,
    ) -> Result<String, AppError> {
        let now = chrono::Utc::now();
        let id = format!("sc-{}-{}", now.timestamp_nanos_opt().unwrap_or(0), candidate_name);
        let conn = self.conn()?;
        conn.execute(
            CANDIDATE_INSERT_SQL,
            params![
                &id,
                candidate_name,
                candidate_description,
                source_session_id,
                source_summary,
                candidate_content,
                &now.to_rfc3339(),
            ],
        ).map_err(db_err)?;
        Ok(id)
    }

    /// 列出 skill 候选
    pub fn list_skill_candidates(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<SkillCandidateRow>, AppError> {
        let conn = self.conn()?;
        let sql = if status.is_some() {
            format!(
                "SELECT {} FROM skill_candidate_proposals WHERE status = ?1 ORDER BY created_at DESC",
                CANDIDATE_SELECT_COLUMNS
            )
        } else {
            format!(
                "SELECT {} FROM skill_candidate_proposals ORDER BY created_at DESC",
                CANDIDATE_SELECT_COLUMNS
            )
        };
        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = if let Some(s) = status {
            let valid = ["pending", "approved", "rejected", "superseded"];
            if !valid.contains(&s) {
                return Err(AppError::invalid_input(format!(
                    "Invalid status: {} (allowed: {:?})",
                    s, valid
                )));
            }
            stmt.query_map([s], map_candidate_row).map_err(db_err)?
        } else {
            stmt.query_map([], map_candidate_row).map_err(db_err)?
        };
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 更新候选状态
    pub fn update_candidate_status(
        &self,
        id: &str,
        status: &str,
        reviewer_notes: Option<&str>,
    ) -> Result<bool, AppError> {
        let valid = ["approved", "rejected", "superseded"];
        if !valid.contains(&status) {
            return Err(AppError::invalid_input(format!(
                "Invalid status: {} (allowed: {:?})",
                status, valid
            )));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE skill_candidate_proposals SET status = ?1, reviewer_notes = ?2, reviewed_at = ?3 \
             WHERE id = ?4 AND status = 'pending'",
            params![status, reviewer_notes, &now, id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 删除候选
    pub fn delete_skill_candidate(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM skill_candidate_proposals WHERE id = ?1",
            params![id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    #[test]
    fn record_skill_usage_inserts_then_increments() {
        let db = make_in_memory_db();
        db.record_skill_usage("read_file", true, Some("sess-1")).unwrap();
        let stats = db.get_skill_usage_stats("read_file").unwrap().unwrap();
        assert_eq!(stats.used_count, 1);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.last_session_id, Some("sess-1".to_string()));

        db.record_skill_usage("read_file", false, Some("sess-2")).unwrap();
        let stats = db.get_skill_usage_stats("read_file").unwrap().unwrap();
        assert_eq!(stats.used_count, 2);
        assert_eq!(stats.success_count, 1);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.last_session_id, Some("sess-2".to_string()));
    }

    #[test]
    fn maturity_reflects_usage_count() {
        let db = make_in_memory_db();
        db.record_skill_usage("skill1", true, None).unwrap();
        let s = db.get_skill_usage_stats("skill1").unwrap().unwrap();
        assert_eq!(s.maturity(), SkillMaturity::Emerging);

        for _ in 0..2 {
            db.record_skill_usage("skill1", true, None).unwrap();
        }
        let s = db.get_skill_usage_stats("skill1").unwrap().unwrap();
        assert_eq!(s.maturity(), SkillMaturity::Developing);

        for _ in 0..7 {
            db.record_skill_usage("skill1", true, None).unwrap();
        }
        let s = db.get_skill_usage_stats("skill1").unwrap().unwrap();
        assert_eq!(s.maturity(), SkillMaturity::Mature);
    }

    #[test]
    fn success_rate_calculation() {
        let db = make_in_memory_db();
        db.record_skill_usage("s", true, None).unwrap();
        db.record_skill_usage("s", true, None).unwrap();
        db.record_skill_usage("s", false, None).unwrap();
        let s = db.get_skill_usage_stats("s").unwrap().unwrap();
        assert_eq!(s.used_count, 3);
        assert!((s.success_rate() - (2.0 / 3.0)).abs() < 0.01);
    }

    #[test]
    fn list_skill_usage_stats_orders_by_count() {
        let db = make_in_memory_db();
        db.record_skill_usage("rare", true, None).unwrap();
        for _ in 0..5 {
            db.record_skill_usage("common", true, None).unwrap();
        }
        let all = db.list_skill_usage_stats().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].skill_name, "common");
        assert_eq!(all[1].skill_name, "rare");
    }

    #[test]
    fn adjust_feedback_accumulates() {
        let db = make_in_memory_db();
        db.record_skill_usage("f", true, None).unwrap();
        db.adjust_skill_feedback("f", 1.0).unwrap();
        db.adjust_skill_feedback("f", -0.5).unwrap();
        let s = db.get_skill_usage_stats("f").unwrap().unwrap();
        assert!((s.user_feedback_score - 0.5).abs() < 0.01);
    }

    #[test]
    fn insert_and_list_skill_candidate() {
        let db = make_in_memory_db();
        let id = db.insert_skill_candidate(
            "extract-dialogue",
            "从对话中提取角色声纹",
            "sess-123",
            "用户讨论了对话风格",
            "# Extract Dialogue Skill\n\n...",
        ).unwrap();
        assert!(!id.is_empty());

        let pending = db.list_skill_candidates(Some("pending")).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].candidate_name, "extract-dialogue");
        assert_eq!(pending[0].status, "pending");

        let all = db.list_skill_candidates(None).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn update_candidate_status_only_for_pending() {
        let db = make_in_memory_db();
        let id = db.insert_skill_candidate("x", "y", "s", "z", "c").unwrap();

        let updated = db.update_candidate_status(&id, "approved", Some("looks good")).unwrap();
        assert!(updated);

        let updated_again = db.update_candidate_status(&id, "rejected", None).unwrap();
        assert!(!updated_again);

        let c = db.list_skill_candidates(Some("approved")).unwrap();
        assert_eq!(c[0].status, "approved");
        assert!(c[0].reviewed_at.is_some());
        assert_eq!(c[0].reviewer_notes, Some("looks good".to_string()));
    }

    #[test]
    fn update_candidate_rejects_invalid_status() {
        let db = make_in_memory_db();
        let id = db.insert_skill_candidate("x", "y", "s", "z", "c").unwrap();
        let result = db.update_candidate_status(&id, "invalid", None);
        assert!(result.is_err());
    }

    #[test]
    fn delete_skill_candidate_works() {
        let db = make_in_memory_db();
        let id = db.insert_skill_candidate("x", "y", "s", "z", "c").unwrap();
        let deleted = db.delete_skill_candidate(&id).unwrap();
        assert!(deleted);
        assert_eq!(db.list_skill_candidates(None).unwrap().len(), 0);
    }

    #[test]
    fn maturity_deprecated_for_stale_skill() {
        let db = make_in_memory_db();
        db.record_skill_usage("old", true, None).unwrap();
        let stale = chrono::Utc::now() - chrono::Duration::days(100);
        let conn = db.conn().unwrap();
        conn.execute(
            "UPDATE skill_usage_stats SET last_used_at = ?1 WHERE skill_name = ?2",
            params![stale.to_rfc3339(), "old"],
        ).unwrap();

        let s = db.get_skill_usage_stats("old").unwrap().unwrap();
        assert_eq!(s.maturity(), SkillMaturity::Deprecated);
    }
}