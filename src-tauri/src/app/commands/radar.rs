use tauri::State;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::domain::radar::agent::RadarAgent;
use crate::domain::radar::source::default_sources;

#[tauri::command]
pub async fn radar_scan(
    state: State<'_, AppState>,
) -> Result<IpcResponse<crate::infra::db::models::RadarScan>, AppError> {
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

    let db = state.db.lock().await;
    let scan = db.create_radar_scan(
        &result.market_summary,
        &result.recommendations,
        &raw_rankings,
    )?;
    drop(db);

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
) -> Result<IpcResponse<Vec<crate::infra::db::models::RadarScan>>, AppError> {
    let db = state.db.lock().await;
    let scans = db.list_radar_scans(limit)?;
    Ok(IpcResponse::ok(scans))
}

#[tauri::command]
pub async fn radar_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let db = state.db.lock().await;
    let deleted = db.delete_radar_scan(&id)?;
    Ok(IpcResponse::ok(deleted))
}
