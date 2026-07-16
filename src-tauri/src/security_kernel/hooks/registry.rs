// Hook 注册表 —— 存储 ConfiguredHook，按 priority 降序派发。
//
// 派发语义：
// - 同一 event 的所有 hook 按 priority 降序执行。
// - matcher 不匹配的 hook 跳过。
// - 任意 hook 返回 FailedAbort → 立即停止后续 hook，返回 Err。
// - 任意 hook 返回 FailedContinue → 记录警告，继续后续。
// - handler 内部 panic 会被 catch_unwind 捕获并视为 FailedContinue（不阻断链）。
//
// 内置 action → handler 映射：
// - Log: tracing::info! 记录 payload
// - Audit: 通过 audit_bus 发送 SecurityEvent::HookDispatched
// - Block: 直接返回 FailedAbort
// - Custom(_): 等同 Log

use std::sync::{Arc, RwLock};

use futures_util::FutureExt;
use uuid::Uuid;

use crate::shared::error::AppError;

use super::types::{
    ConfiguredHook, HookAction, HookConfig, HookEvent, HookFn, HookInfo, HookMatcher, HookPayload,
    HookResult, HookTestResult,
};

/// Hook 派发结果（内部用）。
#[derive(Debug, Clone)]
pub struct HookDispatchOutcome {
    pub dispatched_count: usize,
    pub aborted: bool,
    pub triggered_ids: Vec<String>,
}

impl HookDispatchOutcome {
    pub fn ok(count: usize, ids: Vec<String>) -> Self {
        Self {
            dispatched_count: count,
            aborted: false,
            triggered_ids: ids,
        }
    }

    pub fn aborted(ids: Vec<String>) -> Self {
        let count = ids.len();
        Self {
            dispatched_count: count,
            aborted: true,
            triggered_ids: ids,
        }
    }
}

impl From<HookDispatchOutcome> for HookTestResult {
    fn from(o: HookDispatchOutcome) -> Self {
        Self {
            dispatched_count: o.dispatched_count,
            aborted: o.aborted,
            triggered_ids: o.triggered_ids,
        }
    }
}

/// Hook 注册表 —— 线程安全（RwLock）。
pub struct HookRegistry {
    hooks: RwLock<Vec<ConfiguredHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
        }
    }

    /// 注册一个完整 ConfiguredHook（内部使用，handler 由调用方提供）。
    pub fn register_hook(&self, hook: ConfiguredHook) -> String {
        let id = hook.id.clone();
        let mut hooks = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        hooks.push(hook);
        // 按 priority 降序排序（稳定排序保留插入顺序）
        hooks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tracing::debug!(hook_id = %id, "Hook registered");
        id
    }

    /// 配置型注册（IPC 路径）—— action 决定内置 handler。
    /// 返回 hook id。
    pub fn register_config(&self, config: HookConfig) -> Result<String, AppError> {
        // 校验 priority 合理范围（防止极端值影响排序）
        if config.priority < -1000 || config.priority > 1000 {
            return Err(AppError::invalid_input(format!(
                "Hook priority out of range: {} (allowed -1000..=1000)",
                config.priority
            )));
        }

        let id = config
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let handler = handler_for_action(&config.action);
        let hook = ConfiguredHook {
            id: id.clone(),
            event: config.event,
            matcher: config.matcher,
            handler,
            priority: config.priority,
            action: Some(config.action),
        };
        self.register_hook(hook);
        Ok(id)
    }

    /// 注销 hook。返回是否成功删除。
    pub fn unregister(&self, id: &str) -> bool {
        let mut hooks = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        let before = hooks.len();
        hooks.retain(|h| h.id != id);
        let removed = before != hooks.len();
        if removed {
            tracing::debug!(hook_id = %id, "Hook unregistered");
        }
        removed
    }

    /// 列出所有已注册 hook 的描述信息。
    pub fn list(&self) -> Vec<HookInfo> {
        let hooks = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        hooks.iter().map(HookInfo::from).collect()
    }

    /// 统计已注册 hook 数量。
    pub fn count(&self) -> usize {
        self.hooks.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// 异步派发 hook —— 按 priority 顺序执行匹配的 hook。
    ///
    /// 失败语义：
    /// - 任意 hook 返回 FailedAbort → 立即返回 `outcome.aborted = true`。
    /// - 任意 hook 返回 FailedContinue → 记录 warn，继续。
    /// - handler 内部 panic → 记录 error，视为 FailedContinue。
    pub async fn dispatch(&self, event: HookEvent, payload: &HookPayload) -> HookDispatchOutcome {
        let matched: Vec<ConfiguredHook> = {
            let hooks = self.hooks.read().unwrap_or_else(|e| e.into_inner());
            hooks
                .iter()
                .filter(|h| h.event == event && matcher_matches(&h.matcher, payload))
                .cloned()
                .collect()
        };

        let mut triggered_ids = Vec::with_capacity(matched.len());
        for hook in matched {
            triggered_ids.push(hook.id.clone());
            let result = invoke_handler(&hook.handler, payload).await;
            match result {
                HookResult::Success => {
                    tracing::trace!(hook_id = %hook.id, event = event.as_str(), "Hook success");
                }
                HookResult::FailedContinue => {
                    tracing::warn!(
                        hook_id = %hook.id,
                        event = event.as_str(),
                        "Hook returned FailedContinue"
                    );
                }
                HookResult::FailedAbort => {
                    tracing::warn!(
                        hook_id = %hook.id,
                        event = event.as_str(),
                        "Hook returned FailedAbort — aborting dispatch chain"
                    );
                    return HookDispatchOutcome::aborted(triggered_ids);
                }
            }
        }

        HookDispatchOutcome::ok(triggered_ids.len(), triggered_ids)
    }

    /// 清空所有 hook（用于测试或重置）。
    pub fn clear(&self) {
        let mut hooks = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        hooks.clear();
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 ──

fn matcher_matches(matcher: &Option<HookMatcher>, payload: &HookPayload) -> bool {
    match matcher {
        Some(m) => m.matches(payload),
        None => true,
    }
}

/// 调用 hook handler —— 包裹 catch_unwind 以防 handler panic 阻断链。
///
/// handler 是 `Arc<dyn Fn(&HookPayload) -> BoxFuture<'static, HookResult>>`。
/// 调用 handler 返回 future，await future 时若 panic 则被 `catch_unwind` 捕获，
/// 视为 `FailedContinue`（不阻断后续 hook）。
async fn invoke_handler(handler: &HookFn, payload: &HookPayload) -> HookResult {
    let fut = handler(payload);
    std::panic::AssertUnwindSafe(fut)
        .catch_unwind()
        .await
        .unwrap_or_else(|_| {
            tracing::error!("Hook handler panicked — treating as FailedContinue");
            HookResult::FailedContinue
        })
}

/// 内置 action → handler 映射。
///
/// 每个 handler 都是 `Arc<dyn Fn(&HookPayload) -> BoxFuture<'static, HookResult>>`。
/// handler 内部克隆所需数据后返回 'static future。
pub fn handler_for_action(action: &HookAction) -> HookFn {
    match action {
        HookAction::Log => Arc::new(|payload: &HookPayload| {
            let event = payload.event;
            let tool = payload.tool_name.clone();
            let ws = payload.workspace_id.clone();
            let session = payload.session_id.clone();
            Box::pin(async move {
                tracing::info!(
                    event = event.as_str(),
                    tool = ?tool,
                    workspace = ?ws,
                    session = ?session,
                    "Hook Log action"
                );
                HookResult::Success
            })
        }),
        HookAction::Audit => Arc::new(|payload: &HookPayload| {
            // Audit 事件通过 tracing 记录（实际 audit_bus 集成在 HookEngine 中，
            // 避免在 handler 内部持有 audit_bus 引用造成循环依赖）。
            let event = payload.event;
            let tool = payload.tool_name.clone();
            let ws = payload.workspace_id.clone();
            let ts = payload.timestamp;
            Box::pin(async move {
                tracing::info!(
                    event = event.as_str(),
                    tool = ?tool,
                    workspace = ?ws,
                    timestamp = %ts,
                    "Hook Audit action — would emit SecurityEvent"
                );
                HookResult::Success
            })
        }),
        HookAction::Block => Arc::new(|payload: &HookPayload| {
            let event = payload.event;
            let tool = payload.tool_name.clone();
            Box::pin(async move {
                tracing::warn!(
                    event = event.as_str(),
                    tool = ?tool,
                    "Hook Block action — aborting"
                );
                HookResult::FailedAbort
            })
        }),
        HookAction::Custom(desc) => {
            let desc = desc.clone();
            Arc::new(move |payload: &HookPayload| {
                let event = payload.event;
                let tool = payload.tool_name.clone();
                let desc = desc.clone();
                Box::pin(async move {
                    tracing::info!(
                        event = event.as_str(),
                        tool = ?tool,
                        custom = %desc,
                        "Hook Custom action"
                    );
                    HookResult::Success
                })
            })
        }
    }
}
