use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::security_kernel::permission::Operation;
use crate::security_kernel::types::OperationContext;
use crate::security_kernel::types::TrustLevel;
use crate::security_kernel::{SessionId, UserId, WorkspaceId};
use crate::shared::error::AppError;

use super::decision::{
    calculate_operation_risk, OperationRisk, PolicyDecision, PolicyEvaluation, PolicySource,
};
use super::global_policy::GlobalPolicy;
use super::temporary_override::{TemporaryOverride, TemporaryOverrideRegistry};
use super::user_override::{UserOverride, UserOverrideRegistry};
use super::workspace_override::{WorkspaceOverride, WorkspaceOverrideRegistry};

#[derive(Debug)]
pub struct PolicyEngine {
    global_policy: GlobalPolicy,
    workspace_registry: Arc<RwLock<WorkspaceOverrideRegistry>>,
    user_registry: Arc<RwLock<UserOverrideRegistry>>,
    temporary_registry: Arc<RwLock<TemporaryOverrideRegistry>>,
    workspace_trust_levels: HashMap<WorkspaceId, TrustLevel>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        // 注册系统工作区（nil UUID）为 Trusted —— 系统级操作（如 ai_http_stream）
        // 使用 nil UUID 作为 workspace_id，需要 Trusted 级别才能允许网络请求
        let mut trust_levels = HashMap::new();
        trust_levels.insert(WorkspaceId(uuid::Uuid::nil()), TrustLevel::Trusted);
        Self {
            global_policy: GlobalPolicy::default(),
            workspace_registry: Arc::new(RwLock::new(WorkspaceOverrideRegistry::new())),
            user_registry: Arc::new(RwLock::new(UserOverrideRegistry::new())),
            temporary_registry: Arc::new(RwLock::new(TemporaryOverrideRegistry::new())),
            workspace_trust_levels: trust_levels,
        }
    }

    pub fn with_global_policy(global_policy: GlobalPolicy) -> Self {
        let mut trust_levels = HashMap::new();
        trust_levels.insert(WorkspaceId(uuid::Uuid::nil()), TrustLevel::Trusted);
        Self {
            global_policy,
            workspace_registry: Arc::new(RwLock::new(WorkspaceOverrideRegistry::new())),
            user_registry: Arc::new(RwLock::new(UserOverrideRegistry::new())),
            temporary_registry: Arc::new(RwLock::new(TemporaryOverrideRegistry::new())),
            workspace_trust_levels: trust_levels,
        }
    }

    pub fn register_workspace_trust(&mut self, workspace_id: WorkspaceId, trust_level: TrustLevel) {
        self.workspace_trust_levels.insert(workspace_id, trust_level);
    }

    pub fn register_workspace_override(&self, override_entry: WorkspaceOverride) {
        let mut registry = self.workspace_registry.write().unwrap();
        registry.register(override_entry);
    }

    pub fn register_user_override(&self, override_entry: UserOverride) {
        let mut registry = self.user_registry.write().unwrap();
        registry.register(override_entry);
    }

    pub fn register_temporary_override(&self, override_entry: TemporaryOverride) {
        let mut registry = self.temporary_registry.write().unwrap();
        registry.register(override_entry);
    }

    pub fn remove_workspace_override(&self, workspace_id: &WorkspaceId) -> Option<WorkspaceOverride> {
        let mut registry = self.workspace_registry.write().unwrap();
        registry.remove(workspace_id)
    }

    pub fn remove_user_override(&self, user_id: &UserId) -> Option<UserOverride> {
        let mut registry = self.user_registry.write().unwrap();
        registry.remove(user_id)
    }

    pub fn remove_temporary_override(&self, session_id: &SessionId) -> Option<TemporaryOverride> {
        let mut registry = self.temporary_registry.write().unwrap();
        registry.remove(session_id)
    }

    pub fn cleanup_expired(&self) {
        let mut workspace_registry = self.workspace_registry.write().unwrap();
        workspace_registry.cleanup_all_expired();

        let mut user_registry = self.user_registry.write().unwrap();
        user_registry.cleanup_all_expired();

        let mut temp_registry = self.temporary_registry.write().unwrap();
        temp_registry.cleanup_expired();
    }

    pub fn evaluate(&self, op: &Operation, ctx: &OperationContext) -> Result<PolicyEvaluation, AppError> {
        let risk = calculate_operation_risk(op);

        if let Some(evaluation) = self.check_temporary_override(op, ctx, risk) {
            return Ok(evaluation);
        }

        if let Some(evaluation) = self.check_user_override(op, ctx, risk) {
            return Ok(evaluation);
        }

        if let Some(evaluation) = self.check_workspace_override(op, ctx, risk) {
            return Ok(evaluation);
        }

        let evaluation = self.apply_global_policy(op, ctx, risk);
        Ok(evaluation)
    }

    fn check_temporary_override(
        &self,
        op: &Operation,
        ctx: &OperationContext,
        risk: OperationRisk,
    ) -> Option<PolicyEvaluation> {
        let registry = self.temporary_registry.read().unwrap();
        let temp_override = registry.get(&ctx.session)?;

        let operation_override = temp_override.get_override(op)?;
        let decision = operation_override.decision.to_policy_decision();
        let remaining = operation_override.remaining_count;

        if remaining.map_or(true, |r| r > 0) {
            Some(PolicyEvaluation {
                decision,
                risk,
                source: PolicySource::TemporaryOverride,
                reason: format!(
                    "Temporary override: {} (approved by {:?}, remaining: {:?})",
                    operation_override.reason,
                    operation_override.approved_by,
                    remaining
                ),
            })
        } else {
            Some(PolicyEvaluation {
                decision: PolicyDecision::Deny,
                risk,
                source: PolicySource::TemporaryOverride,
                reason: "Temporary override approval count exhausted".to_string(),
            })
        }
    }

    fn check_user_override(
        &self,
        op: &Operation,
        ctx: &OperationContext,
        risk: OperationRisk,
    ) -> Option<PolicyEvaluation> {
        let registry = self.user_registry.read().unwrap();
        let user_override = registry.get(&ctx.user)?;

        let operation_override = user_override.get_global_override(op)
            .or_else(|| user_override.get_workspace_override(&ctx.workspace, op))?;

        let decision = operation_override.decision.to_policy_decision();

        if let Some(expires) = operation_override.expires_at {
            if chrono::Utc::now() > expires {
                return None;
            }
        }

        Some(PolicyEvaluation {
            decision,
            risk,
            source: PolicySource::UserOverride,
            reason: format!("User override: {}", operation_override.reason),
        })
    }

    fn check_workspace_override(
        &self,
        op: &Operation,
        ctx: &OperationContext,
        risk: OperationRisk,
    ) -> Option<PolicyEvaluation> {
        let registry = self.workspace_registry.read().unwrap();
        let workspace_override = registry.get(&ctx.workspace)?;

        let operation_override = workspace_override.get_override(op)?;

        if let Some(expires) = operation_override.expires_at {
            if chrono::Utc::now() > expires {
                return None;
            }
        }

        let decision = operation_override.decision.to_policy_decision();

        Some(PolicyEvaluation {
            decision,
            risk,
            source: PolicySource::WorkspaceOverride,
            reason: format!("Workspace override: {}", operation_override.reason),
        })
    }

    fn apply_global_policy(
        &self,
        op: &Operation,
        ctx: &OperationContext,
        risk: OperationRisk,
    ) -> PolicyEvaluation {
        if self.global_policy.is_blocked(op) {
            return PolicyEvaluation {
                decision: PolicyDecision::Deny,
                risk,
                source: PolicySource::GlobalPolicy,
                reason: "Operation blocked by global policy".to_string(),
            };
        }

        if self.global_policy.requires_approval(op) {
            return PolicyEvaluation {
                decision: PolicyDecision::RequireApproval,
                risk,
                source: PolicySource::GlobalPolicy,
                reason: "Operation requires approval by global policy".to_string(),
            };
        }

        let trust_level = self.workspace_trust_levels
            .get(&ctx.workspace)
            .copied()
            .unwrap_or(TrustLevel::Unknown);

        use crate::security_kernel::permission::Operation as PermOp;
        match op {
            PermOp::Filesystem { scope, operation, .. } => {
                let decision = self.global_policy.evaluate_fs_operation(scope, operation, trust_level);
                PolicyEvaluation {
                    decision,
                    risk,
                    source: PolicySource::GlobalPolicy,
                    reason: format!(
                        "Filesystem operation {} on {} for trust level {}",
                        operation, scope, trust_level
                    ),
                }
            }
            PermOp::Shell { .. } => {
                let workspace_policy = self.global_policy.get_workspace_policy(trust_level);
                let decision = if workspace_policy.allow_shell {
                    self.global_policy.decide_by_risk(risk)
                } else {
                    PolicyDecision::Deny
                };
                PolicyEvaluation {
                    decision,
                    risk,
                    source: PolicySource::GlobalPolicy,
                    reason: format!("Shell operation for trust level {}", trust_level),
                }
            }
            PermOp::Network { .. } => {
                let workspace_policy = self.global_policy.get_workspace_policy(trust_level);
                // Trusted 工作区的网络操作直接 Allow（LLM API 调用是核心功能，不应触发 approval）
                // 其他信任级别仍按 risk 评估
                let decision = if !workspace_policy.allow_network {
                    PolicyDecision::Deny
                } else if trust_level == TrustLevel::Trusted {
                    PolicyDecision::Allow
                } else {
                    self.global_policy.decide_by_risk(risk)
                };
                PolicyEvaluation {
                    decision,
                    risk,
                    source: PolicySource::GlobalPolicy,
                    reason: format!("Network operation for trust level {}", trust_level),
                }
            }
        }
    }

    pub fn is_allowed(&self, op: &Operation, ctx: &OperationContext) -> Result<bool, AppError> {
        let evaluation = self.evaluate(op, ctx)?;
        Ok(evaluation.decision == PolicyDecision::Allow)
    }

    pub fn requires_approval(&self, op: &Operation, ctx: &OperationContext) -> Result<bool, AppError> {
        let evaluation = self.evaluate(op, ctx)?;
        Ok(evaluation.decision == PolicyDecision::RequireApproval)
    }

    pub fn is_denied(&self, op: &Operation, ctx: &OperationContext) -> Result<bool, AppError> {
        let evaluation = self.evaluate(op, ctx)?;
        Ok(evaluation.decision == PolicyDecision::Deny)
    }

    pub fn get_workspace_count(&self) -> usize {
        self.workspace_registry.read().unwrap().count()
    }

    pub fn get_user_count(&self) -> usize {
        self.user_registry.read().unwrap().count()
    }

    pub fn get_temporary_count(&self) -> usize {
        self.temporary_registry.read().unwrap().count()
    }

    pub fn get_active_temporary_count(&self) -> usize {
        self.temporary_registry.read().unwrap().active_count()
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_kernel::permission::{FsOperation, FsScope, Operation};
    use crate::security_kernel::policy::workspace_override::{OverrideDecision, WorkspaceOverride};
    use crate::security_kernel::policy::user_override::UserOverride;
    use crate::security_kernel::policy::temporary_override::TemporaryOverride;
    use uuid::Uuid;

    fn make_ctx() -> OperationContext {
        OperationContext {
            workspace: WorkspaceId(Uuid::new_v4()),
            user: UserId(Uuid::new_v4()),
            session: SessionId(Uuid::new_v4()),
            approval_token: None,
        }
    }

    fn make_fs_op() -> Operation {
        Operation::Filesystem {
            scope: FsScope::Workspace,
            operation: FsOperation::Write,
            path: "/test/path".to_string(),
        }
    }

    fn fs_op_key() -> String {
        "fs:write:workspace:/test/path".to_string()
    }

    fn user_override_pattern() -> String {
        "fs:write:workspace:*".to_string()
    }

    #[test]
    fn test_global_policy_default_decision() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = Operation::Filesystem {
            scope: FsScope::Workspace,
            operation: FsOperation::Read,
            path: "/test".to_string(),
        };

        let result = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result.source, PolicySource::GlobalPolicy);
    }

    #[test]
    fn test_workspace_override_overrides_global() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = make_fs_op();

        let workspace_override = WorkspaceOverride::new(ctx.workspace)
            .add_override(
                fs_op_key(),
                OverrideDecision::AlwaysAllow,
                "Test override".to_string()
            );

        engine.register_workspace_override(workspace_override);

        let result = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result.source, PolicySource::WorkspaceOverride);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_user_override_overrides_workspace() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = make_fs_op();

        let workspace_override = WorkspaceOverride::new(ctx.workspace)
            .add_override(
                fs_op_key(),
                OverrideDecision::AlwaysDeny,
                "Workspace denies".to_string()
            );
        engine.register_workspace_override(workspace_override);

        let user_override = UserOverride::new(ctx.user)
            .add_global_override(
                user_override_pattern(),
                OverrideDecision::AlwaysAllow,
                "User overrides to allow".to_string()
            );
        engine.register_user_override(user_override);

        let result = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result.source, PolicySource::UserOverride);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_temporary_override_has_highest_priority() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = make_fs_op();

        let workspace_override = WorkspaceOverride::new(ctx.workspace)
            .add_override(
                fs_op_key(),
                OverrideDecision::AlwaysDeny,
                "Workspace denies".to_string()
            );
        engine.register_workspace_override(workspace_override);

        let user_override = UserOverride::new(ctx.user)
            .add_global_override(
                user_override_pattern(),
                OverrideDecision::AlwaysDeny,
                "User also denies".to_string()
            );
        engine.register_user_override(user_override);

        let temp_override = TemporaryOverride::new(ctx.session, 30)
            .approve_operation(&op, OverrideDecision::AlwaysAllow, Some("admin".to_string()), None, "Emergency override".to_string());
        engine.register_temporary_override(temp_override);

        let result = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result.source, PolicySource::TemporaryOverride);
        assert_eq!(result.decision, PolicyDecision::Allow);
    }

    #[test]
    fn test_override_priority_chain() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = make_fs_op();

        let result_global = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result_global.source, PolicySource::GlobalPolicy);

        let workspace_override = WorkspaceOverride::new(ctx.workspace)
            .add_override(
                fs_op_key(),
                OverrideDecision::AlwaysAllow,
                "Workspace override".to_string()
            );
        engine.register_workspace_override(workspace_override);

        let result_workspace = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result_workspace.source, PolicySource::WorkspaceOverride);

        let user_override = UserOverride::new(ctx.user)
            .add_global_override(
                user_override_pattern(),
                OverrideDecision::AlwaysDeny,
                "User override".to_string()
            );
        engine.register_user_override(user_override);

        let result_user = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result_user.source, PolicySource::UserOverride);
        assert_eq!(result_user.decision, PolicyDecision::Deny);
    }

    #[test]
    fn test_expired_override_is_skipped() {
        let engine = PolicyEngine::new();
        let ctx = make_ctx();
        let op = make_fs_op();

        let expired_time = chrono::Utc::now() - chrono::Duration::seconds(10);
        let workspace_override = WorkspaceOverride::new(ctx.workspace)
            .add_temporary_override(
                fs_op_key(),
                OverrideDecision::AlwaysAllow,
                expired_time,
                "Expired override".to_string()
            );
        engine.register_workspace_override(workspace_override);

        let result = engine.evaluate(&op, &ctx).unwrap();
        assert_eq!(result.source, PolicySource::GlobalPolicy);
    }
}