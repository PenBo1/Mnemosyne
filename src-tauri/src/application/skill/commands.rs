
use crate::shared::error::{AppError, IpcResponse};
use super::types::{SkillMeta, Skill};
use super::state::SkillState;
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

fn validate_skill_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Skill name cannot be empty"));
    }
    if name.len() > 255 {
        return Err(AppError::invalid_input("Skill name too long (max 255 chars)"));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(AppError::path_traversal());
    }
    Ok(())
}

#[tauri::command]
pub async fn skill_list(
    state: State<'_, SkillState>,
) -> Result<IpcResponse<Vec<SkillMeta>>, AppError> {
    tracing::debug!("skill_list");
    let manager = state.manager.lock().await;
    let skills: Vec<_> = manager.list().into_iter().map(|s| s.meta.clone()).collect();
    tracing::debug!(count = skills.len(), "Skills listed");
    Ok(IpcResponse::ok(skills))
}

#[tauri::command]
pub async fn skill_get(
    state: State<'_, SkillState>,
    name: String,
) -> Result<IpcResponse<Skill>, AppError> {
    validate_skill_name(&name)?;
    tracing::debug!(name = %name, "skill_get");
    let manager = state.manager.lock().await;
    let skill = manager.load(&name)
        .ok_or_else(|| {
            tracing::warn!(name = %name, "Skill not found");
            AppError::skill_not_found(name)
        })?;
    Ok(IpcResponse::ok(skill.clone()))
}

#[tauri::command]
pub async fn skill_create(
    state: State<'_, SkillState>,
    req: CreateSkillRequest,
) -> Result<IpcResponse<super::types::Skill>, AppError> {
    validate_skill_name(&req.name)?;
    if req.description.len() > 2000 {
        return Err(AppError::invalid_input("Skill description too long (max 2000 chars)"));
    }
    if req.content.len() > 10_000_000 {
        return Err(AppError::invalid_input("Skill content too long (max 10MB)"));
    }
    tracing::info!(name = %req.name, category = %req.category, "skill_create");
    let meta = SkillMeta {
        name: req.name,
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
        version: 1,
        tags: Vec::new(),
        depends_on: Vec::new(),
    };
    // TODO(perf): Mutex 跨 fs::write 持有，并发 skill_create/update/delete 会串行化。
    // 当前 skill 写操作频率低，可接受。若未来高频化，需重构 SkillManager：
    // 1) 锁内只读 dirs/skills 等状态；2) 释放锁做 fs I/O；3) 重锁 push 到 skills。
    let mut manager = state.manager.lock().await;
    let skill = manager.create_skill(meta, &req.content)?;
    tracing::info!(name = %skill.meta.name, "Skill created");
    Ok(IpcResponse::created(skill))
}

#[tauri::command]
pub async fn skill_update(
    state: State<'_, SkillState>,
    req: UpdateSkillRequest,
) -> Result<IpcResponse<super::types::Skill>, AppError> {
    validate_skill_name(&req.name)?;
    if req.description.len() > 2000 {
        return Err(AppError::invalid_input("Skill description too long (max 2000 chars)"));
    }
    if req.content.len() > 10_000_000 {
        return Err(AppError::invalid_input("Skill content too long (max 10MB)"));
    }
    tracing::info!(name = %req.name, "skill_update");
    let meta = SkillMeta {
        name: req.name.clone(),
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
        version: 1,
        tags: Vec::new(),
        depends_on: Vec::new(),
    };
    let mut manager = state.manager.lock().await;
    let skill = manager.update_skill(&req.name, meta, &req.content)?;
    tracing::info!(name = %req.name, "Skill updated");
    Ok(IpcResponse::ok(skill))
}

#[tauri::command]
pub async fn skill_delete(
    state: State<'_, SkillState>,
    name: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_skill_name(&name)?;
    tracing::info!(name = %name, "skill_delete");
    let mut manager = state.manager.lock().await;
    manager.delete_skill(&name)?;
    tracing::info!(name = %name, "Skill deleted");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn skill_index(
    state: State<'_, SkillState>,
) -> Result<IpcResponse<String>, AppError> {
    tracing::debug!("skill_index");
    let manager = state.manager.lock().await;
    let index = manager.build_index();
    tracing::debug!(length = index.len(), "Skill index built");
    Ok(IpcResponse::ok(index))
}

#[tauri::command]
pub async fn skill_refresh(
    state: State<'_, SkillState>,
) -> Result<IpcResponse<usize>, AppError> {
    tracing::info!("skill_refresh");
    let mut manager = state.manager.lock().await;
    manager.discover()?;
    let count = manager.list().len();
    tracing::info!(count, "Skills refreshed");
    Ok(IpcResponse::ok(count))
}