use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::WorkspaceId;
use super::super::types::RiskLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    OperationStart {
        operation: String,
        workspace: WorkspaceId,
        timestamp: DateTime<Utc>,
    },
    OperationComplete {
        operation: String,
        workspace: WorkspaceId,
        duration_ms: u64,
        success: bool,
    },
    ApprovalRequested {
        approval_id: Uuid,
        operation: String,
        risk_level: RiskLevel,
        workspace: WorkspaceId,
    },
    ApprovalGranted {
        approval_id: Uuid,
        approved_by: String,
        workspace: WorkspaceId,
    },
    ApprovalRejected {
        approval_id: Uuid,
        rejected_by: String,
        reason: String,
        workspace: WorkspaceId,
    },
    PolicyDenied {
        operation: String,
        workspace: WorkspaceId,
        reason: String,
    },
    RateLimited {
        operation: String,
        workspace: WorkspaceId,
        reason: String,
    },
    ResourceExceeded {
        workspace: WorkspaceId,
        resource: String,
        quota: String,
    },
    PluginLoaded {
        plugin_id: String,
        permissions: Vec<String>,
        workspace: WorkspaceId,
    },
    PluginUnloaded {
        plugin_id: String,
        workspace: WorkspaceId,
    },
    QuotaSet {
        workspace: WorkspaceId,
        resource: String,
        limit: String,
    },
    OverrideApplied {
        workspace: WorkspaceId,
        override_type: String,
        operation: String,
        duration_secs: u64,
    },
    OverrideExpired {
        workspace: WorkspaceId,
        override_type: String,
        operation: String,
    },
    /// Hook 派发事件 —— hook registry/engine 完成一次派发后触发。
    /// `aborted = true` 表示有 hook 返回 FailedAbort（操作链被拦截）。
    HookDispatched {
        event: String,
        tool_name: Option<String>,
        workspace: Option<WorkspaceId>,
        dispatched_count: u32,
        aborted: bool,
        timestamp: DateTime<Utc>,
    },
}

impl SecurityEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            SecurityEvent::OperationStart { .. } => "operation_start",
            SecurityEvent::OperationComplete { .. } => "operation_complete",
            SecurityEvent::ApprovalRequested { .. } => "approval_requested",
            SecurityEvent::ApprovalGranted { .. } => "approval_granted",
            SecurityEvent::ApprovalRejected { .. } => "approval_rejected",
            SecurityEvent::PolicyDenied { .. } => "policy_denied",
            SecurityEvent::RateLimited { .. } => "rate_limited",
            SecurityEvent::ResourceExceeded { .. } => "resource_exceeded",
            SecurityEvent::PluginLoaded { .. } => "plugin_loaded",
            SecurityEvent::PluginUnloaded { .. } => "plugin_unloaded",
            SecurityEvent::QuotaSet { .. } => "quota_set",
            SecurityEvent::OverrideApplied { .. } => "override_applied",
            SecurityEvent::OverrideExpired { .. } => "override_expired",
            SecurityEvent::HookDispatched { .. } => "hook_dispatched",
        }
    }

    pub fn workspace(&self) -> Option<&WorkspaceId> {
        match self {
            SecurityEvent::OperationStart { workspace, .. } => Some(workspace),
            SecurityEvent::OperationComplete { workspace, .. } => Some(workspace),
            SecurityEvent::ApprovalRequested { workspace, .. } => Some(workspace),
            SecurityEvent::ApprovalGranted { workspace, .. } => Some(workspace),
            SecurityEvent::ApprovalRejected { workspace, .. } => Some(workspace),
            SecurityEvent::PolicyDenied { workspace, .. } => Some(workspace),
            SecurityEvent::RateLimited { workspace, .. } => Some(workspace),
            SecurityEvent::ResourceExceeded { workspace, .. } => Some(workspace),
            SecurityEvent::PluginLoaded { workspace, .. } => Some(workspace),
            SecurityEvent::PluginUnloaded { workspace, .. } => Some(workspace),
            SecurityEvent::QuotaSet { workspace, .. } => Some(workspace),
            SecurityEvent::OverrideApplied { workspace, .. } => Some(workspace),
            SecurityEvent::OverrideExpired { workspace, .. } => Some(workspace),
            SecurityEvent::HookDispatched { workspace, .. } => workspace.as_ref(),
        }
    }

    pub fn operation(&self) -> Option<&str> {
        match self {
            SecurityEvent::OperationStart { operation, .. } => Some(operation),
            SecurityEvent::OperationComplete { operation, .. } => Some(operation),
            SecurityEvent::ApprovalRequested { operation, .. } => Some(operation),
            SecurityEvent::PolicyDenied { operation, .. } => Some(operation),
            SecurityEvent::RateLimited { operation, .. } => Some(operation),
            SecurityEvent::OverrideApplied { operation, .. } => Some(operation),
            SecurityEvent::OverrideExpired { operation, .. } => Some(operation),
            SecurityEvent::HookDispatched { tool_name, .. } => tool_name.as_deref(),
            _ => None,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SecurityEvent::OperationStart { timestamp, .. } => *timestamp,
            SecurityEvent::OperationComplete { .. } => Utc::now(),
            SecurityEvent::ApprovalRequested { .. } => Utc::now(),
            SecurityEvent::ApprovalGranted { .. } => Utc::now(),
            SecurityEvent::ApprovalRejected { .. } => Utc::now(),
            SecurityEvent::PolicyDenied { .. } => Utc::now(),
            SecurityEvent::RateLimited { .. } => Utc::now(),
            SecurityEvent::ResourceExceeded { .. } => Utc::now(),
            SecurityEvent::PluginLoaded { .. } => Utc::now(),
            SecurityEvent::PluginUnloaded { .. } => Utc::now(),
            SecurityEvent::QuotaSet { .. } => Utc::now(),
            SecurityEvent::OverrideApplied { .. } => Utc::now(),
            SecurityEvent::OverrideExpired { .. } => Utc::now(),
            SecurityEvent::HookDispatched { timestamp, .. } => *timestamp,
        }
    }

    pub fn is_security_related(&self) -> bool {
        matches!(
            self,
            SecurityEvent::PolicyDenied { .. }
                | SecurityEvent::RateLimited { .. }
                | SecurityEvent::ResourceExceeded { .. }
                | SecurityEvent::ApprovalRequested { .. }
                | SecurityEvent::ApprovalRejected { .. }
                | SecurityEvent::HookDispatched { aborted: true, .. }
        )
    }

    pub fn is_denied(&self) -> bool {
        matches!(
            self,
            SecurityEvent::PolicyDenied { .. }
                | SecurityEvent::RateLimited { .. }
                | SecurityEvent::ResourceExceeded { .. }
                | SecurityEvent::ApprovalRejected { .. }
                | SecurityEvent::HookDispatched { aborted: true, .. }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub event: SecurityEvent,
    pub recorded_at: DateTime<Utc>,
}

impl AuditEntry {
    pub fn new(event: SecurityEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            event,
            recorded_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub workspace: Option<WorkspaceId>,
    pub event_type: Option<String>,
    pub operation: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub only_security: bool,
    pub only_denied: bool,
}

impl AuditFilter {
    pub fn for_workspace(workspace: WorkspaceId) -> Self {
        Self {
            workspace: Some(workspace),
            ..Default::default()
        }
    }

    pub fn for_operation(operation: String) -> Self {
        Self {
            operation: Some(operation),
            ..Default::default()
        }
    }

    pub fn security_events() -> Self {
        Self {
            only_security: true,
            ..Default::default()
        }
    }

    pub fn denied_events() -> Self {
        Self {
            only_denied: true,
            ..Default::default()
        }
    }

    pub fn in_time_range(since: DateTime<Utc>, until: DateTime<Utc>) -> Self {
        Self {
            since: Some(since),
            until: Some(until),
            ..Default::default()
        }
    }

    pub fn matches(&self, entry: &AuditEntry) -> bool {
        if let Some(ws) = &self.workspace {
            if entry.event.workspace() != Some(ws) {
                return false;
            }
        }

        if let Some(et) = &self.event_type {
            if entry.event.event_type() != et.as_str() {
                return false;
            }
        }

        if let Some(op) = &self.operation {
            if entry.event.operation() != Some(op.as_str()) {
                return false;
            }
        }

        if let Some(since) = &self.since {
            if entry.recorded_at < *since {
                return false;
            }
        }

        if let Some(until) = &self.until {
            if entry.recorded_at > *until {
                return false;
            }
        }

        if self.only_security && !entry.event.is_security_related() {
            return false;
        }

        if self.only_denied && !entry.event.is_denied() {
            return false;
        }

        true
    }
}