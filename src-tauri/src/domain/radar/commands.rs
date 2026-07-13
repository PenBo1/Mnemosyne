use crate::core::agent::commands::AgentState;
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
/// 不需要前端传参,后端自行抓取数据 + 调用 LLM 分析 + 持久化。
#[tauri::command]
pub async fn radar_scan(
    agent_state: State<'_, AgentState>,
    db_state: State<'_, DbState>,
) -> Result<IpcResponse<crate::infrastructure::db::types::RadarScan>, AppError> {
    let outcome = super::agent::scan(&agent_state.engine, None).await?;

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