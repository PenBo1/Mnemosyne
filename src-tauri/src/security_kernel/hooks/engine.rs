// Hook 引擎 —— 包装 HookRegistry，提供带审计的派发接口。
//
// 职责：
// - 持有 HookRegistry（Arc 共享）
// - 持有 SharedAuditEventBus，每次派发后 emit `HookDispatched` 事件
// - 提供 `dispatch` async 方法：派发 hook，aborted 时返回 Err
//
// 不持有 SecurityKernel 引用（避免循环依赖）。kernel 反向持有 hook_engine。

use std::sync::Arc;

use crate::security_kernel::audit::{SecurityEvent, SharedAuditEventBus};
use crate::security_kernel::types::WorkspaceId;
use crate::shared::error::AppError;

use super::registry::{HookDispatchOutcome, HookRegistry};
use super::types::HookPayload;

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
        let outcome = self.registry.dispatch(payload.event, payload).await;
        self.emit_audit(&outcome, payload);
        if outcome.aborted {
            return Err(AppError::forbidden(format!(
                "Hook '{}' aborted by Block action (triggered {} hooks)",
                payload.event.as_str(),
                outcome.dispatched_count
            )));
        }
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
