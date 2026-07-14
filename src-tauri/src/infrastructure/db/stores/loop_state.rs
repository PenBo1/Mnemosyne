// loop_states 表的 CRUD —— Loop-Engineering 状态实例持久化。
//
// 设计要点：
// - per-novel 的循环实例:绑定 pattern + 状态 + 预算用量
// - JSON 字段(state_payload / config / last_run_result)由业务层序列化
//
// 架构约束(AGENTS.md):
// - infrastructure 层只依赖 shared/,不依赖 core/agent/ 或 application/
// - 因此本模块定义自己的 LoopStateRow DTO,字段命名与 DB 列名一致(snake_case)
// - 业务层(application/loop_engine/)负责 LoopStateRow ↔ 前端 camelCase 类型转换

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const LOOP_STATE_INSERT_SQL: &str = "\
INSERT INTO loop_states (\
    id, novel_id, pattern_id, status, readiness_level, state_payload, config,\
    token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const LOOP_STATE_SELECT_COLUMNS: &str = "\
id, novel_id, pattern_id, status, readiness_level, state_payload, config,\
token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at";

/// loop_states 表的行级表示。
///
/// JSON 字段(state_payload / config / last_run_result)为原始 JSON 字符串,
/// 由业务层负责序列化/反序列化。字段命名与 DB 列名一致(snake_case)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStateRow {
    pub id: String,
    pub novel_id: String,
    pub pattern_id: String,
    pub status: String,
    pub readiness_level: String,
    pub state_payload: Option<String>,
    pub config: Option<String>,
    pub token_usage_today: i64,
    pub token_cap_daily: i64,
    pub last_run_at: Option<String>,
    pub last_run_result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

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

impl Database {
    /// 创建 loop state。
    pub fn insert_loop_state(&self, row: &LoopStateRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        // state_payload / config 在 DB 中为 NOT NULL DEFAULT '{}',None 时用 "{}" 兜底
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

    /// 列出指定 novel 的所有 loop states。
    pub fn list_loop_states(&self, novel_id: &str) -> Result<Vec<LoopStateRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM loop_states WHERE novel_id = ?1 ORDER BY created_at ASC",
                LOOP_STATE_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map([novel_id], map_loop_state_row).map_err(db_err)?;
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// 获取单个 loop state。
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

    /// 更新 loop state 的可变字段。
    ///
    /// 仅更新非 None 字段,None 字段保持原值(部分更新语义)。
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

    /// 删除 loop state。
    pub fn delete_loop_state(&self, state_id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM loop_states WHERE id = ?1",
            params![state_id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
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
