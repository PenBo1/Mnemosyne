//! ═══════════════════════════════════════════════════════════════════════════
//! LLM 命令 - Tauri IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::llm::state::LlmState;
use crate::infrastructure::llm::presets::{PRESETS, PresetProtocol};
use crate::core::agent::PromptOptimizer;
use serde::{Deserialize, Serialize};
use tauri::State;
use std::time::Instant;

// ── 响应类型 ────────────────────────────────────────────────────────────────

/// 提示词优化响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptOptimizeResponse {
    pub optimized_prompt: String,
}

/// 提供商预设信息
#[derive(Debug, Serialize, Clone)]
pub struct ProviderPresetInfo {
    /// 预设 ID
    pub id: String,
    /// 显示名称
    pub label: String,
    /// 分组
    pub group: String,
    /// 协议类型
    pub protocol: &'static str,
    /// 基础 URL
    pub base_url: String,
    /// 环境变量名
    pub env_var: String,
    /// 基础 URL 环境变量名
    pub env_base_url: String,
    /// 检查模型
    pub check_model: String,
    /// 可用模型列表
    pub models: Vec<crate::infrastructure::llm::types::ModelInfo>,
}

// ── IPC 命令 ─────────────────────────────────────────────────────────────────

/// 优化提示词
///
/// 使用当前激活的模型对用户提示词进行优化。
/// 业务逻辑委托给 core::agent::PromptOptimizer。
#[tauri::command]
pub async fn prompt_optimize(
    state: State<'_, LlmState>,
    prompt: String,
) -> Result<IpcResponse<PromptOptimizeResponse>, AppError> {
    // 参数校验
    if prompt.trim().is_empty() {
        return Err(AppError::invalid_input("Prompt cannot be empty"));
    }
    if prompt.len() > 100_000 {
        return Err(AppError::invalid_input("Prompt too long (max 100KB)"));
    }

    // 委托给核心层
    let registry = state.registry.lock().await;
    let optimized = PromptOptimizer::optimize(&registry, &prompt).await?;

    Ok(IpcResponse::ok(PromptOptimizeResponse {
        optimized_prompt: optimized,
    }))
}

/// 列出已配置的模型
#[tauri::command]
pub async fn llm_model_list(
    state: State<'_, LlmState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm::registry::AiModelConfig>>, AppError> {
    let start = Instant::now();
    tracing::info!("llm_model_list: enter");

    let registry = state.registry.lock().await;
    let models = registry.model_configs().to_vec();

    tracing::info!(
        count = models.len(),
        duration_ms = start.elapsed().as_millis(),
        "llm_model_list: exit"
    );
    Ok(IpcResponse::ok(models))
}

/// 列出所有提供商预设
#[tauri::command]
pub async fn llm_list_provider_presets() -> Result<IpcResponse<Vec<ProviderPresetInfo>>, AppError> {
    let start = Instant::now();
    tracing::info!("llm_list_provider_presets: enter");

    let presets: Vec<_> = PRESETS.iter().map(|p| ProviderPresetInfo {
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

    tracing::info!(
        count = presets.len(),
        duration_ms = start.elapsed().as_millis(),
        "llm_list_provider_presets: exit"
    );
    Ok(IpcResponse::ok(presets))
}