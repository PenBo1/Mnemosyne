//! ═══════════════════════════════════════════════════════════════════════════
//! 雷达命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════

use crate::core::agent::commands::AgentState;
use crate::domain::radar::sources::{builtin_source_infos, default_sources, TextRadarSource};
use crate::domain::radar::types::RadarSourceInfo;
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::types::RadarScan;
use crate::infrastructure::validation::validate_id;
use tauri::State;
use std::time::Instant;

fn validate_scan_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "scan_id").map_err(AppError::invalid_input)
}

#[tauri::command]
pub async fn radar_scan_list(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<RadarScan>>, AppError> {
    let start = Instant::now();
    tracing::info!(limit = ?limit, "radar_scan_list: enter");
    
    let limit_val = limit.unwrap_or(50).clamp(1, 200);
    let scans = state.db.list_radar_scans(Some(limit_val)).map_err(|e| {
        tracing::error!(error = %e, "radar_scan_list: Failed to list radar scans");
        e
    })?;
    
    tracing::info!(
        count = scans.len(),
        duration_ms = start.elapsed().as_millis(),
        "radar_scan_list: exit"
    );
    Ok(IpcResponse::ok(scans))
}

#[tauri::command]
pub async fn radar_scan_delete(
    state: State<'_, DbState>,
    scan_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(scan_id = %scan_id, "radar_scan_delete: enter");
    
    validate_scan_id(&scan_id)?;
    
    let deleted = state.db.delete_radar_scan(&scan_id).map_err(|e| {
        tracing::error!(scan_id = %scan_id, error = %e, "radar_scan_delete: Failed to delete radar scan");
        e
    })?;
    
    tracing::info!(
        scan_id = %scan_id,
        deleted,
        duration_ms = start.elapsed().as_millis(),
        "radar_scan_delete: exit"
    );
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn radar_scan_create(
    state: State<'_, DbState>,
    market_summary: String,
    recommendations_json: String,
    raw_rankings_json: String,
) -> Result<IpcResponse<RadarScan>, AppError> {
    let start = Instant::now();
    tracing::info!("radar_scan_create: enter");
    
    let recommendations: Vec<crate::infrastructure::db::types::RadarRecommendation> = 
        serde_json::from_str(&recommendations_json)
            .map_err(|e| {
                tracing::error!(error = %e, "radar_scan_create: Invalid recommendations JSON");
                AppError::invalid_input(format!("Invalid recommendations JSON: {}", e))
            })?;
    
    let raw_rankings: Vec<crate::infrastructure::db::types::PlatformRankings> = 
        serde_json::from_str(&raw_rankings_json)
            .map_err(|e| {
                tracing::error!(error = %e, "radar_scan_create: Invalid raw rankings JSON");
                AppError::invalid_input(format!("Invalid raw rankings JSON: {}", e))
            })?;
    
    let scan = state.db.create_radar_scan(&market_summary, &recommendations, &raw_rankings).map_err(|e| {
        tracing::error!(error = %e, "radar_scan_create: Failed to create radar scan");
        e
    })?;
    
    tracing::info!(
        scan_id = %scan.id,
        recommendations = recommendations.len(),
        duration_ms = start.elapsed().as_millis(),
        "radar_scan_create: exit"
    );
    Ok(IpcResponse::created(scan))
}

/// 执行雷达扫描:抓取排行榜 → LLM 分析 → 存储到数据库 → 返回结果。
///
/// 前端调用此命令触发完整扫描流程。
/// `extra_texts` 可选注入外部分析文本(每条作为一个 TextRadarSource),为空时仅用内置数据源。
#[tauri::command]
pub async fn radar_scan(
    agent_state: State<'_, AgentState>,
    db_state: State<'_, DbState>,
    extra_texts: Option<Vec<String>>,
) -> Result<IpcResponse<RadarScan>, AppError> {
    let start = Instant::now();
    tracing::info!(extra_texts_count = ?extra_texts.as_ref().map(|v| v.len()), "radar_scan: enter");
    
    let sources = match extra_texts {
        Some(texts) if !texts.is_empty() => {
            let mut sources = default_sources();
            for (i, text) in texts.into_iter().enumerate() {
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                sources.push(Box::new(TextRadarSource::new(
                    trimmed,
                    format!("external-{}", i),
                )));
            }
            Some(sources)
        }
        _ => None,
    };

    let outcome = super::agent::scan(&agent_state.engine, sources).await.map_err(|e| {
        tracing::error!(error = %e, "radar_scan: Failed to scan");
        e
    })?;

    let scan = db_state.db.create_radar_scan(
        &outcome.result.market_summary,
        &outcome.result.recommendations,
        &outcome.raw_rankings,
    ).map_err(|e| {
        tracing::error!(error = %e, "radar_scan: Failed to create radar scan in DB");
        e
    })?;

    tracing::info!(
        scan_id = %scan.id,
        recommendations = outcome.result.recommendations.len(),
        duration_ms = start.elapsed().as_millis(),
        "radar_scan: exit"
    );

    Ok(IpcResponse::created(scan))
}

/// 列出可用数据源元信息(内置自动抓取源 + 文本注入源)。
#[tauri::command]
pub async fn radar_list_sources() -> Result<IpcResponse<Vec<RadarSourceInfo>>, AppError> {
    let start = Instant::now();
    tracing::info!("radar_list_sources: enter");
    
    let sources = builtin_source_infos();
    
    tracing::info!(
        count = sources.len(),
        duration_ms = start.elapsed().as_millis(),
        "radar_list_sources: exit"
    );
    Ok(IpcResponse::ok(sources))
}