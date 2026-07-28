
//! ═══════════════════════════════════════════════════════════════════════════
//! Commands - 技能 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供技能管理的 IPC 命令实现：
//! - skill_list：列出技能
//! - skill_get：获取技能
//! - skill_create：创建技能
//! - skill_update：更新技能
//! - skill_delete：删除技能
//! - skill_index：构建技能索引
//! - skill_refresh：刷新技能发现

use crate::shared::error::{AppError, IpcResponse};
use super::types::{SkillMeta, Skill};
use super::state::SkillState;
use tauri::State;
use serde::Deserialize;
use std::time::Instant;

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
    let start = Instant::now();
    tracing::info!("skill_list: enter");
    
    let manager = state.manager.lock().await;
    let skills: Vec<_> = manager.list().into_iter().map(|s| s.meta.clone()).collect();
    
    tracing::info!(
        count = skills.len(),
        duration_ms = start.elapsed().as_millis(),
        "skill_list: exit"
    );
    Ok(IpcResponse::ok(skills))
}

#[tauri::command]
pub async fn skill_get(
    state: State<'_, SkillState>,
    name: String,
) -> Result<IpcResponse<Skill>, AppError> {
    let start = Instant::now();
    tracing::info!(name = %name, "skill_get: enter");
    
    validate_skill_name(&name)?;
    
    let manager = state.manager.lock().await;
    let skill = manager.load(&name)
        .ok_or_else(|| {
            tracing::error!(name = %name, "skill_get: Skill not found");
            AppError::skill_not_found(name)
        })?;
    
    tracing::info!(
        name = %skill.meta.name,
        duration_ms = start.elapsed().as_millis(),
        "skill_get: exit"
    );
    Ok(IpcResponse::ok(skill.clone()))
}

#[tauri::command]
pub async fn skill_create(
    state: State<'_, SkillState>,
    req: CreateSkillRequest,
) -> Result<IpcResponse<super::types::Skill>, AppError> {
    let start = Instant::now();
    tracing::info!(
        name = %req.name,
        category = %req.category,
        "skill_create: enter"
    );
    
    validate_skill_name(&req.name)?;
    if req.description.len() > 2000 {
        tracing::error!(len = req.description.len(), "skill_create: Skill description too long");
        return Err(AppError::invalid_input("Skill description too long (max 2000 chars)"));
    }
    if req.content.len() > 10_000_000 {
        tracing::error!(len = req.content.len(), "skill_create: Skill content too long");
        return Err(AppError::invalid_input("Skill content too long (max 10MB)"));
    }
    
    let meta = SkillMeta {
        name: req.name,
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
        version: 1,
        tags: Vec::new(),
        depends_on: Vec::new(),
        metadata: None,
        policy: None,
    };
    // TODO(perf): Mutex 跨 fs::write 持有，并发 skill_create/update/delete 会串行化。
    // 当前 skill 写操作频率低，可接受。若未来高频化，需重构 SkillManager：
    // 1) 锁内只读 dirs/skills 等状态；2) 释放锁做 fs I/O；3) 重锁 push 到 skills。
    let mut manager = state.manager.lock().await;
    let skill = manager.create_skill(meta, &req.content).map_err(|e| {
        tracing::error!(error = %e, "skill_create: Failed to create skill");
        e
    })?;
    
    tracing::info!(
        name = %skill.meta.name,
        duration_ms = start.elapsed().as_millis(),
        "skill_create: exit"
    );
    Ok(IpcResponse::created(skill))
}

#[tauri::command]
pub async fn skill_update(
    state: State<'_, SkillState>,
    req: UpdateSkillRequest,
) -> Result<IpcResponse<super::types::Skill>, AppError> {
    let start = Instant::now();
    tracing::info!(name = %req.name, "skill_update: enter");
    
    validate_skill_name(&req.name)?;
    if req.description.len() > 2000 {
        tracing::error!(len = req.description.len(), "skill_update: Skill description too long");
        return Err(AppError::invalid_input("Skill description too long (max 2000 chars)"));
    }
    if req.content.len() > 10_000_000 {
        tracing::error!(len = req.content.len(), "skill_update: Skill content too long");
        return Err(AppError::invalid_input("Skill content too long (max 10MB)"));
    }
    
    let meta = SkillMeta {
        name: req.name.clone(),
        description: req.description,
        category: req.category,
        requires_tools: Vec::new(),
        platforms: None,
        version: 1,
        tags: Vec::new(),
        depends_on: Vec::new(),
        metadata: None,
        policy: None,
    };
    let mut manager = state.manager.lock().await;
    let skill = manager.update_skill(&req.name, meta, &req.content).map_err(|e| {
        tracing::error!(name = %req.name, error = %e, "skill_update: Failed to update skill");
        e
    })?;
    
    tracing::info!(
        name = %req.name,
        duration_ms = start.elapsed().as_millis(),
        "skill_update: exit"
    );
    Ok(IpcResponse::ok(skill))
}

#[tauri::command]
pub async fn skill_delete(
    state: State<'_, SkillState>,
    name: String,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(name = %name, "skill_delete: enter");
    
    validate_skill_name(&name)?;
    
    let mut manager = state.manager.lock().await;
    manager.delete_skill(&name).map_err(|e| {
        tracing::error!(name = %name, error = %e, "skill_delete: Failed to delete skill");
        e
    })?;
    
    tracing::info!(
        name = %name,
        duration_ms = start.elapsed().as_millis(),
        "skill_delete: exit"
    );
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn skill_index(
    state: State<'_, SkillState>,
) -> Result<IpcResponse<String>, AppError> {
    let start = Instant::now();
    tracing::info!("skill_index: enter");
    
    let manager = state.manager.lock().await;
    let index = manager.build_index();
    
    tracing::info!(
        length = index.len(),
        duration_ms = start.elapsed().as_millis(),
        "skill_index: exit"
    );
    Ok(IpcResponse::ok(index))
}

#[tauri::command]
pub async fn skill_refresh(
    state: State<'_, SkillState>,
) -> Result<IpcResponse<usize>, AppError> {
    let start = Instant::now();
    tracing::info!("skill_refresh: enter");
    
    let mut manager = state.manager.lock().await;
    manager.discover().map_err(|e| {
        tracing::error!(error = %e, "skill_refresh: Failed to discover skills");
        e
    })?;
    let count = manager.list().len();
    
    tracing::info!(
        count,
        duration_ms = start.elapsed().as_millis(),
        "skill_refresh: exit"
    );
    Ok(IpcResponse::ok(count))
}