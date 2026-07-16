// loop_runs 表的 CRUD —— Loop-Engineering 运行日志持久化。
//
// - append-only:只追加,不修改(结束/累计通过专用 update API)
// - 全局可观测性:所有 pattern 的运行记录汇聚于此
// - GC 30 天:由 GC 模块调用 delete_loop_runs_before()
//
// 架构约束(AGENTS.md):
// - infrastructure 层只依赖 shared/,不依赖 core/agent/
// - 因此本模块定义自己的 LoopRunRow DTO(类比 AuditEventRow),
//   pattern_id / outcome 用 String 表示
// - 业务层(core/agent/loop_engine/)负责 LoopRunRow ↔ LoopRun 转换
//
// 与 budget::daily_token_usage 的关系:
// - budget::daily_token_usage 直接查 SUM(total_tokens),不经过本模块的 Row 类型
// - 本模块提供完整行级的 list/insert/update,供 IPC 层或 dashboard 使用

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const LOOP_RUN_INSERT_SQL: &str = "\
INSERT INTO loop_runs (\
    run_id, pattern_id, book_id, chapter_number, started_at, ended_at, duration_s,\
    outcome, items_found, actions_taken, escalations, tokens_estimate,\
    prompt_tokens, completion_tokens, total_tokens, attempts, notes, loop_state_id,\
    findings_json, actions_json, escalations_json, phase_results_json, error_message\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const LOOP_RUN_SELECT_COLUMNS: &str = "\
run_id, pattern_id, book_id, chapter_number, started_at, ended_at, duration_s,\
outcome, items_found, actions_taken, escalations, tokens_estimate,\
prompt_tokens, completion_tokens, total_tokens, attempts, notes, loop_state_id,\
findings_json, actions_json, escalations_json, phase_results_json, error_message";

/// Loop-Engineering 运行记录的 DB 行级表示。
///
/// pattern_id / outcome 为 String(原始字符串),由业务层转换为强类型枚举。
/// 字段命名与 DB 列名一致(snake_case),前端通过 IPC 序列化为 camelCase。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRunRow {
    pub run_id: String,
    pub pattern_id: String,
    pub book_id: Option<String>,
    pub chapter_number: Option<u32>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_s: Option<u64>,
    /// "running" / "report-only" / "fix-proposed" / "escalated" / "no-op" / "failed"
    pub outcome: String,
    pub items_found: u32,
    pub actions_taken: u32,
    pub escalations: u32,
    pub tokens_estimate: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub attempts: u32,
    pub notes: Option<String>,
    /// 关联的 loop_state ID(可为空,兼容 budget 系统的全局运行记录)
    pub loop_state_id: Option<String>,
    /// JSON 数组:发现项详情(可为空)
    pub findings_json: Option<String>,
    /// JSON 数组:执行操作详情(可为空)
    pub actions_json: Option<String>,
    /// JSON 数组:升级事项详情(可为空)
    pub escalations_json: Option<String>,
    /// JSON 数组:分阶段结果(可为空)
    pub phase_results_json: Option<String>,
    /// 失败时的错误信息(可为空)
    pub error_message: Option<String>,
}

fn map_loop_run_row(row: &rusqlite::Row) -> rusqlite::Result<LoopRunRow> {
    Ok(LoopRunRow {
        run_id: row.get(0)?,
        pattern_id: row.get(1)?,
        book_id: row.get(2)?,
        chapter_number: row.get::<_, Option<i64>>(3)?.map(|n| n as u32),
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        duration_s: row.get::<_, Option<i64>>(6)?.map(|n| n as u64),
        outcome: row.get(7)?,
        items_found: row.get::<_, i64>(8)? as u32,
        actions_taken: row.get::<_, i64>(9)? as u32,
        escalations: row.get::<_, i64>(10)? as u32,
        tokens_estimate: row.get::<_, i64>(11)? as u64,
        prompt_tokens: row.get::<_, i64>(12)? as u64,
        completion_tokens: row.get::<_, i64>(13)? as u64,
        total_tokens: row.get::<_, i64>(14)? as u64,
        attempts: row.get::<_, i64>(15)? as u32,
        notes: row.get(16)?,
        loop_state_id: row.get(17)?,
        findings_json: row.get(18)?,
        actions_json: row.get(19)?,
        escalations_json: row.get(20)?,
        phase_results_json: row.get(21)?,
        error_message: row.get(22)?,
    })
}

impl Database {
    /// 追加一条 Loop-Engineering 运行记录。
    ///
    /// 设计为 append-only:运行结束时调用 update_loop_run_ended() 而非重新 insert。
    /// run_id 冲突(同毫秒并发)时返回 Err。
    pub fn insert_loop_run(&self, row: &LoopRunRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        let book_id = row.book_id.as_deref();
        let chapter_number = row.chapter_number.map(|n| n as i64);
        let duration_s = row.duration_s.map(|n| n as i64);
        let notes = row.notes.as_deref();
        let loop_state_id = row.loop_state_id.as_deref();
        let findings_json = row.findings_json.as_deref();
        let actions_json = row.actions_json.as_deref();
        let escalations_json = row.escalations_json.as_deref();
        let phase_results_json = row.phase_results_json.as_deref();
        let error_message = row.error_message.as_deref();
        conn.execute(
            LOOP_RUN_INSERT_SQL,
            params![
                &row.run_id,
                &row.pattern_id,
                book_id,
                chapter_number,
                &row.started_at,
                &row.ended_at,
                duration_s,
                &row.outcome,
                row.items_found as i64,
                row.actions_taken as i64,
                row.escalations as i64,
                row.tokens_estimate as i64,
                row.prompt_tokens as i64,
                row.completion_tokens as i64,
                row.total_tokens as i64,
                row.attempts as i64,
                notes,
                loop_state_id,
                findings_json,
                actions_json,
                escalations_json,
                phase_results_json,
                error_message,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 更新运行结束状态(outcome / ended_at / duration_s / counts / notes)。
    ///
    /// 用 run_id 作为唯一键定位(token 用量通过 update_loop_run_usage 单独累积,
    /// 避免每次 add_usage 都重写整个行)。
    pub fn update_loop_run_ended(
        &self,
        run_id: &str,
        outcome: &str,
        duration_s: u64,
        items_found: u32,
        actions_taken: u32,
        escalations: u32,
        attempts: u32,
        notes: Option<&str>,
    ) -> Result<bool, AppError> {
        let ended_at = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE loop_runs SET \
                outcome = ?1, ended_at = ?2, duration_s = ?3, \
                items_found = ?4, actions_taken = ?5, escalations = ?6, \
                attempts = ?7, notes = ?8 \
             WHERE run_id = ?9",
            params![
                outcome,
                &ended_at,
                duration_s as i64,
                items_found as i64,
                actions_taken as i64,
                escalations as i64,
                attempts as i64,
                notes,
                run_id,
            ],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 累加 token 用量(每次 add_usage 调用,而非全量替换)。
    pub fn update_loop_run_usage(
        &self,
        run_id: &str,
        prompt_tokens_delta: u64,
        completion_tokens_delta: u64,
    ) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "UPDATE loop_runs SET \
                prompt_tokens = prompt_tokens + ?1, \
                completion_tokens = completion_tokens + ?2, \
                total_tokens = prompt_tokens + completion_tokens + ?1 + ?2, \
                tokens_estimate = prompt_tokens + completion_tokens + ?1 + ?2 \
             WHERE run_id = ?3",
            params![
                prompt_tokens_delta as i64,
                completion_tokens_delta as i64,
                run_id,
            ],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }

    /// 列出最近 N 条运行记录(按 started_at DESC)。
    /// limit 上限 1000,避免大表全扫。
    pub fn list_recent_loop_runs(&self, limit: i64) -> Result<Vec<LoopRunRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM loop_runs ORDER BY started_at DESC LIMIT ?1",
                LOOP_RUN_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map([limit], map_loop_run_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出指定 loop_state 的运行记录(按 started_at DESC)。
    /// limit 上限 1000,避免大表全扫。
    pub fn list_loop_runs_by_state(&self, state_id: &str, limit: i64) -> Result<Vec<LoopRunRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM loop_runs WHERE loop_state_id = ?1 ORDER BY started_at DESC LIMIT ?2",
                LOOP_RUN_SELECT_COLUMNS
            )
        ).map_err(db_err)?;
        let rows = stmt.query_map(params![state_id, limit], map_loop_run_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// GC:删除 started_at 早于 cutoff_iso 的运行记录。
    /// 用于 30 天滚动日志保留。
    /// 返回删除的行数。
    pub fn delete_loop_runs_before(&self, cutoff_iso: &str) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM loop_runs WHERE started_at < ?1",
            params![cutoff_iso],
        ).map_err(db_err)?;
        Ok(affected as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    fn make_row(run_id: &str, pattern_id: &str, outcome: &str, total_tokens: u64) -> LoopRunRow {
        LoopRunRow {
            run_id: run_id.to_string(),
            pattern_id: pattern_id.to_string(),
            book_id: Some("book-1".to_string()),
            chapter_number: Some(1),
            started_at: run_id.to_string(),
            ended_at: None,
            duration_s: None,
            outcome: outcome.to_string(),
            items_found: 3,
            actions_taken: 2,
            escalations: 0,
            tokens_estimate: total_tokens,
            prompt_tokens: total_tokens / 2,
            completion_tokens: total_tokens / 2,
            total_tokens,
            attempts: 1,
            notes: Some("test run".to_string()),
            loop_state_id: None,
            findings_json: None,
            actions_json: None,
            escalations_json: None,
            phase_results_json: None,
            error_message: None,
        }
    }

    #[test]
    fn insert_and_list_roundtrip() {
        let db = make_in_memory_db();
        let row = make_row(
            "2026-07-13T10:00:00Z",
            "audit-revise-loop",
            "fix-proposed",
            50_000,
        );
        db.insert_loop_run(&row).unwrap();

        let listed = db.list_recent_loop_runs(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].run_id, "2026-07-13T10:00:00Z");
        assert_eq!(listed[0].pattern_id, "audit-revise-loop");
        assert_eq!(listed[0].total_tokens, 50_000);
        assert_eq!(listed[0].outcome, "fix-proposed");
    }

    #[test]
    fn update_ended_marks_finished() {
        let db = make_in_memory_db();
        let row = make_row(
            "2026-07-13T11:00:00Z",
            "audit-revise-loop",
            "running",
            10_000,
        );
        db.insert_loop_run(&row).unwrap();

        let updated = db.update_loop_run_ended(
            "2026-07-13T11:00:00Z",
            "escalated",
            120,
            4,
            1,
            2,
            3,
            Some("escalated to human"),
        ).unwrap();
        assert!(updated);

        let listed = db.list_recent_loop_runs(10).unwrap();
        assert_eq!(listed[0].outcome, "escalated");
        assert!(listed[0].ended_at.is_some());
        assert_eq!(listed[0].duration_s, Some(120));
        assert_eq!(listed[0].escalations, 2);
        assert_eq!(listed[0].attempts, 3);
    }

    #[test]
    fn update_usage_accumulates() {
        let db = make_in_memory_db();
        let row = make_row(
            "2026-07-13T12:00:00Z",
            "observation-loop",
            "running",
            1000, // 初始 total
        );
        db.insert_loop_run(&row).unwrap();

        let updated = db.update_loop_run_usage(
            "2026-07-13T12:00:00Z",
            500, // prompt delta
            250, // completion delta
        ).unwrap();
        assert!(updated);

        let listed = db.list_recent_loop_runs(10).unwrap();
        assert_eq!(listed[0].prompt_tokens, 1000); // 500(initial) + 500(delta)
        assert_eq!(listed[0].completion_tokens, 750); // 500(initial) + 250(delta)
        assert_eq!(listed[0].total_tokens, 1750);
    }

    #[test]
    fn delete_before_gc_old_records() {
        let db = make_in_memory_db();
        db.insert_loop_run(&make_row(
            "2026-06-01T00:00:00Z",
            "audit-revise-loop",
            "no-op",
            100,
        )).unwrap();
        db.insert_loop_run(&make_row(
            "2026-07-13T00:00:00Z",
            "audit-revise-loop",
            "no-op",
            200,
        )).unwrap();

        let deleted = db.delete_loop_runs_before("2026-07-01T00:00:00Z").unwrap();
        assert_eq!(deleted, 1);

        let remaining = db.list_recent_loop_runs(100).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].run_id, "2026-07-13T00:00:00Z");
    }

    #[test]
    fn list_by_state_filters_correctly() {
        let db = make_in_memory_db();
        let mut row1 = make_row("2026-07-13T10:00:00Z", "audit-revise-loop", "running", 1000);
        row1.loop_state_id = Some("state-1".to_string());
        let mut row2 = make_row("2026-07-13T11:00:00Z", "observation-loop", "no-op", 500);
        row2.loop_state_id = Some("state-1".to_string());
        let mut row3 = make_row("2026-07-13T12:00:00Z", "audit-revise-loop", "running", 2000);
        row3.loop_state_id = Some("state-2".to_string());

        db.insert_loop_run(&row1).unwrap();
        db.insert_loop_run(&row2).unwrap();
        db.insert_loop_run(&row3).unwrap();

        let state1_runs = db.list_loop_runs_by_state("state-1", 100).unwrap();
        assert_eq!(state1_runs.len(), 2);
        // 按 started_at DESC 排序
        assert_eq!(state1_runs[0].run_id, "2026-07-13T11:00:00Z");
        assert_eq!(state1_runs[1].run_id, "2026-07-13T10:00:00Z");

        let state2_runs = db.list_loop_runs_by_state("state-2", 100).unwrap();
        assert_eq!(state2_runs.len(), 1);
        assert_eq!(state2_runs[0].run_id, "2026-07-13T12:00:00Z");

        let state3_runs = db.list_loop_runs_by_state("state-3", 100).unwrap();
        assert_eq!(state3_runs.len(), 0);
    }
}
