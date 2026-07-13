use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::security_kernel::permission::Operation;
use crate::security_kernel::UserId;

use super::workspace_override::{OverrideConditions, OverrideDecision, OperationOverride};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOverride {
    pub user_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub global_overrides: HashMap<String, OperationOverride>,
    pub workspace_overrides: HashMap<String, HashMap<String, OperationOverride>>,
    pub enabled: bool,
}

impl UserOverride {
    pub fn new(user_id: UserId) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            created_at: now,
            updated_at: now,
            global_overrides: HashMap::new(),
            workspace_overrides: HashMap::new(),
            enabled: true,
        }
    }

    pub fn add_global_override(
        mut self,
        pattern: String,
        decision: OverrideDecision,
        reason: String,
    ) -> Self {
        let override_entry = OperationOverride {
            operation_pattern: pattern.clone(),
            decision,
            conditions: None,
            expires_at: None,
            reason,
        };
        self.global_overrides.insert(pattern, override_entry);
        self.updated_at = Utc::now();
        self
    }

    pub fn add_global_override_with_conditions(
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
        self.global_overrides.insert(pattern, override_entry);
        self.updated_at = Utc::now();
        self
    }

    pub fn add_workspace_override(
        mut self,
        workspace_pattern: String,
        operation_pattern: String,
        decision: OverrideDecision,
        reason: String,
    ) -> Self {
        let override_entry = OperationOverride {
            operation_pattern: operation_pattern.clone(),
            decision,
            conditions: None,
            expires_at: None,
            reason,
        };

        self.workspace_overrides
            .entry(workspace_pattern)
            .or_insert_with(HashMap::new)
            .insert(operation_pattern, override_entry);

        self.updated_at = Utc::now();
        self
    }

    pub fn remove_global_override(mut self, pattern: &str) -> Self {
        self.global_overrides.remove(pattern);
        self.updated_at = Utc::now();
        self
    }

    pub fn remove_workspace_override(mut self, workspace_pattern: &str, operation_pattern: &str) -> Self {
        if let Some(workspace_map) = self.workspace_overrides.get_mut(workspace_pattern) {
            workspace_map.remove(operation_pattern);
            if workspace_map.is_empty() {
                self.workspace_overrides.remove(workspace_pattern);
            }
        }
        self.updated_at = Utc::now();
        self
    }

    pub fn get_global_override(&self, op: &Operation) -> Option<&OperationOverride> {
        if !self.enabled {
            return None;
        }

        let op_key = operation_pattern(op);
        self.global_overrides.get(&op_key)
    }

    pub fn get_workspace_override(
        &self,
        workspace_id: &crate::security_kernel::WorkspaceId,
        op: &Operation,
    ) -> Option<&OperationOverride> {
        if !self.enabled {
            return None;
        }

        let workspace_key = workspace_id.0.to_string();
        let op_key = operation_pattern(op);

        self.workspace_overrides
            .get(&workspace_key)
            .and_then(|map| map.get(&op_key))
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();

        self.global_overrides.retain(|_, override_entry| {
            override_entry.expires_at.map_or(true, |expires| expires > now)
        });

        for workspace_map in self.workspace_overrides.values_mut() {
            workspace_map.retain(|_, override_entry| {
                override_entry.expires_at.map_or(true, |expires| expires > now)
            });
        }

        self.workspace_overrides.retain(|_, map| !map.is_empty());
        self.updated_at = now;
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self.updated_at = Utc::now();
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self.updated_at = Utc::now();
        self
    }
}

fn operation_pattern(op: &Operation) -> String {
    match op {
        Operation::Filesystem { scope, operation, .. } => {
            format!("fs:{}:{}:*", operation, scope)
        }
        Operation::Shell { scope, command, .. } => {
            format!("shell:{}:{}:*", scope, command)
        }
        Operation::Network { method, endpoint, .. } => {
            format!("network:{}:{}:*", method, endpoint)
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserOverrideRegistry {
    overrides: HashMap<UserId, UserOverride>,
}

impl UserOverrideRegistry {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    pub fn register(&mut self, override_entry: UserOverride) {
        self.overrides.insert(override_entry.user_id, override_entry);
    }

    pub fn get(&self, user_id: &UserId) -> Option<&UserOverride> {
        self.overrides.get(user_id)
    }

    pub fn get_mut(&mut self, user_id: &UserId) -> Option<&mut UserOverride> {
        self.overrides.get_mut(user_id)
    }

    pub fn remove(&mut self, user_id: &UserId) -> Option<UserOverride> {
        self.overrides.remove(user_id)
    }

    pub fn cleanup_all_expired(&mut self) {
        for override_entry in self.overrides.values_mut() {
            override_entry.cleanup_expired();
        }
    }

    pub fn count(&self) -> usize {
        self.overrides.len()
    }

    pub fn list(&self) -> Vec<&UserOverride> {
        self.overrides.values().collect()
    }
}

impl Default for UserOverrideRegistry {
    fn default() -> Self {
        Self::new()
    }
}