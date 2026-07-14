// Capability Skill + PromptPack IPC 命令
//
// 暴露给前端的命令:
// - capability_skill_list:         列出所有 builtin capability skills
// - capability_skill_get:          按 id 获取单个 skill
// - capability_skill_resolve:      解析技能(三阶段:forced/candidate/auto)
// - prompt_pack_list:              列出所有 builtin prompt packs
// - prompt_pack_get:               按 promptId 加载单个 prompt(三层覆盖)
// - prompt_pack_append_guidance:   把 prompt 追加到 base prompt 末尾

use crate::shared::error::{AppError, IpcResponse};
use super::capability_registry::CapabilitySkillRegistry;
use super::capability_types::{
    CapabilitySkillManifest, PromptPackManifest, SkillResolutionInput, SkillResolutionResult,
};
use super::prompt_pack::PromptPackLoader;
use tauri::State;
use tokio::sync::Mutex;

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
    let skills = state.registry.list_skills().to_vec();
    Ok(IpcResponse::ok(skills))
}

#[tauri::command]
pub async fn capability_skill_get(
    state: State<'_, CapabilityState>,
    id: String,
) -> Result<IpcResponse<CapabilitySkillManifest>, AppError> {
    let skill = state
        .registry
        .get_skill(&id)
        .ok_or_else(|| AppError::not_found(format!("Capability skill not found: {}", id)))?;
    Ok(IpcResponse::ok(skill.clone()))
}

#[tauri::command]
pub async fn capability_skill_resolve(
    state: State<'_, CapabilityState>,
    input: SkillResolutionInput,
) -> Result<IpcResponse<SkillResolutionResult>, AppError> {
    let result = state.registry.resolve_skills(&input);
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn prompt_pack_list(
    state: State<'_, CapabilityState>,
) -> Result<IpcResponse<Vec<PromptPackManifest>>, AppError> {
    let loader = state.loader.lock().await;
    Ok(IpcResponse::ok(loader.list_builtin_prompt_packs()))
}

#[tauri::command]
pub async fn prompt_pack_get(
    state: State<'_, CapabilityState>,
    prompt_id: String,
) -> Result<IpcResponse<super::capability_types::LoadedPromptPackPrompt>, AppError> {
    let loader = state.loader.lock().await;
    let prompt = loader.load_prompt(&prompt_id)?;
    Ok(IpcResponse::ok(prompt))
}

#[tauri::command]
pub async fn prompt_pack_append_guidance(
    state: State<'_, CapabilityState>,
    base_prompt: String,
    prompt_id: String,
) -> Result<IpcResponse<String>, AppError> {
    let loader = state.loader.lock().await;
    let result = loader.append_guidance(&base_prompt, &prompt_id)?;
    Ok(IpcResponse::ok(result))
}
