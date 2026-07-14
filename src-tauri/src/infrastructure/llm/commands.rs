use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::llm::state::LlmState;
use crate::infrastructure::llm::presets::{PRESETS, PresetProtocol};
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub async fn llm_model_list(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm::registry::AiModelConfig>>, AppError> {
    let registry = state.registry.lock().await;
    let models = registry.model_configs().to_vec();
    Ok(IpcResponse::ok(models))
}

/// 预设 provider 信息(给前端 UI 用)
#[derive(Debug, Serialize, Clone)]
pub struct ProviderPresetInfo {
    pub id: String,
    pub label: String,
    pub group: String,
    pub protocol: &'static str,
    pub base_url: String,
    pub env_var: String,
    pub env_base_url: String,
    pub check_model: String,
    pub models: Vec<crate::infrastructure::llm::types::ModelInfo>,
}

/// 列出所有可用的 provider 预设(供前端"添加模型"下拉选择)
#[tauri::command]
pub async fn llm_list_provider_presets() -> Result<IpcResponse<Vec<ProviderPresetInfo>>, AppError> {
    let presets = PRESETS.iter().map(|p| ProviderPresetInfo {
        id: p.id.to_string(),
        label: p.label.to_string(),
        group: p.group.to_string(),
        protocol: match p.protocol {
            PresetProtocol::OpenAi => "openai",
            PresetProtocol::Anthropic => "anthropic",
        },
        base_url: p.base_url.to_string(),
        env_var: p.env_var.to_string(),
        env_base_url: p.env_base_url.to_string(),
        check_model: p.check_model.to_string(),
        models: p.to_model_infos(),
    }).collect();
    Ok(IpcResponse::ok(presets))
}