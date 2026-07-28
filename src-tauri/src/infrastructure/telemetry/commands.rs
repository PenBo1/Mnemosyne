//! ═══════════════════════════════════════════════════════════════════════════
//! 遥测命令 - IPC 命令接口
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::time::Instant;

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
    let start = Instant::now();
    tracing::info!(limit = ?limit, "telemetry_list_traces: enter");
    
    let limit_val = limit.unwrap_or(50);
    let traces = state.db.list_recent_traces(limit_val).map_err(|e| {
        tracing::error!(error = %e, "telemetry_list_traces: Failed to list traces");
        e
    })?;
    
    tracing::info!(
        count = traces.len(),
        duration_ms = start.elapsed().as_millis(),
        "telemetry_list_traces: exit"
    );
    Ok(IpcResponse::ok(traces))
}

/// 获取指定 trace 的所有 span（按 start_time ASC，便于构建调用树）。
#[tauri::command]
pub async fn telemetry_get_trace(
    trace_id: String,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<SpanRow>>, AppError> {
    let start = Instant::now();
    tracing::info!(trace_id = %trace_id, "telemetry_get_trace: enter");
    
    if trace_id.trim().is_empty() {
        tracing::error!("telemetry_get_trace: trace_id cannot be empty");
        return Err(AppError::bad_request("trace_id cannot be empty"));
    }
    
    let spans = state.db.list_spans_by_trace(&trace_id, 1000).map_err(|e| {
        tracing::error!(trace_id = %trace_id, error = %e, "telemetry_get_trace: Failed to get trace");
        e
    })?;
    
    tracing::info!(
        trace_id = %trace_id,
        span_count = spans.len(),
        duration_ms = start.elapsed().as_millis(),
        "telemetry_get_trace: exit"
    );
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
    let start = Instant::now();
    tracing::info!(
        name = ?name,
        limit = ?limit,
        "telemetry_list_spans: enter"
    );
    
    let limit_val = limit.unwrap_or(50);
    let spans = match name {
        Some(n) if !n.trim().is_empty() => state.db.list_spans_by_name(&n, limit_val).map_err(|e| {
            tracing::error!(name = %n, error = %e, "telemetry_list_spans: Failed to list spans by name");
            e
        })?,
        _ => state.db.list_recent_spans(limit_val).map_err(|e| {
            tracing::error!(error = %e, "telemetry_list_spans: Failed to list recent spans");
            e
        })?,
    };
    
    tracing::info!(
        count = spans.len(),
        duration_ms = start.elapsed().as_millis(),
        "telemetry_list_spans: exit"
    );
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
    let start = Instant::now();
    tracing::info!(
        name = %name,
        from = ?from,
        to = ?to,
        limit = ?limit,
        "telemetry_query_metrics: enter"
    );
    
    if name.trim().is_empty() {
        tracing::error!("telemetry_query_metrics: name cannot be empty");
        return Err(AppError::bad_request("name cannot be empty"));
    }
    
    let limit_val = limit.unwrap_or(1000);
    let points = state.db.query_metrics(&name, from, to, limit_val).map_err(|e| {
        tracing::error!(name = %name, error = %e, "telemetry_query_metrics: Failed to query metrics");
        e
    })?;
    
    tracing::info!(
        name = %name,
        count = points.len(),
        duration_ms = start.elapsed().as_millis(),
        "telemetry_query_metrics: exit"
    );
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
    let start = Instant::now();
    tracing::info!(
        name = %name,
        from = ?from,
        to = ?to,
        interval_ms,
        "telemetry_aggregate_metrics: enter"
    );
    
    if name.trim().is_empty() {
        tracing::error!("telemetry_aggregate_metrics: name cannot be empty");
        return Err(AppError::bad_request("name cannot be empty"));
    }
    
    let buckets = state.db.aggregate_metrics(&name, from, to, interval_ms).map_err(|e| {
        tracing::error!(name = %name, error = %e, "telemetry_aggregate_metrics: Failed to aggregate metrics");
        e
    })?;
    
    tracing::info!(
        name = %name,
        bucket_count = buckets.len(),
        duration_ms = start.elapsed().as_millis(),
        "telemetry_aggregate_metrics: exit"
    );
    Ok(IpcResponse::ok(buckets))
}

// ── 总览统计 ──

/// Telemetry 总览统计:span 数 / trace 数 / error 率 / metric 数。
#[tauri::command]
pub async fn telemetry_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<TelemetryOverview>, AppError> {
    let start = Instant::now();
    tracing::info!("telemetry_stats: enter");
    
    let span_stats: SpanStats = state.db.span_stats().map_err(|e| {
        tracing::error!(error = %e, "telemetry_stats: Failed to get span stats");
        e
    })?;
    let metric_stats: MetricStats = state.db.metric_stats().map_err(|e| {
        tracing::error!(error = %e, "telemetry_stats: Failed to get metric stats");
        e
    })?;
    let error_rate = if span_stats.total_spans > 0 {
        span_stats.error_spans as f64 / span_stats.total_spans as f64
    } else {
        0.0
    };
    
    tracing::info!(
        total_spans = span_stats.total_spans,
        total_traces = span_stats.total_traces,
        error_rate,
        duration_ms = start.elapsed().as_millis(),
        "telemetry_stats: exit"
    );
    
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