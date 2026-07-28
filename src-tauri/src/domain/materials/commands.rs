//! ═══════════════════════════════════════════════════════════════════════════
//! 材料命令 - 材料系统 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;
use std::time::Instant;

use super::ingest::ingest_material;
use super::retrieve::{delete_material, list_material_assets, retrieve_materials};
use super::types::{IngestMaterialInput, MaterialAsset, RetrieveMaterialsInput, RetrievedMaterial};

#[tauri::command]
pub async fn material_ingest(
    data_dir: State<'_, DataDir>,
    input: IngestMaterialInput,
) -> Result<IpcResponse<MaterialAsset>, AppError> {
    let start = Instant::now();
    tracing::info!(
        source_kind = %input.source_kind,
        "material_ingest: enter"
    );
    
    if input.source_kind.trim().is_empty() {
        tracing::error!("material_ingest: sourceKind cannot be empty");
        return Err(AppError::missing_field("sourceKind"));
    }
    
    let asset = ingest_material(&data_dir, &input).await.map_err(|e| {
        tracing::error!(error = %e, "material_ingest: Failed to ingest material");
        e
    })?;
    
    tracing::info!(
        material_id = %asset.id,
        duration_ms = start.elapsed().as_millis(),
        "material_ingest: exit"
    );
    Ok(IpcResponse::created(asset))
}

#[tauri::command]
pub async fn material_retrieve(
    data_dir: State<'_, DataDir>,
    input: RetrieveMaterialsInput,
) -> Result<IpcResponse<Vec<RetrievedMaterial>>, AppError> {
    let start = Instant::now();
    tracing::info!(
        query_len = input.query.len(),
        limit = input.limit,
        "material_retrieve: enter"
    );
    
    let data_dir = data_dir.inner().clone();
    let results = tokio::task::spawn_blocking(move || retrieve_materials(&data_dir, &input))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "material_retrieve: spawn_blocking join failed");
            AppError::internal(format!("spawn_blocking join failed: {}", e))
        })?
        .map_err(|e| {
            tracing::error!(error = %e, "material_retrieve: Failed to retrieve materials");
            e
        })?;
    
    tracing::info!(
        result_count = results.len(),
        duration_ms = start.elapsed().as_millis(),
        "material_retrieve: exit"
    );
    Ok(IpcResponse::ok(results))
}

#[tauri::command]
pub async fn material_list(
    data_dir: State<'_, DataDir>,
) -> Result<IpcResponse<Vec<MaterialAsset>>, AppError> {
    let start = Instant::now();
    tracing::info!("material_list: enter");
    
    let data_dir = data_dir.inner().clone();
    let assets = tokio::task::spawn_blocking(move || list_material_assets(&data_dir))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "material_list: spawn_blocking join failed");
            AppError::internal(format!("spawn_blocking join failed: {}", e))
        })?
        .map_err(|e| {
            tracing::error!(error = %e, "material_list: Failed to list materials");
            e
        })?;
    
    tracing::info!(
        count = assets.len(),
        duration_ms = start.elapsed().as_millis(),
        "material_list: exit"
    );
    Ok(IpcResponse::ok(assets))
}

#[tauri::command]
pub async fn material_delete(
    data_dir: State<'_, DataDir>,
    material_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(material_id = %material_id, "material_delete: enter");
    
    validate_id(&material_id, "material_id").map_err(AppError::invalid_input)?;
    
    let data_dir = data_dir.inner().clone();
    let material_id_for_log = material_id.clone();
    let removed = tokio::task::spawn_blocking(move || delete_material(&data_dir, &material_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "material_delete: spawn_blocking join failed");
            AppError::internal(format!("spawn_blocking join failed: {}", e))
        })?
        .map_err(|e| {
            tracing::error!(material_id = %material_id_for_log, error = %e, "material_delete: Failed to delete material");
            e
        })?;
    
    tracing::info!(
        material_id = %material_id_for_log,
        removed,
        duration_ms = start.elapsed().as_millis(),
        "material_delete: exit"
    );
    Ok(IpcResponse::deleted(removed))
}