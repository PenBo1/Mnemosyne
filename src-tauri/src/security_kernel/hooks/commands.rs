// Hook IPC 命令 —— 配置型 hook 管理 + 测试派发。
//
// 设计要点：
// - IPC 仅支持配置型 hook（action = Log/Audit/Block/Custom），不支持注入函数。
// - hook_register 接收 HookConfig，内部由 registry 映射 action → handler。
// - hook_test_dispatch 用于调试 —— 模拟一次 hook 触发，返回每个被触发 hook 的结果。
// - 所有命令通过 HookEngineState 访问 HookEngine。

use tauri::State;

use crate::shared::error::{AppError, IpcResponse};

use super::engine::HookEngine;
use super::registry::HookDispatchOutcome;
use super::types::{HookConfig, HookInfo, HookPayload, HookTestRequest, HookTestResult};

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

// ── IPC 命令 ──

/// 列出所有已注册 hook。
#[tauri::command]
pub async fn hook_list(
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<Vec<HookInfo>>, AppError> {
    Ok(IpcResponse::ok(state.registry().list()))
}

/// 注册配置型 hook。返回生成的 hook id。
#[tauri::command]
pub async fn hook_register(
    config: HookConfig,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<String>, AppError> {
    // 校验：matcher 若提供 tool_name_pattern，必须可被 glob 解析
    if let Some(matcher) = &config.matcher {
        if let Some(pattern) = &matcher.tool_name_pattern {
            if glob::Pattern::new(pattern).is_err() {
                return Err(AppError::invalid_input(format!(
                    "Invalid glob pattern in matcher.toolNamePattern: {}",
                    pattern
                )));
            }
        }
    }

    let id = state.registry().register_config(config)?;
    Ok(IpcResponse::created(id))
}

/// 注销 hook。返回是否成功删除。
#[tauri::command]
pub async fn hook_unregister(
    id: String,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<bool>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::missing_field("id"));
    }
    let removed = state.registry().unregister(&id);
    if !removed {
        return Err(AppError::not_found(format!("Hook '{}' not found", id)));
    }
    Ok(IpcResponse::deleted(true))
}

/// 测试派发 —— 模拟一次 hook 触发，返回每个被触发 hook 的结果汇总。
/// 用于调试和验证 matcher 配置。
///
/// 注意：测试场景下即使有 hook 返回 Block（FailedAbort），命令仍返回 200，
/// 通过 `aborted: true` 字段表达拦截状态。
///
/// 直接调用 registry.dispatch（绕过 engine.dispatch 的 Err 转换），
/// 以便在 abort 时仍能返回实际 triggered_ids。
#[tauri::command]
pub async fn hook_test_dispatch(
    request: HookTestRequest,
    state: State<'_, HookEngineState>,
) -> Result<IpcResponse<HookTestResult>, AppError> {
    let mut payload = HookPayload::new(request.event);
    payload.tool_name = request.tool_name;
    payload.workspace_id = request.workspace_id;
    payload.session_id = request.session_id;
    payload.agent_role = request.agent_role;
    payload.tool_args = request.tool_args;

    let outcome: HookDispatchOutcome = state.registry().dispatch(request.event, &payload).await;

    Ok(IpcResponse::ok(HookTestResult::from(outcome)))
}
