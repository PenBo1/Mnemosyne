// Agent Registry IPC 命令:列出所有 agent / 按类别列出 / 查询单个 / 列出类别。
//
// 供前端 Agent 管理面板、调度器配置、权限设置页面调用。
// 与 core::agent::commands（chat 命令）正交：本模块只读元数据，不执行 agent。

use std::sync::Arc;

use tauri::State;

use crate::shared::error::{AppError, IpcResponse};

use super::registry::AgentRegistry;
use super::types::{AgentCategory, AgentDescriptor};

/// AgentRegistry 的 Tauri State 包装。
///
/// 使用 Arc 而非直接管理 AgentRegistry，便于未来在多个 State 间共享。
pub struct AgentRegistryState(pub Arc<AgentRegistry>);

impl AgentRegistryState {
    pub fn new() -> Self {
        Self(Arc::new(AgentRegistry::new()))
    }
}

impl Default for AgentRegistryState {
    fn default() -> Self {
        Self::new()
    }
}

/// 列出所有 agent（内置 + 自定义）。
#[tauri::command]
pub async fn agent_list_all(
    state: State<'_, AgentRegistryState>,
) -> Result<IpcResponse<Vec<AgentDescriptor>>, AppError> {
    let agents = state.0.list_all();
    Ok(IpcResponse::ok(agents))
}

/// 按类别列出 agent。
/// category ∈ {"main","pipeline","subagent","loopskill"}。
#[tauri::command]
pub async fn agent_list_by_category(
    category: String,
    state: State<'_, AgentRegistryState>,
) -> Result<IpcResponse<Vec<AgentDescriptor>>, AppError> {
    let cat = AgentCategory::from_str(&category)
        .map_err(|e| AppError::bad_request(e))?;
    let agents = state.0.list_by_category(cat);
    Ok(IpcResponse::ok(agents))
}

/// 按 id 查询单个 agent。
#[tauri::command]
pub async fn agent_get(
    id: String,
    state: State<'_, AgentRegistryState>,
) -> Result<IpcResponse<AgentDescriptor>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("id cannot be empty"));
    }
    let agent = state
        .0
        .get(&id)
        .ok_or_else(|| AppError::not_found(format!("Agent '{}' not found", id)))?;
    Ok(IpcResponse::ok(agent))
}

/// 列出所有 agent 类别。
#[tauri::command]
pub async fn agent_list_categories(
    state: State<'_, AgentRegistryState>,
) -> Result<IpcResponse<Vec<CategoryDto>>, AppError> {
    let categories = state
        .0
        .list_categories()
        .into_iter()
        .map(CategoryDto::from)
        .collect();
    Ok(IpcResponse::ok(categories))
}

/// 类别 DTO（含 id 与显示名，便于前端渲染）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
}

impl From<AgentCategory> for CategoryDto {
    fn from(c: AgentCategory) -> Self {
        let name = match c {
            AgentCategory::Main => "Main",
            AgentCategory::Pipeline => "Pipeline",
            AgentCategory::SubAgent => "SubAgent",
            AgentCategory::LoopSkill => "LoopSkill",
        }
        .to_string();
        Self {
            id: c.as_str().to_string(),
            name,
        }
    }
}
