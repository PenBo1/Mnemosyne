//! ═══════════════════════════════════════════════════════════════════════════
//! dispatcher - Hook 派发器模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use async_trait::async_trait;

use crate::shared::error::AppError;

use super::engine::HookEngine;
use super::types::{HookEvent, HookPayload};

// ── Hook 派发器 trait ────────────────────────────────────────────────────────

/// Hook 派发器 trait —— 抽象 hook 派发逻辑，供 AgentEngine / SubAgentExecutor 使用。
#[async_trait]
pub trait HookDispatcher: Send + Sync {
    /// 异步派发 hook。aborted 时返回 Err（调用方决定是否中止主流程）。
    async fn dispatch_hook(&self, payload: &HookPayload) -> Result<(), AppError>;

    /// 快速检查：registry 是否有已注册 hook。
    /// 空时调用方可跳过 payload 构造与派发，避免无谓开销。
    fn has_hooks(&self) -> bool;
}

#[async_trait]
impl HookDispatcher for HookEngine {
    async fn dispatch_hook(&self, payload: &HookPayload) -> Result<(), AppError> {
        // registry 为空时快速返回（避免无 hook 时的无谓 audit emit）
        if self.registry().count() == 0 {
            tracing::debug!(
                operation = "dispatch_hook",
                decision = "Skip",
                reason = "No hooks registered",
                "hook_dispatcher: skipped (no hooks)"
            );
            return Ok(());
        }

        tracing::warn!(
            operation = "dispatch_hook",
            decision = "Processing",
            event = payload.event.as_str(),
            "hook_dispatcher: dispatching hook"
        );

        match self.dispatch(payload).await {
            Ok(_) => {
                tracing::warn!(
                    operation = "dispatch_hook",
                    decision = "Allow",
                    event = payload.event.as_str(),
                    "hook_dispatcher: hook dispatch succeeded"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    operation = "dispatch_hook",
                    decision = "Deny",
                    reason = e.message.as_str(),
                    event = payload.event.as_str(),
                    "hook_dispatcher: hook dispatch failed"
                );
                Err(e)
            }
        }
    }

    fn has_hooks(&self) -> bool {
        self.registry().count() > 0
    }
}

// ── HookPayload 构造辅助函数 ────────────────────────────────────────────────

/// 构造 SessionStart payload。
pub fn session_start_payload(
    session_id: &str,
    workspace_id: Option<&str>,
    agent_role: Option<&str>,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::SessionStart)
        .with_session(session_id)
        .with_metadata("event_source", serde_json::Value::String("agent_engine".to_string()));
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    if let Some(role) = agent_role {
        payload = payload.with_agent_role(role);
    }
    payload
}

/// 构造 UserPromptSubmit payload。
pub fn user_prompt_submit_payload(
    session_id: &str,
    workspace_id: Option<&str>,
    content: &str,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::UserPromptSubmit)
        .with_session(session_id)
        .with_metadata("event_source", serde_json::Value::String("agent_engine".to_string()))
        .with_metadata(
            "prompt_length",
            serde_json::Value::Number(serde_json::Number::from(content.len() as u64)),
        );
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

/// 构造 Stop payload（主 agent 停止）。
pub fn stop_payload(
    session_id: &str,
    workspace_id: Option<&str>,
    success: bool,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::Stop)
        .with_session(session_id)
        .with_metadata("event_source", serde_json::Value::String("agent_engine".to_string()))
        .with_metadata("success", serde_json::Value::Bool(success));
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

/// 构造 SubagentStart payload。
pub fn subagent_start_payload(
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    role: &str,
    task: &str,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::SubagentStart)
        .with_agent_role(role)
        .with_metadata("event_source", serde_json::Value::String("subagent_executor".to_string()))
        .with_metadata("task", serde_json::Value::String(task.to_string()));
    if let Some(s) = session_id {
        payload = payload.with_session(s);
    }
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

/// 构造 SubagentStop payload。
pub fn subagent_stop_payload(
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    role: &str,
    success: bool,
    duration_ms: u64,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::SubagentStop)
        .with_agent_role(role)
        .with_metadata("event_source", serde_json::Value::String("subagent_executor".to_string()))
        .with_metadata("success", serde_json::Value::Bool(success))
        .with_metadata(
            "duration_ms",
            serde_json::Value::Number(serde_json::Number::from(duration_ms)),
        );
    if let Some(s) = session_id {
        payload = payload.with_session(s);
    }
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

/// 构造 PreCompact payload（预留，当前 Rust 侧无 compact 逻辑）。
pub fn pre_compact_payload(session_id: &str, workspace_id: Option<&str>) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::PreCompact)
        .with_session(session_id)
        .with_metadata("event_source", serde_json::Value::String("agent_engine".to_string()));
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

/// 构造 PostCompact payload（预留，当前 Rust 侧无 compact 逻辑）。
pub fn post_compact_payload(
    session_id: &str,
    workspace_id: Option<&str>,
    original_tokens: u64,
    compacted_tokens: u64,
) -> HookPayload {
    let mut payload = HookPayload::new(HookEvent::PostCompact)
        .with_session(session_id)
        .with_metadata("event_source", serde_json::Value::String("agent_engine".to_string()))
        .with_metadata(
            "original_tokens",
            serde_json::Value::Number(serde_json::Number::from(original_tokens)),
        )
        .with_metadata(
            "compacted_tokens",
            serde_json::Value::Number(serde_json::Number::from(compacted_tokens)),
        );
    if let Some(ws) = workspace_id {
        payload = payload.with_workspace(ws);
    }
    payload
}

// ── 类型别名与辅助函数 ────────────────────────────────────────────────

/// 类型别名：Option<Arc<HookEngine>> 作为 AgentEngine 的 hook 字段类型。
pub type OptionalHookDispatcher = Option<Arc<HookEngine>>;

/// 在有 dispatcher 时派发 hook（None 时静默跳过，不报错）。
///
/// 用于 AgentEngine / SubAgentExecutor 的 hook 集成点：
/// dispatcher 为 None 表示未注入 HookEngine（如测试环境），跳过即可。
pub async fn try_dispatch(
    dispatcher: &OptionalHookDispatcher,
    payload: &HookPayload,
) -> Result<(), AppError> {
    if let Some(d) = dispatcher {
        tracing::warn!(
            operation = "try_dispatch",
            decision = "Processing",
            event = payload.event.as_str(),
            has_dispatcher = true,
            "hook_dispatcher: trying to dispatch hook"
        );
        d.dispatch_hook(payload).await?;
    } else {
        tracing::debug!(
            operation = "try_dispatch",
            decision = "Skip",
            reason = "No dispatcher provided",
            event = payload.event.as_str(),
            "hook_dispatcher: skipped (no dispatcher)"
        );
    }
    Ok(())
}