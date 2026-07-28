//! ═══════════════════════════════════════════════════════════════════════════
//! 循环模式存储 - Loop-Engineering 模式定义持久化
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计要点：
//! - 内置 4 种模式（对应 LoopPatternId）
//! - 用户自定义模式（UUID id, is_builtin=0）
//! - 内置模式不可删除（is_builtin=1）

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── SQL 语句 ────────────────────────────────────────────────────────────────

const LOOP_PATTERN_UPSERT_SQL: &str = "\
INSERT INTO loop_patterns (\
    id, name, description, goal, cadence, risk_level, phases, human_gates,\
    cost_config, skills_required, is_active, is_builtin, created_at, updated_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
ON CONFLICT(id) DO UPDATE SET \
    name = excluded.name, \
    description = excluded.description, \
    goal = excluded.goal, \
    cadence = excluded.cadence, \
    risk_level = excluded.risk_level, \
    phases = excluded.phases, \
    human_gates = excluded.human_gates, \
    cost_config = excluded.cost_config, \
    skills_required = excluded.skills_required, \
    is_active = excluded.is_active, \
    updated_at = excluded.updated_at";

const LOOP_PATTERN_SELECT_COLUMNS: &str = "\
id, name, description, goal, cadence, risk_level, phases, human_gates,\
cost_config, skills_required, is_active, is_builtin, created_at, updated_at";

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 循环模式行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopPatternRow {
    /// 模式 ID
    pub id: String,
    /// 模式名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 目标
    pub goal: Option<String>,
    /// 执行节奏
    pub cadence: String,
    /// 风险等级
    pub risk_level: String,
    /// 阶段 JSON
    pub phases: Option<String>,
    /// 人工审核门 JSON
    pub human_gates: Option<String>,
    /// 成本配置 JSON
    pub cost_config: Option<String>,
    /// 所需技能 JSON
    pub skills_required: Option<String>,
    /// 是否激活
    pub is_active: i64,
    /// 是否内置
    pub is_builtin: i64,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到循环模式行
fn map_loop_pattern_row(row: &rusqlite::Row) -> rusqlite::Result<LoopPatternRow> {
    Ok(LoopPatternRow {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        goal: row.get(3)?,
        cadence: row.get(4)?,
        risk_level: row.get(5)?,
        phases: row.get(6)?,
        human_gates: row.get(7)?,
        cost_config: row.get(8)?,
        skills_required: row.get(9)?,
        is_active: row.get(10)?,
        is_builtin: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 插入或更新循环模式
    pub fn upsert_loop_pattern(&self, row: &LoopPatternRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        conn.execute(
            LOOP_PATTERN_UPSERT_SQL,
            params![
                &row.id,
                &row.name,
                row.description.as_deref().unwrap_or(""),
                row.goal.as_deref().unwrap_or(""),
                &row.cadence,
                &row.risk_level,
                row.phases.as_deref().unwrap_or("[]"),
                row.human_gates.as_deref().unwrap_or("[]"),
                row.cost_config.as_deref().unwrap_or("{}"),
                row.skills_required.as_deref().unwrap_or("[]"),
                row.is_active,
                row.is_builtin,
                &row.created_at,
                &row.updated_at,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 列出所有循环模式
    pub fn list_loop_patterns(&self) -> Result<Vec<LoopPatternRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM loop_patterns ORDER BY is_builtin DESC, name ASC",
                LOOP_PATTERN_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map([], map_loop_pattern_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 获取单个循环模式
    pub fn get_loop_pattern(&self, pattern_id: &str) -> Result<Option<LoopPatternRow>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            &format!(
                "SELECT {} FROM loop_patterns WHERE id = ?1",
                LOOP_PATTERN_SELECT_COLUMNS
            ),
            params![pattern_id],
            map_loop_pattern_row,
        );
        match result {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    /// 删除循环模式（内置模式不可删除）
    pub fn delete_loop_pattern(&self, pattern_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM loop_patterns WHERE id = ?1 AND is_builtin = 0",
            params![pattern_id],
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

    fn make_row(id: &str, name: &str, is_builtin: i64) -> LoopPatternRow {
        LoopPatternRow {
            id: id.to_string(),
            name: name.to_string(),
            description: Some("test pattern".to_string()),
            goal: Some("test goal".to_string()),
            cadence: "manual".to_string(),
            risk_level: "low".to_string(),
            phases: None,
            human_gates: Some("[]".to_string()),
            cost_config: None,
            skills_required: Some("[]".to_string()),
            is_active: 1,
            is_builtin,
            created_at: "2026-07-13T10:00:00Z".to_string(),
            updated_at: "2026-07-13T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn upsert_and_list_roundtrip() {
        let db = make_in_memory_db();
        db.upsert_loop_pattern(&make_row("chapter-write-loop", "Chapter Write", 1)).unwrap();
        db.upsert_loop_pattern(&make_row("user-1", "My Pattern", 0)).unwrap();

        let patterns = db.list_loop_patterns().unwrap();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].id, "chapter-write-loop");
        assert_eq!(patterns[1].id, "user-1");
    }

    #[test]
    fn upsert_updates_existing() {
        let db = make_in_memory_db();
        db.upsert_loop_pattern(&make_row("pat-1", "Original", 0)).unwrap();

        let mut updated = make_row("pat-1", "Updated", 0);
        updated.goal = Some("new goal".to_string());
        db.upsert_loop_pattern(&updated).unwrap();

        let fetched = db.get_loop_pattern("pat-1").unwrap().unwrap();
        assert_eq!(fetched.name, "Updated");
        assert_eq!(fetched.goal.as_deref(), Some("new goal"));
    }

    #[test]
    fn delete_user_pattern_succeeds() {
        let db = make_in_memory_db();
        db.upsert_loop_pattern(&make_row("user-1", "User Pattern", 0)).unwrap();

        let deleted = db.delete_loop_pattern("user-1").unwrap();
        assert!(deleted);

        let fetched = db.get_loop_pattern("user-1").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn delete_builtin_pattern_fails() {
        let db = make_in_memory_db();
        db.upsert_loop_pattern(&make_row("chapter-write-loop", "Chapter Write", 1)).unwrap();

        let deleted = db.delete_loop_pattern("chapter-write-loop").unwrap();
        assert!(!deleted);

        let fetched = db.get_loop_pattern("chapter-write-loop").unwrap();
        assert!(fetched.is_some());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = make_in_memory_db();
        let fetched = db.get_loop_pattern("nonexistent").unwrap();
        assert!(fetched.is_none());
    }
}