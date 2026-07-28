//! ═══════════════════════════════════════════════════════════════════════════
//! limiter - 速率限制核心实现模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;

use chrono::Duration;

use crate::shared::error::AppError;

use super::super::WorkspaceId;
use super::policy::{RatePolicy, DefaultPolicies};
use super::store::RateStore;

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub operation: String,
    pub current_minute: u32,
    pub current_hour: u32,
    pub current_day: u32,
    pub limit_minute: u32,
    pub limit_hour: u32,
    pub limit_day: u32,
}

impl RateLimitResult {
    pub fn exceeded_window(&self) -> Option<&'static str> {
        if self.current_minute > self.limit_minute {
            Some("minute")
        } else if self.current_hour > self.limit_hour {
            Some("hour")
        } else if self.current_day > self.limit_day {
            Some("day")
        } else {
            None
        }
    }
}

pub struct RateLimiter {
    policies: HashMap<String, RatePolicy>,
    store: RateStore,
}

impl RateLimiter {
    pub fn new() -> Self {
        let mut policies = HashMap::new();
        
        for policy in DefaultPolicies::all() {
            policies.insert(policy.operation.clone(), policy);
        }

        Self {
            policies,
            store: RateStore::new(),
        }
    }

    pub fn with_policies(policies: Vec<RatePolicy>) -> Self {
        let mut map = HashMap::new();
        
        for policy in policies {
            map.insert(policy.operation.clone(), policy);
        }

        Self {
            policies: map,
            store: RateStore::new(),
        }
    }

    pub fn add_policy(&mut self, policy: RatePolicy) {
        self.policies.insert(policy.operation.clone(), policy);
    }

    pub fn remove_policy(&mut self, operation: &str) -> Option<RatePolicy> {
        self.policies.remove(operation)
    }

    pub fn get_policy(&self, operation: &str) -> Option<&RatePolicy> {
        self.policies.get(operation)
    }

    pub fn check(&self, operation: &str, workspace: WorkspaceId) -> Result<RateLimitResult, AppError> {
        let policy = self.policies.get(operation);
        
        if policy.is_none() {
            tracing::warn!(
                operation = operation,
                workspace = %workspace.0,
                decision = "Allow",
                reason = "No policy defined",
                "rate_limiter: operation allowed (no policy)"
            );
            return Ok(RateLimitResult {
                allowed: true,
                operation: operation.to_string(),
                current_minute: 0,
                current_hour: 0,
                current_day: 0,
                limit_minute: u32::MAX,
                limit_hour: u32::MAX,
                limit_day: u32::MAX,
            });
        }

        let policy = policy.unwrap();
        
        let current_minute = self.store.count_in_window(workspace, operation, Duration::minutes(1));
        let current_hour = self.store.count_in_window(workspace, operation, Duration::hours(1));
        let current_day = self.store.count_in_window(workspace, operation, Duration::days(1));

        let allowed = current_minute <= policy.max_per_minute
            && current_hour <= policy.max_per_hour
            && current_day <= policy.max_per_day;

        if allowed {
            tracing::warn!(
                operation = operation,
                workspace = %workspace.0,
                decision = "Allow",
                current_minute = current_minute,
                current_hour = current_hour,
                current_day = current_day,
                limit_minute = policy.max_per_minute,
                limit_hour = policy.max_per_hour,
                limit_day = policy.max_per_day,
                "rate_limiter: operation allowed"
            );
        } else {
            tracing::error!(
                operation = operation,
                workspace = %workspace.0,
                decision = "Deny",
                reason = "Rate limit exceeded",
                current_minute = current_minute,
                current_hour = current_hour,
                current_day = current_day,
                limit_minute = policy.max_per_minute,
                limit_hour = policy.max_per_hour,
                limit_day = policy.max_per_day,
                "rate_limiter: operation denied"
            );
        }

        Ok(RateLimitResult {
            allowed,
            operation: operation.to_string(),
            current_minute,
            current_hour,
            current_day,
            limit_minute: policy.max_per_minute,
            limit_hour: policy.max_per_hour,
            limit_day: policy.max_per_day,
        })
    }

    pub fn check_and_fail(&self, operation: &str, workspace: WorkspaceId) -> Result<(), AppError> {
        let result = self.check(operation, workspace)?;
        
        if !result.allowed {
            let window = result.exceeded_window().unwrap_or("unknown");
            return Err(AppError::rate_limit_exceeded(
                operation,
                window,
                result.current_minute,
                result.limit_minute,
            ));
        }

        Ok(())
    }

    pub fn record(&self, operation: &str, workspace: WorkspaceId) {
        tracing::warn!(
            operation = operation,
            workspace = %workspace.0,
            decision = "Record",
            "rate_limiter: operation recorded"
        );
        self.store.add_record(workspace, operation, chrono::Utc::now());
    }

    pub fn check_and_record(&self, operation: &str, workspace: WorkspaceId) -> Result<(), AppError> {
        self.check_and_fail(operation, workspace)?;
        self.record(operation, workspace);
        Ok(())
    }

    pub fn cleanup_expired(&self) {
        self.store.cleanup_expired(Duration::days(1));
    }

    pub fn clear_workspace(&self, workspace: WorkspaceId) {
        self.store.clear_workspace(workspace);
    }

    pub fn clear_all(&self) {
        self.store.clear_all();
    }

    pub fn policies_count(&self) -> usize {
        self.policies.len()
    }

    pub fn total_records(&self) -> usize {
        self.store.total_records()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_workspace() -> WorkspaceId {
        WorkspaceId(Uuid::new_v4())
    }

    #[test]
    fn test_rate_policy_creation() {
        let policy = RatePolicy::new("ReadFile", 100, 1000, 10000);
        assert_eq!(policy.operation, "ReadFile");
        assert_eq!(policy.max_per_minute, 100);
        assert_eq!(policy.max_per_hour, 1000);
        assert_eq!(policy.max_per_day, 10000);
    }

    #[test]
    fn test_rate_policy_unlimited() {
        let policy = RatePolicy::unlimited("Default");
        assert_eq!(policy.max_per_minute, u32::MAX);
        assert_eq!(policy.max_per_hour, u32::MAX);
        assert_eq!(policy.max_per_day, u32::MAX);
    }

    #[test]
    fn test_default_policies_read_file() {
        let policy = DefaultPolicies::read_file();
        assert_eq!(policy.operation, "ReadFile");
        assert_eq!(policy.max_per_minute, 10000);
        assert_eq!(policy.max_per_hour, 600_000);
        assert_eq!(policy.max_per_day, 10_000_000);
    }

    #[test]
    fn test_rate_limiter_check_allowed() {
        let limiter = RateLimiter::new();
        let workspace = make_workspace();

        let result = limiter.check("ReadFile", workspace).unwrap();
        assert!(result.allowed);
        assert_eq!(result.operation, "ReadFile");
    }

    #[test]
    fn test_rate_limiter_check_unknown_operation() {
        let limiter = RateLimiter::new();
        let workspace = make_workspace();

        let result = limiter.check("UnknownOp", workspace).unwrap();
        assert!(result.allowed);
        assert_eq!(result.limit_minute, u32::MAX);
    }

    #[test]
    fn test_rate_limiter_record_and_check() {
        let limiter = RateLimiter::with_policies(vec![
            RatePolicy::new("TestOp", 3, 100, 1000),
        ]);
        let workspace = make_workspace();

        limiter.record("TestOp", workspace);
        limiter.record("TestOp", workspace);
        limiter.record("TestOp", workspace);

        let result = limiter.check("TestOp", workspace).unwrap();
        assert_eq!(result.current_minute, 3);
        assert!(result.allowed);

        limiter.record("TestOp", workspace);

        let result_after = limiter.check("TestOp", workspace).unwrap();
        assert_eq!(result_after.current_minute, 4);
        assert!(!result_after.allowed);
        assert_eq!(result_after.exceeded_window(), Some("minute"));
    }

    #[test]
    fn test_rate_limiter_check_and_fail() {
        let limiter = RateLimiter::with_policies(vec![
            RatePolicy::new("LimitedOp", 2, 100, 1000),
        ]);
        let workspace = make_workspace();

        limiter.check_and_record("LimitedOp", workspace).unwrap();
        limiter.check_and_record("LimitedOp", workspace).unwrap();

        let result = limiter.check_and_fail("LimitedOp", workspace);
        assert!(result.is_ok());

        limiter.record("LimitedOp", workspace);

        let result_fail = limiter.check_and_fail("LimitedOp", workspace);
        assert!(result_fail.is_err());
    }

    #[test]
    fn test_rate_limit_result_exceeded_window() {
        let result_minute = RateLimitResult {
            allowed: false,
            operation: "Test".to_string(),
            current_minute: 100,
            current_hour: 50,
            current_day: 10,
            limit_minute: 50,
            limit_hour: 100,
            limit_day: 1000,
        };
        assert_eq!(result_minute.exceeded_window(), Some("minute"));

        let result_hour = RateLimitResult {
            allowed: false,
            operation: "Test".to_string(),
            current_minute: 50,
            current_hour: 200,
            current_day: 10,
            limit_minute: 100,
            limit_hour: 100,
            limit_day: 1000,
        };
        assert_eq!(result_hour.exceeded_window(), Some("hour"));

        let result_day = RateLimitResult {
            allowed: false,
            operation: "Test".to_string(),
            current_minute: 50,
            current_hour: 50,
            current_day: 2000,
            limit_minute: 100,
            limit_hour: 100,
            limit_day: 1000,
        };
        assert_eq!(result_day.exceeded_window(), Some("day"));

        let result_ok = RateLimitResult {
            allowed: true,
            operation: "Test".to_string(),
            current_minute: 50,
            current_hour: 50,
            current_day: 10,
            limit_minute: 100,
            limit_hour: 100,
            limit_day: 1000,
        };
        assert_eq!(result_ok.exceeded_window(), None);
    }

    #[test]
    fn test_rate_limiter_clear_workspace() {
        let limiter = RateLimiter::with_policies(vec![
            RatePolicy::new("TestOp", 3, 100, 1000),
        ]);
        let workspace = make_workspace();

        limiter.record("TestOp", workspace);
        limiter.record("TestOp", workspace);

        let result_before = limiter.check("TestOp", workspace).unwrap();
        assert_eq!(result_before.current_minute, 2);

        limiter.clear_workspace(workspace);

        let result_after = limiter.check("TestOp", workspace).unwrap();
        assert_eq!(result_after.current_minute, 0);
    }

    #[test]
    fn test_rate_limiter_policies_count() {
        let limiter = RateLimiter::new();
        assert!(limiter.policies_count() >= 3);

        let custom_limiter = RateLimiter::with_policies(vec![
            RatePolicy::new("Op1", 10, 100, 1000),
            RatePolicy::new("Op2", 20, 200, 2000),
        ]);
        assert_eq!(custom_limiter.policies_count(), 2);
    }
}