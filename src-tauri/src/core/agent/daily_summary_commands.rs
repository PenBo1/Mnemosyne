//! ═══════════════════════════════════════════════════════════════════════════
//! Daily Summary Commands - 每日摘要 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 暴露 DailySummaryTask 的控制能力给前端:
//! - daily_summary_trigger: 手动触发一次摘要生成
//! - daily_summary_get_config: 读取当前配置
//! - daily_summary_update_config: 更新配置
//! - daily_summary_start: 启动定时任务
//! - daily_summary_stop: 停止定时任务
//! - daily_summary_is_running: 查询任务是否在运行

use tauri::State;
use std::time::Instant;

use crate::core::agent::daily_summary::{DailySummaryConfig, DailySummaryReport, DailySummaryState};
use crate::shared::error::{AppError, IpcResponse};

// ── 配置参数边界 ────────────────────────────────────────────────────────────

/// 最低间隔 (1 分钟)
const MIN_INTERVAL_MS: u64 = 60_000;
/// 最高间隔 (7 天)
const MAX_INTERVAL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
/// 最大回看天数
const MAX_LOOKBACK_DAYS: u32 = 30;
/// 最大衰减天数
const MAX_STALE_CUTOFF_DAYS: u32 = 365;

/// 验证配置参数
fn validate_config(cfg: &DailySummaryConfig) -> Result<(), AppError> {
    if cfg.interval_ms < MIN_INTERVAL_MS {
        return Err(AppError::invalid_input(format!(
            "intervalMs 不能小于 {}(1 分钟), got {}",
            MIN_INTERVAL_MS, cfg.interval_ms
        )));
    }
    if cfg.interval_ms > MAX_INTERVAL_MS {
        return Err(AppError::invalid_input(format!(
            "intervalMs 不能大于 {}(7 天), got {}",
            MAX_INTERVAL_MS, cfg.interval_ms
        )));
    }
    if cfg.lookback_days == 0 || cfg.lookback_days > MAX_LOOKBACK_DAYS {
        return Err(AppError::invalid_input(format!(
            "lookbackDays 必须在 1..={} 范围内, got {}",
            MAX_LOOKBACK_DAYS, cfg.lookback_days
        )));
    }
    if cfg.stale_cutoff_days == 0 || cfg.stale_cutoff_days > MAX_STALE_CUTOFF_DAYS {
        return Err(AppError::invalid_input(format!(
            "staleCutoffDays 必须在 1..={} 范围内, got {}",
            MAX_STALE_CUTOFF_DAYS, cfg.stale_cutoff_days
        )));
    }
    Ok(())
}

// ── IPC 命令 ────────────────────────────────────────────────────────────────

/// 手动触发一次摘要生成
#[tauri::command]
pub async fn daily_summary_trigger(
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<DailySummaryReport>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_trigger: enter");
    
    let report = state
        .task
        .clone()
        .trigger_once(state.engine.clone(), state.db.clone(), state.data_dir.clone())
        .await.map_err(|e| {
            tracing::error!(error = %e, "daily_summary_trigger: Failed to trigger summary");
            e
        })?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_trigger: exit"
    );
    Ok(IpcResponse::ok(report))
}

/// 读取当前配置
#[tauri::command]
pub async fn daily_summary_get_config(
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<DailySummaryConfig>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_get_config: enter");
    
    let cfg = state.task.config().await;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_get_config: exit"
    );
    Ok(IpcResponse::ok(cfg))
}

/// 更新配置 (运行时生效)
#[tauri::command]
pub async fn daily_summary_update_config(
    config: DailySummaryConfig,
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<DailySummaryConfig>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_update_config: enter");
    
    validate_config(&config)?;
    state.task.update_config(config.clone()).await;
    
    tracing::info!(
        interval_ms = config.interval_ms,
        lookback_days = config.lookback_days,
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_update_config: exit"
    );
    Ok(IpcResponse::ok(config))
}

/// 启动定时任务
#[tauri::command]
pub async fn daily_summary_start(
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_start: enter");
    
    state
        .task
        .clone()
        .start(state.engine.clone(), state.db.clone(), state.data_dir.clone())
        .await.map_err(|e| {
            tracing::error!(error = %e, "daily_summary_start: Failed to start task");
            e
        })?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_start: exit"
    );
    Ok(IpcResponse::ok(true))
}

/// 停止定时任务
#[tauri::command]
pub async fn daily_summary_stop(
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_stop: enter");
    
    state.task.stop().await;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_stop: exit"
    );
    Ok(IpcResponse::ok(true))
}

/// 查询任务是否在运行
#[tauri::command]
pub async fn daily_summary_is_running(
    state: State<'_, DailySummaryState>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!("daily_summary_is_running: enter");
    
    let running = state.task.is_running().await;
    
    tracing::info!(
        running,
        duration_ms = start.elapsed().as_millis(),
        "daily_summary_is_running: exit"
    );
    Ok(IpcResponse::ok(running))
}