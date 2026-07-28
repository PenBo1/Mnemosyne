//! ═══════════════════════════════════════════════════════════════════════════
//! 循环状态存储 - Loop-Engineering 状态实例持久化
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计要点：
//! - 每小说的循环实例：绑定 pattern + 状态 + 预算用量
//! - JSON 字段（state_payload / config / last_run_result）由业务层序列化

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── SQL 语句 ────────────────────────────────────────────────────────────────

const LOOP_STATE_INSERT_SQL: &str = "\
INSERT INTO loop_states (\
    id, novel_id, pattern_id, status, readiness_level, state_payload, config,\
    token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const LOOP_STATE_SELECT_COLUMNS: &str = "\
id, novel_id, pattern_id, status, readiness_level, state_payload, config,\
token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at";

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 循环状态行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStateRow {
    /// 状态 ID
    pub id: String,
    /// 小说 ID
    pub novel_id: String,
    /// 模式 ID
    pub pattern_id: String,
    /// 状态
    pub status: String,
    /// 就绪等级
    pub readiness_level: String,
    /// 状态载荷 JSON
    pub state_payload: Option<String>,
    /// 配置 JSON
    pub config: Option<String>,
    /// 今日 Token 用量
    pub token_usage_today: i64,
    /// 每日 Token 上限
    pub token_cap_daily: i64,
    /// 最后运行时间
    pub last_run_at: Option<String>,
    /// 最后运行结果 JSON
    pub last_run_result: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到循环状态行
fn map_loop_state_row(row: &rusqlite::Row) -> rusqlite::Result<LoopStateRow> {
    Ok(LoopStateRow {
        id: row.get(0)?,
        novel_id: row.get(1)?,
        pattern_id: row.get(2)?,
        status: row.get(3)?,
        readiness_level: row.get(4)?,
        state_payload: row.get(5)?,
        config: row.get(6)?,
        token_usage_today: row.get(7)?,
        token_cap_daily: row.get(8)?,
        last_run_at: row.get(9)?,
        last_run_result: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 创建循环状态
    pub fn insert_loop_state(&self, row: &LoopStateRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        conn.execute(
            LOOP_STATE_INSERT_SQL,
            params![
                &row.id,
                &row.novel_id,
                &row.pattern_id,
                &row.status,
                &row.readiness_level,
                row.state_payload.as_deref().unwrap_or("{}"),
                row.config.as_deref().unwrap_or("{}"),
                row.token_usage_today,
                row.token_cap_daily,
                &row.last_run_at,
                &row.last_run_result,
                &row.created_at,
                &row.updated_at,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 列出指定小说的所有循环状态
    pub fn list_loop_states(&self, novel_id: &str) -> Result<Vec<LoopStateRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM loop_states WHERE novel_id = ?1 ORDER BY created_at ASC",
                LOOP_STATE_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map([novel_id], map_loop_state_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 获取单个循环状态
    pub fn get_loop_state(&self, state_id: &str) -> Result<Option<LoopStateRow>, AppError> {
        let conn = self.conn()?;
        let result = conn.query_row(
            &format!(
                "SELECT {} FROM loop_states WHERE id = ?1",
                LOOP_STATE_SELECT_COLUMNS
            ),
            params![state_id],
            map_loop_state_row,
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(e)),
        }
    }

    /// 更新循环状态（部分更新语义）
    pub fn update_loop_state(
        &self,
        state_id: &str,
        status: Option<&str>,
        readiness_level: Option<&str>,
        state_payload: Option<&str>,
        config: Option<&str>,
        token_usage_today: Option<i64>,
        token_cap_daily: Option<i64>,
        last_run_at: Option<&str>,
        last_run_result: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE loop_states SET \
                status = COALESCE(?1, status), \
                readiness_level = COALESCE(?2, readiness_level), \
                state_payload = COALESCE(?3, state_payload), \
                config = COALESCE(?4, config), \
                token_usage_today = COALESCE(?5, token_usage_today), \
                token_cap_daily = COALESCE(?6, token_cap_daily), \
                last_run_at = COALESCE(?7, last_run_at), \
                last_run_result = COALESCE(?8, last_run_result), \
                updated_at = ?9 \
             WHERE id = ?10",
            params![
                status,
                readiness_level,
                state_payload,
                config,
                token_usage_today,
                token_cap_daily,
                last_run_at,
                last_run_result,
                updated_at,
                state_id,
            ],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 删除循环状态
    pub fn delete_loop_state(&self, state_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM loop_states WHERE id = ?1",
            params![state_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        let db = Database::connect_in_memory().expect("in-memory db should init");
        {
            let conn = db.conn().expect("db conn should lock");
            conn.execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["ws-1", "ws1", "/tmp/ws1", "2026-07-13T10:00:00Z", "2026-07-13T10:00:00Z"],
            ).expect("seed workspace ws-1");
            conn.execute(
                "INSERT INTO novels (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["novel-1", "ws-1", "n1", "2026-07-13T10:00:00Z", "2026-07-13T10:00:00Z"],
            ).expect("seed novel novel-1");
            conn.execute(
                "INSERT INTO workspaces (id, name, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["ws-2", "ws2", "/tmp/ws2", "2026-07-13T10:00:00Z", "2026-07-13T10:00:00Z"],
            ).expect("seed workspace ws-2");
            conn.execute(
                "INSERT INTO novels (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params!["novel-2", "ws-2", "n2", "2026-07-13T10:00:00Z", "2026-07-13T10:00:00Z"],
            ).expect("seed novel novel-2");
        }
        db
    }

    fn make_row(id: &str, novel_id: &str, pattern_id: &str) -> LoopStateRow {
        LoopStateRow {
            id: id.to_string(),
            novel_id: novel_id.to_string(),
            pattern_id: pattern_id.to_string(),
            status: "idle".to_string(),
            readiness_level: "L0".to_string(),
            state_payload: None,
            config: None,
            token_usage_today: 0,
            token_cap_daily: 50_000,
            last_run_at: None,
            last_run_result: None,
            created_at: "2026-07-13T10:00:00Z".to_string(),
            updated_at: "2026-07-13T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let db = make_in_memory_db();
        let row = make_row("state-1", "novel-1", "chapter-write-loop");
        db.insert_loop_state(&row).unwrap();

        let fetched = db.get_loop_state("state-1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, "state-1");
        assert_eq!(fetched.novel_id, "novel-1");
        assert_eq!(fetched.pattern_id, "chapter-write-loop");
        assert_eq!(fetched.status, "idle");
        assert_eq!(fetched.token_cap_daily, 50_000);
    }

    #[test]
    fn list_by_novel() {
        let db = make_in_memory_db();
        db.insert_loop_state(&make_row("state-1", "novel-1", "chapter-write-loop")).unwrap();
        db.insert_loop_state(&make_row("state-2", "novel-1", "audit-revise-loop")).unwrap();
        db.insert_loop_state(&make_row("state-3", "novel-2", "observation-loop")).unwrap();

        let novel1_states = db.list_loop_states("novel-1").unwrap();
        assert_eq!(novel1_states.len(), 2);

        let novel2_states = db.list_loop_states("novel-2").unwrap();
        assert_eq!(novel2_states.len(), 1);
    }

    #[test]
    fn update_changes_fields() {
        let db = make_in_memory_db();
        db.insert_loop_state(&make_row("state-1", "novel-1", "chapter-write-loop")).unwrap();

        let updated = db.update_loop_state(
            "state-1",
            Some("paused"),
            Some("L2"),
            None,
            None,
            Some(10_000),
            Some(100_000),
            Some("2026-07-13T12:00:00Z"),
            None,
            "2026-07-13T12:00:00Z",
        ).unwrap();
        assert!(updated);

        let fetched = db.get_loop_state("state-1").unwrap().unwrap();
        assert_eq!(fetched.status, "paused");
        assert_eq!(fetched.readiness_level, "L2");
        assert_eq!(fetched.token_usage_today, 10_000);
        assert_eq!(fetched.token_cap_daily, 100_000);
        assert_eq!(fetched.last_run_at.as_deref(), Some("2026-07-13T12:00:00Z"));
    }

    #[test]
    fn delete_removes_state() {
        let db = make_in_memory_db();
        db.insert_loop_state(&make_row("state-1", "novel-1", "chapter-write-loop")).unwrap();

        let deleted = db.delete_loop_state("state-1").unwrap();
        assert!(deleted);

        let fetched = db.get_loop_state("state-1").unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let db = make_in_memory_db();
        let fetched = db.get_loop_state("nonexistent").unwrap();
        assert!(fetched.is_none());
    }
}