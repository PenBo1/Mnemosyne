//! ═══════════════════════════════════════════════════════════════════════════
//! LLM Commands - LLM 应用层 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供 LLM 相关的应用层命令。此文件位于 application 层，允许依赖 core::agent。

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::llm::state::LlmState;
use crate::core::agent::PromptOptimizer;
use serde::{Deserialize, Serialize};
use tauri::State;

// ── 响应类型 ────────────────────────────────────────────────────────────────

/// 提示词优化响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PromptOptimizeResponse {
    pub optimized_prompt: String,
}

// ── IPC 命令 ─────────────────────────────────────────────────────────────────

/// 优化提示词
///
/// 使用当前激活的模型对用户提示词进行优化。
/// 业务逻辑委托给 core::agent::PromptOptimizer。
#[tauri::command]
pub async fn prompt_optimize(
    state: State<'_, LlmState>,
    prompt: String,
) -> Result<IpcResponse<PromptOptimizeResponse>, AppError> {
    // 参数校验
    if prompt.trim().is_empty() {
        return Err(AppError::invalid_input("Prompt cannot be empty"));
    }
    if prompt.len() > 100_000 {
        return Err(AppError::invalid_input("Prompt too long (max 100KB)"));
    }

    // 委托给核心层
    let registry = state.registry.lock().await;
    let optimized = PromptOptimizer::optimize(&registry, &prompt).await?;

    Ok(IpcResponse::ok(PromptOptimizeResponse {
        optimized_prompt: optimized,
    }))
}