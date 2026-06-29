use tauri::State;
use crate::shared::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::features::radar::agent::RadarAgent;
use crate::features::radar::source::default_sources;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;

#[tauri::command]
pub async fn radar_scan(
    state: State<'_, AppState>,
) -> Result<IpcResponse<crate::infrastructure::db::models::RadarScan>, AppError> {
    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider_arc = registry.default()
            .map_err(|e| AppError::internal(format!("No LLM provider configured: {}", e)))?;
        let model = registry.default_model().to_string();
        (provider_arc, model)
    };

    let sources = default_sources();
    let agent = RadarAgent::new(provider, model, sources);
    let (result, raw_rankings) = agent.scan().await?;

    let scan = state.db.create_radar_scan(
        &result.market_summary,
        &result.recommendations,
        &raw_rankings,
    ).await?;

    tracing::info!(
        recommendations = result.recommendations.len(),
        "Radar scan completed"
    );

    Ok(IpcResponse::ok(scan))
}

#[tauri::command]
pub async fn radar_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::models::RadarScan>>, AppError> {
    let scans = state.db.list_radar_scans(limit).await?;
    Ok(IpcResponse::ok(scans))
}

#[tauri::command]
pub async fn radar_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id_component(&id, "radar_scan_id")?;
    let deleted = state.db.delete_radar_scan(&id).await?;
    Ok(IpcResponse::ok(deleted))
}
