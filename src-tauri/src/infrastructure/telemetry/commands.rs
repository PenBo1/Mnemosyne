// Telemetry IPC 命令:trace 查询、metric 查询、聚合、总览统计。
//
// 供仪表盘 Traces 面板、Metrics 面板、调用链可视化调用。
// 与 security_kernel::commands（审计）正交：本模块关注性能与调用链。

use tauri::State;

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::metric::{MetricBucket, MetricPointRow, MetricStats};
use crate::infrastructure::db::stores::trace::{SpanRow, SpanStats, TraceSummary};
use crate::shared::error::{AppError, IpcResponse};

// ── Trace 查询 ──

/// 列出最近的 trace（每个 trace 含 first_start / last_end / span_count）。
/// limit 默认 50，上限 200。
#[tauri::command]
pub async fn telemetry_list_traces(
    limit: Option<i64>,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<TraceSummary>>, AppError> {
    let limit = limit.unwrap_or(50);
    let traces = state.db.list_recent_traces(limit)?;
    Ok(IpcResponse::ok(traces))
}

/// 获取指定 trace 的所有 span（按 start_time ASC，便于构建调用树）。
#[tauri::command]
pub async fn telemetry_get_trace(
    trace_id: String,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<SpanRow>>, AppError> {
    if trace_id.trim().is_empty() {
        return Err(AppError::bad_request("trace_id cannot be empty"));
    }
    let spans = state.db.list_spans_by_trace(&trace_id, 1000)?;
    Ok(IpcResponse::ok(spans))
}

/// 按 name 列出 span（用于查询特定操作的所有执行记录）。
/// limit 默认 50，上限 1000。
#[tauri::command]
pub async fn telemetry_list_spans(
    name: Option<String>,
    limit: Option<i64>,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<SpanRow>>, AppError> {
    let limit = limit.unwrap_or(50);
    let spans = match name {
        Some(n) if !n.trim().is_empty() => state.db.list_spans_by_name(&n, limit)?,
        _ => state.db.list_recent_spans(limit)?,
    };
    Ok(IpcResponse::ok(spans))
}

// ── Metric 查询 ──

/// 查询指定 metric 在时间范围内的数据点。
/// from/to 为可选 unix ms，limit 默认 1000 上限 5000。
#[tauri::command]
pub async fn telemetry_query_metrics(
    name: String,
    from: Option<i64>,
    to: Option<i64>,
    limit: Option<i64>,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<MetricPointRow>>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::bad_request("name cannot be empty"));
    }
    let limit = limit.unwrap_or(1000);
    let points = state.db.query_metrics(&name, from, to, limit)?;
    Ok(IpcResponse::ok(points))
}

/// 按时间桶聚合 metric（用于时间序列可视化）。
/// interval_ms 为桶宽（毫秒）。
#[tauri::command]
pub async fn telemetry_aggregate_metrics(
    name: String,
    from: Option<i64>,
    to: Option<i64>,
    interval_ms: i64,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<MetricBucket>>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::bad_request("name cannot be empty"));
    }
    let buckets = state.db.aggregate_metrics(&name, from, to, interval_ms)?;
    Ok(IpcResponse::ok(buckets))
}

// ── 总览统计 ──

/// Telemetry 总览统计:span 数 / trace 数 / error 率 / metric 数。
#[tauri::command]
pub async fn telemetry_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<TelemetryOverview>, AppError> {
    let span_stats: SpanStats = state.db.span_stats()?;
    let metric_stats: MetricStats = state.db.metric_stats()?;
    let error_rate = if span_stats.total_spans > 0 {
        span_stats.error_spans as f64 / span_stats.total_spans as f64
    } else {
        0.0
    };
    Ok(IpcResponse::ok(TelemetryOverview {
        span_stats,
        metric_stats,
        error_rate,
    }))
}

/// Telemetry 总览 DTO。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOverview {
    pub span_stats: SpanStats,
    pub metric_stats: MetricStats,
    pub error_rate: f64,
}
