//! ═══════════════════════════════════════════════════════════════════════════
//! commands - Hook IPC 命令模块
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::time::Instant;

use crate::shared::error::{AppError, IpcResponse};

use super::engine::HookEngine;
use super::registry::HookDispatchOutcome;
use super::types::{HookConfig, HookInfo, HookPayload, HookTestRequest, HookTestResult};

// ── Tauri State 包装 ────────────────────────────────────────────────────────

/// Tauri State 包装 —— 在 setup 中注入到 app.manage。
///
/// 持有 `Arc<HookEngine>`（与 SecurityKernel 共享同一实例）。
#[derive(Clone)]
pub struct HookEngineState {
    engine: std::sync::Arc<HookEngine>,
}

impl HookEngineState {
    /// 从 SecurityKernel 共享的 Arc<HookEngine> 构造。
    pub fn from_arc(engine: std::sync::Arc<HookEngine>) -> Self {
        Self { engine }
    }

    pub fn engine(&self) -> &HookEngine {
        &self.engine
    }
}

impl std::ops::Deref for HookEngineState {
    type Target = HookEngine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

// ── IPC 命令 ────────────────────────────────────────────────────────────────

/// 列出所有已注册 hook。
#[tauri::command]
pub async fn hook_list(
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<Vec<HookInfo>>, AppError> {
    let start = Instant::now();
    tracing::info!("hook_list: enter");

    let hooks = state.registry().list();

    tracing::info!(
        count = hooks.len(),
        duration_ms = start.elapsed().as_millis(),
        "hook_list: exit"
    );
    Ok(IpcResponse::ok(hooks))
}

/// 注册配置型 hook。返回生成的 hook id。
#[tauri::command]
pub async fn hook_register(
    config: HookConfig,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<String>, AppError> {
    let start = Instant::now();
    tracing::info!("hook_register: enter");

    if let Some(matcher) = &config.matcher {
        if let Some(pattern) = &matcher.tool_name_pattern {
            match pattern {
                super::types::MatcherPattern::Glob(g) => {
                    if glob::Pattern::new(g).is_err() {
                        tracing::error!(pattern = %g, "hook_register: Invalid glob pattern");
                        return Err(AppError::invalid_input(format!(
                            "Invalid glob pattern in matcher.toolNamePattern: {}",
                            g
                        )));
                    }
                }
                super::types::MatcherPattern::Regex(r) => {
                    if regex::Regex::new(r).is_err() {
                        tracing::error!(pattern = %r, "hook_register: Invalid regex pattern");
                        return Err(AppError::invalid_input(format!(
                            "Invalid regex pattern in matcher.toolNamePattern: {}",
                            r
                        )));
                    }
                }
                _ => {}
            }
        }
    }

    let id = state.registry().register_config(config).map_err(|e| {
        tracing::error!(error = %e, "hook_register: Failed to register hook");
        e
    })?;

    tracing::info!(
        hook_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "hook_register: exit"
    );
    Ok(IpcResponse::created(id))
}

/// 注销 hook。返回是否成功删除。
#[tauri::command]
pub async fn hook_unregister(
    id: String,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(hook_id = %id, "hook_unregister: enter");

    if id.trim().is_empty() {
        tracing::error!("hook_unregister: id cannot be empty");
        return Err(AppError::missing_field("id"));
    }

    let removed = state.registry().unregister(&id);
    if !removed {
        tracing::error!(hook_id = %id, "hook_unregister: Hook not found");
        return Err(AppError::not_found(format!("Hook '{}' not found", id)));
    }

    tracing::info!(
        hook_id = %id,
        duration_ms = start.elapsed().as_millis(),
        "hook_unregister: exit"
    );
    Ok(IpcResponse::deleted(true))
}

/// 测试派发 —— 模拟一次 hook 触发，返回每个被触发 hook 的结果汇总。
/// 用于调试和验证 matcher 配置。
///
/// 注意：测试场景下即使有 hook 返回 Block（FailedAbort），命令仍返回 200，
/// 通过 `aborted: true` 字段表达拦截状态。
#[tauri::command]
pub async fn hook_test_dispatch(
    request: HookTestRequest,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<HookTestResult>, AppError> {
    let start = Instant::now();
    tracing::info!(
        event = ?request.event,
        tool_name = ?request.tool_name,
        "hook_test_dispatch: enter"
    );

    let mut payload = HookPayload::new(request.event);
    payload.tool_name = request.tool_name;
    payload.workspace_id = request.workspace_id;
    payload.session_id = request.session_id;
    payload.agent_role = request.agent_role;
    payload.tool_args = request.tool_args;

    let outcome: HookDispatchOutcome = state.registry().dispatch(request.event, &payload).await;
    let result = HookTestResult::from(outcome);

    tracing::info!(
        triggered_count = result.triggered_ids.len(),
        aborted = result.aborted,
        duration_ms = start.elapsed().as_millis(),
        "hook_test_dispatch: exit"
    );
    Ok(IpcResponse::ok(result))
}