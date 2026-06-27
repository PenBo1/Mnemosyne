use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::infra::data_dir::AGENT_ROLES;
use crate::infra::fs_utils::validate_id_component;
use serde::{Deserialize, Serialize};
use tauri::State;

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
pub struct AgentIdentityData {
    pub role: String,
    pub soul: String,
    pub context: String,
    pub memory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentIdentityRequest {
    pub role: String,
    pub soul: Option<String>,
    pub context: Option<String>,
    pub memory: Option<String>,
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

/// Default agent configurations (embedded in binary)
fn default_agents() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            id: "architect".into(),
            name: "Architect".into(),
            description: "Creates story framework, world settings, characters".into(),
            model: "gpt-4o".into(),
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
            model: "gpt-4o".into(),
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
            model: "gpt-4o".into(),
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
            model: "gpt-4o".into(),
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
            model: "gpt-4o".into(),
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
            model: "gpt-4o".into(),
            system_prompt: "You are a revision specialist.".into(),
            temperature: 0.7,
            max_tokens: 8192,
            status: "active".into(),
            enabled: true,
        },
    ]
}

#[tauri::command]
pub async fn list_agents(
    _state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<AgentConfig>>, AppError> {
    tracing::debug!("list_agents");
    let agents = default_agents();
    tracing::debug!(count = agents.len(), "Agents listed");
    Ok(IpcResponse::ok(agents))
}

#[tauri::command]
pub async fn update_agent(
    _state: State<'_, AppState>,
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

    let mut agents = default_agents();
    let agent = agents.iter_mut().find(|a| a.id == req.id)
        .ok_or_else(|| AppError::not_found(format!("Agent '{}' not found", req.id)))?;

    if let Some(name) = req.name { agent.name = name; }
    if let Some(desc) = req.description { agent.description = desc; }
    if let Some(model) = req.model { agent.model = model; }
    if let Some(prompt) = req.system_prompt { agent.system_prompt = prompt; }
    if let Some(temp) = req.temperature { agent.temperature = temp; }
    if let Some(max_tokens) = req.max_tokens { agent.max_tokens = max_tokens; }
    if let Some(enabled) = req.enabled { agent.enabled = enabled; }

    let result = agent.clone();
    tracing::info!(agent_id = %result.id, "Agent updated");
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn toggle_agent_status(
    _state: State<'_, AppState>,
    id: String,
) -> Result<IpcResponse<AgentConfig>, AppError> {
    validate_id_component(&id, "agent_id")?;
    tracing::info!(agent_id = %id, "toggle_agent_status");

    let mut agents = default_agents();
    let agent = agents.iter_mut().find(|a| a.id == id)
        .ok_or_else(|| AppError::not_found(format!("Agent '{}' not found", id)))?;

    agent.enabled = !agent.enabled;
    agent.status = if agent.enabled { "active".into() } else { "inactive".into() };

    let result = agent.clone();
    tracing::info!(agent_id = %result.id, status = %result.status, "Agent status toggled");
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn get_agent_identity(
    state: State<'_, AppState>,
    role: String,
) -> Result<IpcResponse<AgentIdentityData>, AppError> {
    validate_id_component(&role, "agent_role")?;
    if !AGENT_ROLES.contains(&role.as_str()) {
        return Err(AppError::not_found(format!("Agent role '{}' not found", role)));
    }

    let identity = crate::domain::agents::agent_identity::AgentIdentity::load(&state.data_dir, &role);
    Ok(IpcResponse::ok(AgentIdentityData {
        role,
        soul: identity.soul,
        context: identity.context,
        memory: identity.memory,
    }))
}

#[tauri::command]
pub async fn update_agent_identity(
    state: State<'_, AppState>,
    req: UpdateAgentIdentityRequest,
) -> Result<IpcResponse<AgentIdentityData>, AppError> {
    validate_id_component(&req.role, "agent_role")?;
    if !AGENT_ROLES.contains(&req.role.as_str()) {
        return Err(AppError::not_found(format!("Agent role '{}' not found", req.role)));
    }

    let role = &req.role;
    if let Some(soul) = &req.soul {
        std::fs::write(state.data_dir.agent_soul_path(role), soul)
            .map_err(|e| AppError::internal(format!("Failed to write SOUL.md: {}", e)))?;
    }
    if let Some(context) = &req.context {
        std::fs::write(state.data_dir.agent_context_path(role), context)
            .map_err(|e| AppError::internal(format!("Failed to write CONTEXT.md: {}", e)))?;
    }
    if let Some(memory) = &req.memory {
        std::fs::write(state.data_dir.agent_memory_path(role), memory)
            .map_err(|e| AppError::internal(format!("Failed to write MEMORY.md: {}", e)))?;
    }

    let identity = crate::domain::agents::agent_identity::AgentIdentity::load(&state.data_dir, role);
    Ok(IpcResponse::ok(AgentIdentityData {
        role: role.clone(),
        soul: identity.soul,
        context: identity.context,
        memory: identity.memory,
    }))
}
