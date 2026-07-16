use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::security_kernel::permission::Operation;
use crate::security_kernel::SessionId;

use super::workspace_override::OverrideDecision;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryOverride {
    pub session_id: SessionId,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub overrides: HashMap<String, TemporaryOperationOverride>,
    pub approval_count: HashMap<String, u32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryOperationOverride {
    pub operation_pattern: String,
    pub decision: OverrideDecision,
    pub approved_at: DateTime<Utc>,
    pub approved_by: Option<String>,
    pub remaining_count: Option<u32>,
    pub reason: String,
}

impl TemporaryOverride {
    pub fn new(session_id: SessionId, duration_minutes: u32) -> Self {
        let now = Utc::now();
        let expires = now + chrono::Duration::minutes(duration_minutes as i64);
        Self {
            session_id,
            created_at: now,
            expires_at: expires,
            overrides: HashMap::new(),
            approval_count: HashMap::new(),
            enabled: true,
        }
    }

    pub fn with_duration(session_id: SessionId, expires_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            created_at: now,
            expires_at,
            overrides: HashMap::new(),
            approval_count: HashMap::new(),
            enabled: true,
        }
    }

    pub fn approve_operation(
        mut self,
        op: &Operation,
        decision: OverrideDecision,
        approved_by: Option<String>,
        max_count: Option<u32>,
        reason: String,
    ) -> Self {
        let op_key = operation_key(op);
        let now = Utc::now();

        let override_entry = TemporaryOperationOverride {
            operation_pattern: op_key.clone(),
            decision,
            approved_at: now,
            approved_by,
            remaining_count: max_count,
            reason,
        };

        self.overrides.insert(op_key.clone(), override_entry);
        self.approval_count.insert(op_key, 0);

        self
    }

    pub fn approve_once(self, op: &Operation, approved_by: Option<String>, reason: String) -> Self {
        self.approve_operation(op, OverrideDecision::AskOnce, approved_by, Some(1), reason)
    }

    pub fn approve_always(self, op: &Operation, approved_by: Option<String>, reason: String) -> Self {
        self.approve_operation(op, OverrideDecision::AskAlways, approved_by, None, reason)
    }

    pub fn deny_operation(self, op: &Operation, reason: String) -> Self {
        self.approve_operation(op, OverrideDecision::AlwaysDeny, None, None, reason)
    }

    pub fn get_override(&self, op: &Operation) -> Option<&TemporaryOperationOverride> {
        if !self.enabled || self.is_expired() {
            return None;
        }

        let op_key = operation_key(op);
        self.overrides.get(&op_key)
    }

    pub fn consume_approval(&mut self, op: &Operation) -> bool {
        // High 12: 再次检查 is_expired(),覆盖 evaluate → executor → consume 之间的时间窗口。
        // C12 修复后 consume 在 executor 成功后调用,executor 耗时可能导致 override 过期,
        // 此处返回 false（不消费）,操作已执行无法回滚,但下次调用会被拒绝。
        if !self.enabled || self.is_expired() {
            return false;
        }

        let op_key = operation_key(op);

        if let Some(override_entry) = self.overrides.get_mut(&op_key) {
            if let Some(remaining) = override_entry.remaining_count {
                if remaining > 0 {
                    override_entry.remaining_count = Some(remaining - 1);
                    self.approval_count.entry(op_key.clone()).or_insert(0);
                    if let Some(count) = self.approval_count.get_mut(&op_key) {
                        *count += 1;
                    }
                    return true;
                }
                return false;
            }
            return true;
        }

        false
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn remaining_time(&self) -> chrono::Duration {
        self.expires_at - Utc::now()
    }

    pub fn remaining_seconds(&self) -> i64 {
        self.remaining_time().num_seconds()
    }

    pub fn cleanup_expired(&mut self) {
        if self.is_expired() {
            self.overrides.clear();
            self.approval_count.clear();
            self.enabled = false;
        }
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn extend_duration(mut self, additional_minutes: u32) -> Self {
        self.expires_at += chrono::Duration::minutes(additional_minutes as i64);
        self
    }

    pub fn total_approvals(&self) -> u32 {
        self.approval_count.values().sum()
    }

    pub fn operation_count(&self) -> usize {
        self.overrides.len()
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
pub struct TemporaryOverrideRegistry {
    overrides: HashMap<SessionId, TemporaryOverride>,
    default_duration_minutes: u32,
}

impl TemporaryOverrideRegistry {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
            default_duration_minutes: 30,
        }
    }

    pub fn with_default_duration(default_duration_minutes: u32) -> Self {
        Self {
            overrides: HashMap::new(),
            default_duration_minutes,
        }
    }

    pub fn create(&mut self, session_id: SessionId) -> TemporaryOverride {
        TemporaryOverride::new(session_id, self.default_duration_minutes)
    }

    pub fn create_with_duration(&mut self, session_id: SessionId, duration_minutes: u32) -> TemporaryOverride {
        TemporaryOverride::new(session_id, duration_minutes)
    }

    pub fn register(&mut self, override_entry: TemporaryOverride) {
        self.overrides.insert(override_entry.session_id, override_entry);
    }

    pub fn get(&self, session_id: &SessionId) -> Option<&TemporaryOverride> {
        self.overrides.get(session_id)
    }

    pub fn get_mut(&mut self, session_id: &SessionId) -> Option<&mut TemporaryOverride> {
        self.overrides.get_mut(session_id)
    }

    pub fn remove(&mut self, session_id: &SessionId) -> Option<TemporaryOverride> {
        self.overrides.remove(session_id)
    }

    pub fn cleanup_expired(&mut self) {
        self.overrides.retain(|_, override_entry| {
            !override_entry.is_expired()
        });
    }

    pub fn count(&self) -> usize {
        self.overrides.len()
    }

    pub fn active_count(&self) -> usize {
        self.overrides.values().filter(|o| !o.is_expired() && o.enabled).count()
    }

    pub fn list(&self) -> Vec<&TemporaryOverride> {
        self.overrides.values().collect()
    }

    pub fn list_active(&self) -> Vec<&TemporaryOverride> {
        self.overrides.values().filter(|o| !o.is_expired() && o.enabled).collect()
    }
}

impl Default for TemporaryOverrideRegistry {
    fn default() -> Self {
        Self::new()
    }
}