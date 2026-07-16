
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
    tracing::info!(provider = %provider, model = %model, "provider_test_connection");
    // test_connection 不依赖 registry 状态（内部构造临时 provider 句柄），
    // 因此无需持有 registry 锁。直接从 state.data_dir 重建 registry 仅供查询。
    // 这样避免了跨 await 持有 Mutex（test_connection 会发起网络请求，可能长阻塞）。
    let registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&state.data_dir);
    registry.test_connection(&provider, &api_key, &base_url, &model).await?;
    tracing::info!(provider = %provider, model = %model, "Connection test passed");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn provider_refresh(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("provider_refresh");
    let new_registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&state.data_dir);
    let mut registry = state.registry.lock().await;
    *registry = new_registry;
    tracing::info!("Provider registry refreshed");
    Ok(IpcResponse::ok(()))
}