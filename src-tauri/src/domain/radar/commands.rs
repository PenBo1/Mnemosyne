use crate::core::agent::commands::AgentState;
use crate::domain::radar::sources::{builtin_source_infos, default_sources, TextRadarSource};
use crate::domain::radar::types::RadarSourceInfo;
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::validate_id;
use tauri::State;

fn validate_scan_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "scan_id").map_err(|e| AppError::invalid_input(e))
}

#[tauri::command]
pub async fn radar_scan_list(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::RadarScan>>, AppError> {
    let limit_val = limit.unwrap_or(50).clamp(1, 200);
    let scans = state.db.list_radar_scans(Some(limit_val))?;
    Ok(IpcResponse::ok(scans))
}

#[tauri::command]
pub async fn radar_scan_delete(
    state: State<'_, DbState>,
    scan_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_scan_id(&scan_id)?;
    let deleted = state.db.delete_radar_scan(&scan_id)?;
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn radar_scan_create(
    state: State<'_, DbState>,
    market_summary: String,
    recommendations_json: String,
    raw_rankings_json: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::RadarScan>, AppError> {
    let recommendations: Vec<crate::infrastructure::db::types::RadarRecommendation> = 
        serde_json::from_str(&recommendations_json)
            .map_err(|e| AppError::invalid_input(format!("Invalid recommendations JSON: {}", e)))?;
    
    let raw_rankings: Vec<crate::infrastructure::db::types::PlatformRankings> = 
        serde_json::from_str(&raw_rankings_json)
            .map_err(|e| AppError::invalid_input(format!("Invalid raw rankings JSON: {}", e)))?;
    
    let scan = state.db.create_radar_scan(&market_summary, &recommendations, &raw_rankings)?;
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
) -> Result<IpcResponse<crate::infrastructure::db::types::RadarScan>, AppError> {
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

    let outcome = super::agent::scan(&agent_state.engine, sources).await?;

    let scan = db_state.db.create_radar_scan(
        &outcome.result.market_summary,
        &outcome.result.recommendations,
        &outcome.raw_rankings,
    )?;

    tracing::info!(
        scan_id = %scan.id,
        recommendations = outcome.result.recommendations.len(),
        "Radar scan completed and persisted"
    );

    Ok(IpcResponse::created(scan))
}

/// 列出可用数据源元信息(内置自动抓取源 + 文本注入源)。
#[tauri::command]
pub async fn radar_list_sources() -> Result<IpcResponse<Vec<RadarSourceInfo>>, AppError> {
    Ok(IpcResponse::ok(builtin_source_infos()))
}