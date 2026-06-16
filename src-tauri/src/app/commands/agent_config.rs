use crate::errors::{AppError, IpcResponse};
use crate::AppState;
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
