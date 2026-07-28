//! ═══════════════════════════════════════════════════════════════════════════
//! engine - Hook 引擎模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use crate::security_kernel::audit::{SecurityEvent, SharedAuditEventBus};
use crate::security_kernel::types::WorkspaceId;
use crate::shared::error::AppError;

use super::registry::{HookDispatchOutcome, HookRegistry};
use super::types::HookPayload;

// ── Hook 引擎 ────────────────────────────────────────────────────────────────

pub struct HookEngine {
    registry: Arc<HookRegistry>,
    audit_bus: SharedAuditEventBus,
}

impl HookEngine {
    pub fn new(registry: Arc<HookRegistry>, audit_bus: SharedAuditEventBus) -> Self {
        Self {
            registry,
            audit_bus,
        }
    }

    /// 引用底层 registry（供 IPC 命令直接操作）。
    pub fn registry(&self) -> &Arc<HookRegistry> {
        &self.registry
    }

    /// 派发 hook —— 按 priority 顺序执行匹配的 hook，aborted 时返回 Err。
    ///
    /// 无论是否 aborted，都会 emit `HookDispatched` 审计事件。
    pub async fn dispatch(&self, payload: &HookPayload) -> Result<HookDispatchOutcome, AppError> {
        tracing::warn!(
            operation = "hook_dispatch",
            decision = "Processing",
            event = payload.event.as_str(),
            tool_name = payload.tool_name.as_deref().unwrap_or("none"),
            workspace = payload.workspace_id.as_deref().unwrap_or("none"),
            "hook_engine: dispatching hook"
        );

        let outcome = self.registry.dispatch(payload.event, payload).await;
        self.emit_audit(&outcome, payload);

        if outcome.aborted {
            tracing::error!(
                operation = "hook_dispatch",
                decision = "Deny",
                reason = "Hook aborted by Block action",
                event = payload.event.as_str(),
                dispatched_count = outcome.dispatched_count,
                "hook_engine: hook aborted"
            );
            return Err(AppError::forbidden(format!(
                "Hook '{}' aborted by Block action (triggered {} hooks)",
                payload.event.as_str(),
                outcome.dispatched_count
            )));
        }

        tracing::warn!(
            operation = "hook_dispatch",
            decision = "Allow",
            event = payload.event.as_str(),
            dispatched_count = outcome.dispatched_count,
            "hook_engine: hook dispatch completed"
        );

        Ok(outcome)
    }

    fn emit_audit(&self, outcome: &HookDispatchOutcome, payload: &HookPayload) {
        let workspace = payload
            .workspace_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .map(WorkspaceId);

        self.audit_bus.emit(SecurityEvent::HookDispatched {
            event: payload.event.as_str().to_string(),
            tool_name: payload.tool_name.clone(),
            workspace,
            dispatched_count: outcome.dispatched_count as u32,
            aborted: outcome.aborted,
            timestamp: payload.timestamp,
        });
    }
}