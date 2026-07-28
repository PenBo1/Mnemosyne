//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Agents - 剧本分镜执行函数
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：剧本 / 分镜 / 互动影游创作 agent 执行入口。

use std::time::Instant;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::parsing::normalize_episode_end_labels;
use super::prompts::{
    build_interactive_film_system_prompt, build_interactive_film_user_prompt,
    build_script_system_prompt, build_script_user_prompt, build_storyboard_system_prompt,
    build_storyboard_user_prompt,
};
use super::types::{
    InteractiveFilmCreationInput, ScriptCreationInput, StoryboardCreationInput,
};

/// 1. 剧本创作（temp=0.55, maxTokens=min(32000, max(12000, episodes*2200))）。
pub async fn write_script(
    engine: &AgentEngine,
    input: &ScriptCreationInput,
) -> Result<String, AppError> {
    let start = Instant::now();
    tracing::info!(function = "write_script", title = %input.title, target_format = ?input.target_format, "入口");

    let language = input.language.unwrap_or_default();
    let system_prompt = build_script_system_prompt(language);
    let user_message = build_script_user_prompt(input, language);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let result = normalize_episode_end_labels(response.trim(), 1);
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "write_script", title = %input.title, duration_ms, "出口");
            Ok(result)
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "write_script", title = %input.title, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 2. 分镜创作（temp=0.45, maxTokens=min(24000, max(10000, shots*700))）。
pub async fn write_storyboard(
    engine: &AgentEngine,
    input: &StoryboardCreationInput,
) -> Result<String, AppError> {
    let start = Instant::now();
    tracing::info!(function = "write_storyboard", title = %input.title, max_shots = input.max_shots, "入口");

    let language = input.language.unwrap_or_default();
    let system_prompt = build_storyboard_system_prompt(language);
    let user_message = build_storyboard_user_prompt(input, language);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "write_storyboard", title = %input.title, duration_ms, "出口");
            Ok(response.trim().to_string())
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "write_storyboard", title = %input.title, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}

/// 3. 互动影游创作（temp=0.5, maxTokens=min(36000, max(16000, episodes*3000))）。
pub async fn write_interactive_film(
    engine: &AgentEngine,
    input: &InteractiveFilmCreationInput,
) -> Result<String, AppError> {
    let start = Instant::now();
    tracing::info!(function = "write_interactive_film", title = %input.title, episode_count = input.episode_count, "入口");

    let language = input.language.unwrap_or_default();
    let system_prompt = build_interactive_film_system_prompt(language);
    let user_message = build_interactive_film_user_prompt(input, language);
    match engine.prompt_once(&system_prompt, &user_message).await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "write_interactive_film", title = %input.title, duration_ms, "出口");
            Ok(response.trim().to_string())
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::error!(function = "write_interactive_film", title = %input.title, duration_ms, error = %e, "错误");
            Err(e)
        }
    }
}
