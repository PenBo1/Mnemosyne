use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use crate::infra::skill::SkillMeta;
use tauri::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub content: String,
}

fn default_category() -> String {
    "general".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateSkillRequest {
    pub name: String,
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    pub content: String,
}

#[tauri::command]
pub async fn skill_list(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<crate::infra::skill::SkillMeta>>, AppError> {
    tracing::debug!("skill_list");
    let manager = state.skill_manager.lock().await;
    let skills: Vec<_> = manager.list().into_iter().map(|s| s.meta.clone()).collect();
    tracing::debug!(count = skills.len(), "Skills listed");
    Ok(IpcResponse::ok(skills))
}

#[tauri::command]
pub async fn skill_get(
    state: State<'_, AppState>,
    name: String,
) -> Result<IpcResponse<crate::infra::skill::Skill>, AppError> {
    tracing::debug!(name = %name, "skill_get");
    let manager = state.skill_manager.lock().await;
    let skill = manager.load(&name)
        .ok_or_else(|| {
            tracing::warn!(name = %name, "Skill not found");
            AppError::skill_not_found(name)
        })?;
    Ok(IpcResponse::ok(skill.clone()))
}

#[tauri::command]
pub async fn skill_create(
    state: State<'_, AppState>,
    req: CreateSkillRequest,
) -> Result<IpcResponse<crate::infra::skill::Skill>, AppError> {
    tracing::info!(name = %req.name, category = %req.category, "skill_create");
    let meta = SkillMeta {
        name: req.name,
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
    };
    let mut manager = state.skill_manager.lock().await;
    let skill = manager.create_skill(meta, &req.content)?;
    tracing::info!(name = %skill.meta.name, "Skill created");
    Ok(IpcResponse::created(skill))
}

#[tauri::command]
pub async fn skill_update(
    state: State<'_, AppState>,
    req: UpdateSkillRequest,
) -> Result<IpcResponse<crate::infra::skill::Skill>, AppError> {
    tracing::info!(name = %req.name, "skill_update");
    let meta = SkillMeta {
        name: req.name.clone(),
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
    };
    let mut manager = state.skill_manager.lock().await;
    let skill = manager.update_skill(&req.name, meta, &req.content)?;
    tracing::info!(name = %req.name, "Skill updated");
    Ok(IpcResponse::ok(skill))
}

#[tauri::command]
pub async fn skill_delete(
    state: State<'_, AppState>,
    name: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(name = %name, "skill_delete");
    let mut manager = state.skill_manager.lock().await;
    manager.delete_skill(&name)?;
    tracing::info!(name = %name, "Skill deleted");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn skill_index(
    state: State<'_, AppState>,
) -> Result<IpcResponse<String>, AppError> {
    tracing::debug!("skill_index");
    let manager = state.skill_manager.lock().await;
    let index = manager.build_index();
    tracing::debug!(length = index.len(), "Skill index built");
    Ok(IpcResponse::ok(index))
}

#[tauri::command]
pub async fn skill_refresh(
    state: State<'_, AppState>,
) -> Result<IpcResponse<usize>, AppError> {
    tracing::info!("skill_refresh");
    let mut manager = state.skill_manager.lock().await;
    manager.discover()?;
    let count = manager.list().len();
    tracing::info!(count, "Skills refreshed");
    Ok(IpcResponse::ok(count))
}
