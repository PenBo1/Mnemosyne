use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn provider_list(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::llm::ProviderInfo>>, AppError> {
    tracing::debug!("provider_list");
    let registry = state.provider_registry.lock().await;
    let providers = registry.list_providers();
    tracing::debug!(count = providers.len(), "Providers listed");
    Ok(IpcResponse::ok(providers))
}

#[tauri::command]
pub async fn provider_models(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::llm::ModelInfo>>, AppError> {
    tracing::debug!("provider_models");
    let registry = state.provider_registry.lock().await;
    let models = registry.all_models();
    tracing::debug!(count = models.len(), "Models listed");
    Ok(IpcResponse::ok(models))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn provider_test_connection(
    state: State<'_, AppState>,
    provider: String,
    apiKey: String,
    baseUrl: String,
    model: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(provider = %provider, model = %model, "provider_test_connection");
    let registry = state.provider_registry.lock().await;
    registry.test_connection(&provider, &apiKey, &baseUrl, &model).await?;
    tracing::info!(provider = %provider, model = %model, "Connection test passed");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn provider_refresh(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("provider_refresh");
    let new_registry = crate::infra::llm::ProviderRegistry::new(&state.data_dir);
    let mut registry = state.provider_registry.lock().await;
    *registry = new_registry;
    tracing::info!("Provider registry refreshed");
    Ok(IpcResponse::ok(()))
}
