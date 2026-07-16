// 材料 IPC 命令(前端 camelCase 调用):
// - material_ingest: 导入材料(URL/文件)
// - material_retrieve: 关键词检索
// - material_list: 列出所有材料清单
// - material_delete: 删除材料

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::{AppError, IpcResponse};
use tauri::State;

use super::ingest::ingest_material;
use super::retrieve::{delete_material, list_material_assets, retrieve_materials};
use super::types::{IngestMaterialInput, MaterialAsset, RetrieveMaterialsInput, RetrievedMaterial};

#[tauri::command]
pub async fn material_ingest(
    data_dir: State<'_, DataDir>,
    input: IngestMaterialInput,
) -> Result<IpcResponse<MaterialAsset>, AppError> {
    if input.source_kind.trim().is_empty() {
        return Err(AppError::missing_field("sourceKind"));
    }
    let asset = ingest_material(&data_dir, &input).await?;
    Ok(IpcResponse::created(asset))
}

#[tauri::command]
pub async fn material_retrieve(
    data_dir: State<'_, DataDir>,
    input: RetrieveMaterialsInput,
) -> Result<IpcResponse<Vec<RetrievedMaterial>>, AppError> {
    let data_dir = data_dir.inner().clone();
    let results = tokio::task::spawn_blocking(move || retrieve_materials(&data_dir, &input))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(results))
}

#[tauri::command]
pub async fn material_list(
    data_dir: State<'_, DataDir>,
) -> Result<IpcResponse<Vec<MaterialAsset>>, AppError> {
    let data_dir = data_dir.inner().clone();
    let assets = tokio::task::spawn_blocking(move || list_material_assets(&data_dir))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::ok(assets))
}

#[tauri::command]
pub async fn material_delete(
    data_dir: State<'_, DataDir>,
    material_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_id(&material_id, "material_id").map_err(AppError::invalid_input)?;
    let data_dir = data_dir.inner().clone();
    let removed = tokio::task::spawn_blocking(move || delete_material(&data_dir, &material_id))
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;
    Ok(IpcResponse::deleted(removed))
}
