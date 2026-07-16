// trace_spans 表的 CRUD —— OpenTelemetry 风格的 span 持久化。
//
// - append-only:span 在结束时一次性写入（end_time 已知）
// - GC 30 天:由 GC 模块调用 delete_spans_before()
// - 与 audit_events 正交：本表关注调用链与性能，audit_events 关注安全决策
//
// 架构约束(AGENTS.md):
// - infrastructure 层只依赖 shared/，不依赖 core/agent/
// - 因此本模块定义自己的 SpanRow DTO，业务层（telemetry::tracer）负责转换

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const SPAN_INSERT_SQL: &str = "\
INSERT INTO trace_spans (\
    id, trace_id, parent_span_id, name, kind, start_time, end_time,\
    attributes, events, status, status_message, workspace_id, session_id\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const SPAN_SELECT_COLUMNS: &str = "\
id, trace_id, parent_span_id, name, kind, start_time, end_time,\
attributes, events, status, status_message, workspace_id, session_id";

/// Trace span 的 DB 行级表示。
///
/// 字段命名与 DB 列名一致(snake_case)，前端通过 IPC 序列化为 camelCase。
/// attributes / events 为 JSON 字符串，由业务层序列化/反序列化。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanRow {
    pub id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    /// "internal" / "client" / "server" / "producer" / "consumer"
    pub kind: String,
    /// unix ms
    pub start_time: i64,
    pub end_time: Option<i64>,
    /// JSON object string
    pub attributes: String,
    /// JSON array string
    pub events: String,
    /// "ok" / "error" / "unset"
    pub status: String,
    pub status_message: Option<String>,
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
}

fn map_span_row(row: &rusqlite::Row) -> rusqlite::Result<SpanRow> {
    Ok(SpanRow {
        id: row.get(0)?,
        trace_id: row.get(1)?,
        parent_span_id: row.get(2)?,
        name: row.get(3)?,
        kind: row.get(4)?,
        start_time: row.get(5)?,
        end_time: row.get(6)?,
        attributes: row.get(7)?,
        events: row.get(8)?,
        status: row.get(9)?,
        status_message: row.get(10)?,
        workspace_id: row.get(11)?,
        session_id: row.get(12)?,
    })
}

impl Database {
    /// 写入一条 span 记录。id 冲突时返回 Err。
    pub fn insert_span(&self, row: &SpanRow) -> Result<(), AppError> {
        let conn = self.conn()?;
        conn.execute(
            SPAN_INSERT_SQL,
            params![
                &row.id,
                &row.trace_id,
                &row.parent_span_id,
                &row.name,
                &row.kind,
                row.start_time,
                row.end_time,
                &row.attributes,
                &row.events,
                &row.status,
                &row.status_message,
                &row.workspace_id,
                &row.session_id,
            ],
        )
        .map_err(db_err)?;
        Ok(())
    }

    /// 按 span ID 查询单个 span。
    pub fn get_span(&self, id: &str) -> Result<Option<SpanRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!("SELECT {} FROM trace_spans WHERE id = ?1", SPAN_SELECT_COLUMNS),
        )
        .map_err(db_err)?;
        let mut rows = stmt.query_map(params![id], map_span_row).map_err(db_err)?;
        match rows.next() {
            Some(r) => Ok(Some(r.map_err(db_err)?)),
            None => Ok(None),
        }
    }

    /// 列出指定 trace 的所有 span（按 start_time ASC，便于构建调用树）。
    /// limit 上限 1000。
    pub fn list_spans_by_trace(&self, trace_id: &str, limit: i64) -> Result<Vec<SpanRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM trace_spans WHERE trace_id = ?1 ORDER BY start_time ASC LIMIT ?2",
                SPAN_SELECT_COLUMNS
            ),
        )
        .map_err(db_err)?;
        let rows = stmt
            .query_map(params![trace_id, limit], map_span_row)
            .map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出最近的 span（跨所有 trace，按 start_time DESC）。
    /// limit 上限 1000。
    pub fn list_recent_spans(&self, limit: i64) -> Result<Vec<SpanRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM trace_spans ORDER BY start_time DESC LIMIT ?1",
                SPAN_SELECT_COLUMNS
            ),
        )
        .map_err(db_err)?;
        let rows = stmt.query_map([limit], map_span_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按 name 列出 span（按 start_time DESC）。
    /// 用于查询特定操作（如 "agent.send_message"）的所有执行记录。
    /// limit 上限 1000。
    pub fn list_spans_by_name(&self, name: &str, limit: i64) -> Result<Vec<SpanRow>, AppError> {
        let limit = limit.clamp(1, 1000);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            &format!(
                "SELECT {} FROM trace_spans WHERE name = ?1 ORDER BY start_time DESC LIMIT ?2",
                SPAN_SELECT_COLUMNS
            ),
        )
        .map_err(db_err)?;
        let rows = stmt
            .query_map(params![name, limit], map_span_row)
            .map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出最近的 trace（每个 trace 取最早 span 的时间作为 trace 起始）。
    /// 返回 (trace_id, first_start_time, span_count, last_end_time)。
    /// limit 上限 200。
    pub fn list_recent_traces(&self, limit: i64) -> Result<Vec<TraceSummary>, AppError> {
        let limit = limit.clamp(1, 200);
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "SELECT trace_id, MIN(start_time) AS first_start, MAX(COALESCE(end_time, start_time)) AS last_end, COUNT(*) AS span_count
             FROM trace_spans
             GROUP BY trace_id
             ORDER BY first_start DESC
             LIMIT ?1",
        )
        .map_err(db_err)?;
        let rows = stmt
            .query_map([limit], |row| {
                Ok(TraceSummary {
                    trace_id: row.get(0)?,
                    first_start_time: row.get(1)?,
                    last_end_time: row.get(2)?,
                    span_count: row.get::<_, i64>(3)? as u32,
                })
            })
            .map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// GC:删除 start_time 早于 cutoff_ms 的 span 记录。
    /// 返回删除的行数。
    pub fn delete_spans_before(&self, cutoff_ms: i64) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn
            .execute(
                "DELETE FROM trace_spans WHERE start_time < ?1",
                params![cutoff_ms],
            )
            .map_err(db_err)?;
        Ok(affected as u64)
    }

    /// Trace span 聚合统计:总数 / 错误数 / 平均耗时。
    pub fn span_stats(&self) -> Result<SpanStats, AppError> {
        let conn = self.conn()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_spans", [], |row| row.get(0))
            .map_err(db_err)?;
        let error_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM trace_spans WHERE status = 'error'",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        let trace_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT trace_id) FROM trace_spans",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        let avg_duration_ms: f64 = conn
            .query_row(
                "SELECT AVG(end_time - start_time) FROM trace_spans WHERE end_time IS NOT NULL",
                [],
                |row| {
                    let avg: Option<f64> = row.get(0)?;
                    Ok(avg.unwrap_or(0.0))
                },
            )
            .map_err(db_err)?;
        Ok(SpanStats {
            total_spans: total,
            error_spans: error_count,
            total_traces: trace_count,
            avg_duration_ms,
        })
    }
}

/// Trace 摘要（每个 trace 一行）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceSummary {
    pub trace_id: String,
    pub first_start_time: i64,
    pub last_end_time: i64,
    pub span_count: u32,
}

/// Span 聚合统计。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanStats {
    pub total_spans: i64,
    pub error_spans: i64,
    pub total_traces: i64,
    pub avg_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    fn make_span(id: &str, trace_id: &str, name: &str, status: &str) -> SpanRow {
        SpanRow {
            id: id.to_string(),
            trace_id: trace_id.to_string(),
            parent_span_id: None,
            name: name.to_string(),
            kind: "internal".to_string(),
            start_time: 1000,
            end_time: Some(1500),
            attributes: "{}".to_string(),
            events: "[]".to_string(),
            status: status.to_string(),
            status_message: None,
            workspace_id: None,
            session_id: None,
        }
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let db = make_in_memory_db();
        let span = make_span("span-1", "trace-1", "agent.send_message", "ok");
        db.insert_span(&span).unwrap();

        let got = db.get_span("span-1").unwrap().expect("span should exist");
        assert_eq!(got.id, "span-1");
        assert_eq!(got.trace_id, "trace-1");
        assert_eq!(got.name, "agent.send_message");
        assert_eq!(got.status, "ok");
    }

    #[test]
    fn list_by_trace_returns_all_spans() {
        let db = make_in_memory_db();
        db.insert_span(&make_span("s1", "t1", "a", "ok")).unwrap();
        db.insert_span(&make_span("s2", "t1", "b", "ok")).unwrap();
        db.insert_span(&make_span("s3", "t2", "a", "ok")).unwrap();

        let t1_spans = db.list_spans_by_trace("t1", 100).unwrap();
        assert_eq!(t1_spans.len(), 2);
        let t2_spans = db.list_spans_by_trace("t2", 100).unwrap();
        assert_eq!(t2_spans.len(), 1);
    }

    #[test]
    fn list_by_name_filters_correctly() {
        let db = make_in_memory_db();
        db.insert_span(&make_span("s1", "t1", "agent.send_message", "ok"))
            .unwrap();
        db.insert_span(&make_span("s2", "t1", "llm.chat", "ok")).unwrap();
        db.insert_span(&make_span("s3", "t2", "agent.send_message", "error"))
            .unwrap();

        let agent_spans = db.list_spans_by_name("agent.send_message", 100).unwrap();
        assert_eq!(agent_spans.len(), 2);
    }

    #[test]
    fn list_recent_traces_aggregates() {
        let db = make_in_memory_db();
        db.insert_span(&make_span("s1", "t1", "a", "ok")).unwrap();
        db.insert_span(&make_span("s2", "t1", "b", "ok")).unwrap();
        db.insert_span(&make_span("s3", "t2", "a", "ok")).unwrap();

        let traces = db.list_recent_traces(10).unwrap();
        assert_eq!(traces.len(), 2);
        let t1 = traces.iter().find(|t| t.trace_id == "t1").unwrap();
        assert_eq!(t1.span_count, 2);
    }

    #[test]
    fn span_stats_counts_errors() {
        let db = make_in_memory_db();
        db.insert_span(&make_span("s1", "t1", "a", "ok")).unwrap();
        db.insert_span(&make_span("s2", "t1", "b", "error")).unwrap();
        db.insert_span(&make_span("s3", "t2", "a", "ok")).unwrap();

        let stats = db.span_stats().unwrap();
        assert_eq!(stats.total_spans, 3);
        assert_eq!(stats.error_spans, 1);
        assert_eq!(stats.total_traces, 2);
    }

    #[test]
    fn delete_before_gc_old_spans() {
        let db = make_in_memory_db();
        let mut old = make_span("s1", "t1", "a", "ok");
        old.start_time = 100;
        let mut new_span = make_span("s2", "t2", "a", "ok");
        new_span.start_time = 2000;
        db.insert_span(&old).unwrap();
        db.insert_span(&new_span).unwrap();

        let deleted = db.delete_spans_before(1000).unwrap();
        assert_eq!(deleted, 1);

        let remaining = db.list_recent_spans(100).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "s2");
    }
}
