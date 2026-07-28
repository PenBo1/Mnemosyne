//! ═══════════════════════════════════════════════════════════════════════════
//! Capability Commands - 能力技能 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 暴露给前端的命令：
//! - capability_skill_list：列出所有内置能力技能
//! - capability_skill_get：按 id 获取单个技能
//! - capability_skill_resolve：解析技能（三阶段）
//! - prompt_pack_list：列出所有内置提示包
//! - prompt_pack_get：按 promptId 加载单个提示
//! - prompt_pack_append_guidance：追加提示到基础提示末尾

use crate::shared::error::{AppError, IpcResponse};
use super::capability_registry::CapabilitySkillRegistry;
use super::capability_types::{
    CapabilitySkillManifest, PromptPackManifest, SkillResolutionInput, SkillResolutionResult,
};
use super::prompt_pack::PromptPackLoader;
use tauri::State;
use tokio::sync::Mutex;
use std::time::Instant;

/// Capability Skill + PromptPack 共享状态
pub struct CapabilityState {
    pub registry: CapabilitySkillRegistry,
    pub loader: Mutex<PromptPackLoader>,
}

impl Default for CapabilityState {
    fn default() -> Self {
        Self {
            registry: CapabilitySkillRegistry::new(),
            loader: Mutex::new(PromptPackLoader::new()),
        }
    }
}

impl CapabilityState {
    /// 创建带 project_root / user_root 的状态
    pub fn new(project_root: Option<std::path::PathBuf>, user_root: Option<std::path::PathBuf>) -> Self {
        let mut loader = PromptPackLoader::new();
        if let Some(p) = project_root {
            loader = loader.with_project_root(p);
        }
        if let Some(u) = user_root {
            loader = loader.with_user_root(u);
        }
        Self {
            registry: CapabilitySkillRegistry::new(),
            loader: Mutex::new(loader),
        }
    }
}

#[tauri::command]
pub async fn capability_skill_list(
    state: State<'_, CapabilityState>,
) -> Result<IpcResponse<Vec<CapabilitySkillManifest>>, AppError> {
    let start = Instant::now();
    tracing::info!("capability_skill_list: enter");
    
    let skills = state.registry.list_skills().to_vec();
    
    tracing::info!(
        count = skills.len(),
        duration_ms = start.elapsed().as_millis(),
        "capability_skill_list: exit"
    );
    Ok(IpcResponse::ok(skills))
}

#[tauri::command]
pub async fn capability_skill_get(
    state: State<'_, CapabilityState>,
    id: String,
) -> Result<IpcResponse<CapabilitySkillManifest>, AppError> {
    let start = Instant::now();
    tracing::info!(id = %id, "capability_skill_get: enter");
    
    let skill = state
        .registry
        .get_skill(&id)
        .ok_or_else(|| {
            tracing::error!(id = %id, "capability_skill_get: Skill not found");
            AppError::not_found(format!("Capability skill not found: {}", id))
        })?;
    
    tracing::info!(
        id = %id,
        duration_ms = start.elapsed().as_millis(),
        "capability_skill_get: exit"
    );
    Ok(IpcResponse::ok(skill.clone()))
}

#[tauri::command]
pub async fn capability_skill_resolve(
    state: State<'_, CapabilityState>,
    input: SkillResolutionInput,
) -> Result<IpcResponse<SkillResolutionResult>, AppError> {
    let start = Instant::now();
    tracing::info!("capability_skill_resolve: enter");
    
    let result = state.registry.resolve_skills(&input);
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "capability_skill_resolve: exit"
    );
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn prompt_pack_list(
    state: State<'_, CapabilityState>,
) -> Result<IpcResponse<Vec<PromptPackManifest>>, AppError> {
    let start = Instant::now();
    tracing::info!("prompt_pack_list: enter");
    
    let loader = state.loader.lock().await;
    let packs = loader.list_builtin_prompt_packs();
    
    tracing::info!(
        count = packs.len(),
        duration_ms = start.elapsed().as_millis(),
        "prompt_pack_list: exit"
    );
    Ok(IpcResponse::ok(packs))
}

#[tauri::command]
pub async fn prompt_pack_get(
    state: State<'_, CapabilityState>,
    prompt_id: String,
) -> Result<IpcResponse<super::capability_types::LoadedPromptPackPrompt>, AppError> {
    let start = Instant::now();
    tracing::info!(prompt_id = %prompt_id, "prompt_pack_get: enter");
    
    let loader = state.loader.lock().await;
    let prompt = loader.load_prompt(&prompt_id).map_err(|e| {
        tracing::error!(prompt_id = %prompt_id, error = %e, "prompt_pack_get: Failed to load prompt");
        e
    })?;
    
    tracing::info!(
        prompt_id = %prompt_id,
        duration_ms = start.elapsed().as_millis(),
        "prompt_pack_get: exit"
    );
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn prompt_pack_append_guidance(
    state: State<'_, CapabilityState>,
    base_prompt: String,
    prompt_id: String,
) -> Result<IpcResponse<String>, AppError> {
    let start = Instant::now();
    tracing::info!(
        prompt_id = %prompt_id,
        base_len = base_prompt.len(),
        "prompt_pack_append_guidance: enter"
    );
    
    let loader = state.loader.lock().await;
    let result = loader.append_guidance(&base_prompt, &prompt_id).map_err(|e| {
        tracing::error!(prompt_id = %prompt_id, error = %e, "prompt_pack_append_guidance: Failed to append guidance");
        e
    })?;
    
    tracing::info!(
        prompt_id = %prompt_id,
        result_len = result.len(),
        duration_ms = start.elapsed().as_millis(),
        "prompt_pack_append_guidance: exit"
    );
    Ok(IpcResponse::ok(result))
}