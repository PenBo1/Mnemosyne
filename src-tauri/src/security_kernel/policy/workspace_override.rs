use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::security_kernel::permission::Operation;
use crate::security_kernel::WorkspaceId;

use super::decision::PolicyDecision;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOverride {
    pub workspace_id: WorkspaceId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub overrides: HashMap<String, OperationOverride>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationOverride {
    pub operation_pattern: String,
    pub decision: OverrideDecision,
    pub conditions: Option<OverrideConditions>,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideDecision {
    AlwaysAllow,
    AlwaysDeny,
    AskOnce,
    AskAlways,
}

impl OverrideDecision {
    pub fn to_policy_decision(&self) -> PolicyDecision {
        match self {
            Self::AlwaysAllow => PolicyDecision::Allow,
            Self::AlwaysDeny => PolicyDecision::Deny,
            Self::AskOnce | Self::AskAlways => PolicyDecision::RequireApproval,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideConditions {
    pub max_count: Option<u32>,
    pub max_size_mb: Option<u32>,
    pub time_window: Option<TimeWindow>,
    pub path_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_hour: u8,
    pub end_hour: u8,
    pub weekdays_only: bool,
}

impl WorkspaceOverride {
    pub fn new(workspace_id: WorkspaceId) -> Self {
        let now = Utc::now();
        Self {
            workspace_id,
            created_at: now,
            updated_at: now,
            overrides: HashMap::new(),
            enabled: true,
        }
    }

    pub fn add_override(mut self, pattern: String, decision: OverrideDecision, reason: String) -> Self {
        let override_entry = OperationOverride {
            operation_pattern: pattern.clone(),
            decision,
            conditions: None,
            expires_at: None,
            reason,
        };
        self.overrides.insert(pattern, override_entry);
        self.updated_at = Utc::now();
        self
    }

    pub fn add_override_with_conditions(
        mut self,
        pattern: String,
        decision: OverrideDecision,
        conditions: OverrideConditions,
        reason: String,
    ) -> Self {
        let override_entry = OperationOverride {
            operation_pattern: pattern.clone(),
            decision,
            conditions: Some(conditions),
            expires_at: None,
            reason,
        };
        self.overrides.insert(pattern, override_entry);
        self.updated_at = Utc::now();
        self
    }

    pub fn add_temporary_override(
        mut self,
        pattern: String,
        decision: OverrideDecision,
        expires_at: DateTime<Utc>,
        reason: String,
    ) -> Self {
        let override_entry = OperationOverride {
            operation_pattern: pattern.clone(),
            decision,
            conditions: None,
            expires_at: Some(expires_at),
            reason,
        };
        self.overrides.insert(pattern, override_entry);
        self.updated_at = Utc::now();
        self
    }

    pub fn remove_override(mut self, pattern: &str) -> Self {
        self.overrides.remove(pattern);
        self.updated_at = Utc::now();
        self
    }

    pub fn get_override(&self, op: &Operation) -> Option<&OperationOverride> {
        if !self.enabled {
            return None;
        }

        let op_key = operation_key(op);
        self.overrides.get(&op_key)
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.overrides.retain(|_, override_entry| {
            override_entry.expires_at.is_none_or(|expires| expires > now)
        });
        self.updated_at = now;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self.updated_at = Utc::now();
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self.updated_at = Utc::now();
        self
    }
}

fn operation_key(op: &Operation) -> String {
    match op {
        Operation::Filesystem { scope, operation, path } => {
            format!("fs:{}:{}:{}", operation, scope, path)
        }
        Operation::Shell { scope, command, args } => {
            format!("shell:{}:{}:{}", scope, command, args.join(","))
        }
        Operation::Network { scope, endpoint, method } => {
            format!("network:{}:{}:{}", method, endpoint, scope)
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceOverrideRegistry {
    overrides: HashMap<WorkspaceId, WorkspaceOverride>,
}

impl WorkspaceOverrideRegistry {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    pub fn register(&mut self, override_entry: WorkspaceOverride) {
        self.overrides.insert(override_entry.workspace_id, override_entry);
    }

    pub fn get(&self, workspace_id: &WorkspaceId) -> Option<&WorkspaceOverride> {
        self.overrides.get(workspace_id)
    }

    pub fn get_mut(&mut self, workspace_id: &WorkspaceId) -> Option<&mut WorkspaceOverride> {
        self.overrides.get_mut(workspace_id)
    }

    pub fn remove(&mut self, workspace_id: &WorkspaceId) -> Option<WorkspaceOverride> {
        self.overrides.remove(workspace_id)
    }

    pub fn cleanup_all_expired(&mut self) {
        for override_entry in self.overrides.values_mut() {
            override_entry.cleanup_expired();
        }
    }

    pub fn count(&self) -> usize {
        self.overrides.len()
    }

    pub fn list(&self) -> Vec<&WorkspaceOverride> {
        self.overrides.values().collect()
    }
}

impl Default for WorkspaceOverrideRegistry {
    fn default() -> Self {
        Self::new()
    }
}