use crate::shared::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Agent 配置（IPC 层）。
///
/// `model` 字段的语义在 S9 后变更为 **model_id**（即 `AiModelConfig.id`），
/// 而非直接的模型名（如 "gpt-4o"）。前端通过 `list_ai_models` 获取可用模型列表，
/// 选中后将 `id` 传回 `update_agent`。`list_agents` 返回的 `model` 是当前生效的
/// model_id：优先取 `agent_model_overrides` 中的覆盖值，否则取 `active_model_id`，
/// 最后回退到 "gpt-4o"（仅作占位，无实际 provider 绑定）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub status: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub enabled: Option<bool>,
}

/// 默认 agent 元数据（不含 model 覆盖 —— model 由 registry 决定）。
///
/// 这里只定义 agent 的 id/name/description/system_prompt/temperature/max_tokens 等
/// 静态属性。`model` 字段在 `list_agents` 时由 registry 的 `agent_model_overrides`
/// 或 `active_model_id` 动态填充。
fn default_agent_metadata() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            id: "architect".into(),
            name: "Architect".into(),
            description: "Creates story framework, world settings, characters".into(),
            model: String::new(), // 由 list_agents 动态填充
            system_prompt: "You are a story architecture specialist.".into(),
            temperature: 0.7,
            max_tokens: 4096,
            status: "active".into(),
            enabled: true,
        },
        AgentConfig {
            id: "planner".into(),
            name: "Planner".into(),
            description: "Plans chapter content and produces chapter memo".into(),
            model: String::new(),
            system_prompt: "You are this novel's editor-in-chief.".into(),
            temperature: 0.7,
            max_tokens: 4096,
            status: "active".into(),
            enabled: true,
        },
        AgentConfig {
            id: "composer".into(),
            name: "Composer".into(),
            description: "Assembles context package for the writer".into(),
            model: String::new(),
            system_prompt: "You are a context assembly specialist.".into(),
            temperature: 0.7,
            max_tokens: 4096,
            status: "active".into(),
            enabled: true,
        },
        AgentConfig {
            id: "writer".into(),
            name: "Writer".into(),
            description: "Writes chapter prose following constraints".into(),
            model: String::new(),
            system_prompt: "You are a skilled novelist.".into(),
            temperature: 0.7,
            max_tokens: 8192,
            status: "active".into(),
            enabled: true,
        },
        AgentConfig {
            id: "auditor".into(),
            name: "Auditor".into(),
            description: "Checks chapter quality across 29 dimensions".into(),
            model: String::new(),
            system_prompt: "You are a quality assurance expert.".into(),
            temperature: 0.3,
            max_tokens: 4096,
            status: "active".into(),
            enabled: true,
        },
        AgentConfig {
            id: "reviser".into(),
            name: "Reviser".into(),
            description: "Revises chapter based on audit feedback".into(),
            model: String::new(),
            system_prompt: "You are a revision specialist.".into(),
            temperature: 0.7,
            max_tokens: 8192,
            status: "active".into(),
            enabled: true,
        },
    ]
}

/// 为某个 agent 解析当前生效的 model_id。
///
/// 优先级：`agent_model_overrides[agent_id]` > `active_model_id` > "gpt-4o"（占位）。
fn resolve_effective_model_id(
    registry: &crate::infrastructure::llm_client::ProviderRegistry,
    agent_id: &str,
) -> String {
    if let Some(model_id) = registry.agent_model_overrides().get(agent_id) {
        return model_id.clone();
    }
    registry.active_model_id().map(|s| s.to_string()).unwrap_or_else(|| "gpt-4o".to_string())
}

#[tauri::command]
pub async fn list_agents(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<AgentConfig>>, AppError> {
    tracing::debug!("list_agents");
    let registry = state.provider_registry.lock().await;
    let mut agents = default_agent_metadata();
    for agent in &mut agents {
        agent.model = resolve_effective_model_id(&registry, &agent.id);
    }
    tracing::debug!(count = agents.len(), "Agents listed");
    Ok(IpcResponse::ok(agents))
}

#[tauri::command]
pub async fn update_agent(
    state: State<'_, AppState>,
    req: UpdateAgentRequest,
) -> Result<IpcResponse<AgentConfig>, AppError> {
    validate_id_component(&req.id, "agent_id")?;
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(AppError::invalid_input("Agent name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(AppError::invalid_input("Agent name too long (max 255 chars)"));
        }
    }
    if let Some(temp) = req.temperature {
        if !temp.is_finite() || temp < 0.0 || temp > 2.0 {
            return Err(AppError::invalid_input("Temperature must be between 0.0 and 2.0"));
        }
    }
    if let Some(max_tokens) = req.max_tokens {
        if max_tokens == 0 || max_tokens > 1_000_000 {
            return Err(AppError::invalid_input("max_tokens must be between 1 and 1000000"));
        }
    }

    tracing::info!(agent_id = %req.id, "update_agent");

    // S9: 若传了 model，则持久化为 per-agent 覆盖
    if let Some(ref model_id) = req.model {
        let mut registry = state.provider_registry.lock().await;
        registry.set_agent_model_override(&req.id, Some(model_id.clone()))?;
    }

    // 返回更新后的 agent 配置（重新读取 registry 以反映最新状态）
    let registry = state.provider_registry.lock().await;
    let mut agents = default_agent_metadata();
    let agent = agents.iter_mut().find(|a| a.id == req.id)
        .ok_or_else(|| AppError::not_found(format!("Agent '{}' not found", req.id)))?;

    // 应用非 model 字段的内存覆盖（name/description/system_prompt 等暂不持久化）
    if let Some(name) = req.name { agent.name = name; }
    if let Some(desc) = req.description { agent.description = desc; }
    if let Some(prompt) = req.system_prompt { agent.system_prompt = prompt; }
    if let Some(temp) = req.temperature { agent.temperature = temp; }
    if let Some(max_tokens) = req.max_tokens { agent.max_tokens = max_tokens; }
    if let Some(enabled) = req.enabled {
        agent.enabled = enabled;
        agent.status = if enabled { "active".into() } else { "inactive".into() };
    }
    // model 字段始终从 registry 解析（确保返回最新生效值）
    agent.model = resolve_effective_model_id(&registry, &agent.id);

    let result = agent.clone();
    tracing::info!(agent_id = %result.id, model_id = %result.model, "Agent updated");
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn toggle_agent_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<AgentConfig>, AppError> {
    validate_id_component(&id, "agent_id")?;
    tracing::info!(agent_id = %id, "toggle_agent_status");

    let registry = state.provider_registry.lock().await;
    let mut agents = default_agent_metadata();
    let agent = agents.iter_mut().find(|a| a.id == id)
        .ok_or_else(|| AppError::not_found(format!("Agent '{}' not found", id)))?;

    agent.enabled = !agent.enabled;
    agent.status = if agent.enabled { "active".into() } else { "inactive".into() };
    agent.model = resolve_effective_model_id(&registry, &agent.id);

    let result = agent.clone();
    tracing::info!(agent_id = %result.id, status = %result.status, "Agent status toggled");
    Ok(IpcResponse::ok(result))
}

/// 列出用户配置的所有 AI 模型（供前端 agent 配置下拉选择）。
///
/// 返回 `AiModelConfig` 列表（含 id/name/provider/model）。
/// 前端选中某个模型后，将 `id` 作为 `update_agent` 的 `model` 字段传回。
#[tauri::command]
pub async fn list_ai_models(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::llm_client::AiModelConfig>>, AppError> {
    tracing::debug!("list_ai_models");
    let registry = state.provider_registry.lock().await;
    let models = registry.model_configs().to_vec();
    tracing::debug!(count = models.len(), "AI models listed");
    Ok(IpcResponse::ok(models))
}
