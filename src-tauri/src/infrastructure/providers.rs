//! ═══════════════════════════════════════════════════════════════════════════
//! Provider 管理 - LLM 供应商配置
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::llm::state::LlmState;
use tauri::State;

/// LLM Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
}

/// Provider 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Available,
    Unavailable,
    Error,
}

/// Provider 信息
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub config: ProviderConfig,
    pub status: ProviderStatus,
}

impl ProviderInfo {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            status: ProviderStatus::Available,
        }
    }

    pub fn is_available(&self) -> bool {
        self.status == ProviderStatus::Available
    }
}

#[tauri::command]
pub async fn provider_list(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm::registry::ProviderInfo>>, AppError> {
    tracing::debug!("provider_list");
    let registry = state.registry.lock().await;
    let providers = registry.list_providers();
    tracing::debug!(count = providers.len(), "Providers listed");
    Ok(IpcResponse::ok(providers))
}

#[tauri::command]
pub async fn provider_models(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm::types::ModelInfo>>, AppError> {
    tracing::debug!("provider_models");
    let registry = state.registry.lock().await;
    let models = registry.all_models();
    tracing::debug!(count = models.len(), "Models listed");
    Ok(IpcResponse::ok(models))
}

#[tauri::command]
pub async fn provider_test_connection(
    state: State<'_, LlmState>,
    provider: String,
    api_key: String,
    base_url: String,
    model: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(provider = %provider, model = %model, base_url = %base_url, "provider_test_connection: starting");
    let registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&state.data_dir);
    let result = registry.test_connection(&provider, &api_key, &base_url, &model).await;
    if let Err(ref e) = result {
        tracing::warn!(provider = %provider, model = %model, error = %e, "provider_test_connection: failed");
    }
    result?;
    tracing::info!(provider = %provider, model = %model, "provider_test_connection: passed");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn provider_refresh(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("provider_refresh: starting");
    let new_registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&state.data_dir);
    let provider_count = new_registry.list_providers().len();
    let model_count = new_registry.all_models().len();
    let mut registry = state.registry.lock().await;
    *registry = new_registry;
    tracing::info!(
        providers = provider_count,
        models = model_count,
        "provider_refresh: completed"
    );
    Ok(IpcResponse::ok(()))
}