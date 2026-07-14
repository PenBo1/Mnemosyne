use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::Utc;

use crate::shared::error::AppError;

use super::approval::{ApprovalManager, ApprovalId};
use super::audit::{AuditEventBus, SecurityEvent, SharedAuditEventBus, LoggingHandler, MetricsHandler};
use super::hooks::{HookEngine, HookEvent, HookPayload, HookRegistry};
use super::permission::{PermissionManager, Operation};
use super::policy::{PolicyEngine, PolicyDecision};
use super::rate_limiter::RateLimiter;
use super::resource_manager::ResourceManager;
use super::secrets::SecretManager;
use super::types::{OperationContext, WorkspaceId};
use super::validation::ValidationLayer;

/// 安全内核：统一的安全审批入口。
///
/// ## 适用边界
///
/// SecurityKernel 仅用于高风险 I/O 操作的审批与审计，具体覆盖：
/// - **文件系统**（`Operation::Filesystem`）：跨工作区读写、目录删除等
/// - **Shell 执行**（`Operation::Shell`）：外部命令、git 子进程
/// - **网络请求**（`Operation::Network`）：ai_http_stream 等出站 HTTP
///
/// ## 不适用场景
///
/// 以下操作**不**经过 kernel，由 domain/infrastructure 自行处理：
/// - 域内 DB 增删改查（novel/session/wiki 等 CRUD）
/// - 内存状态读写（state stores）
/// - 工作区内的本地纯计算
///
/// 原因：域内 DB 操作的访问控制已由 IPC 层的 workspace_id 校验 +
/// sqlx 参数化查询保证；走 kernel 会带来不必要的策略/限频/配额开销，
/// 且 DB 操作需要 `&Database` 句柄，无法塞进 `FnOnce() -> Result` 执行器。
///
/// ## 执行流程
///
/// `execute()` 串行经过：Validation → Policy → RateLimiter →
/// ResourceManager → Permission → executor → Audit。
pub struct SecurityKernel {
    policy: Arc<RwLock<PolicyEngine>>,
    permission: Arc<Mutex<PermissionManager>>,
    approval: Arc<Mutex<ApprovalManager>>,
    validation: Arc<RwLock<ValidationLayer>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    resource_manager: Arc<Mutex<ResourceManager>>,
    audit_bus: SharedAuditEventBus,
    secrets: Arc<Mutex<SecretManager>>,
    /// Hook 引擎 —— 在 execute 的关键节点派发 PreToolUse / PermissionRequest / PostToolUse。
    /// 通过 Arc 共享给需要派发 hook 的调用方（AgentEngine、SubAgentExecutor）。
    hook_engine: Arc<HookEngine>,
}

impl SecurityKernel {
    pub fn new() -> Self {
        let audit_bus = SharedAuditEventBus::new();
        audit_bus.subscribe(Box::new(LoggingHandler));
        audit_bus.subscribe(Box::new(MetricsHandler::new()));

        // 加载内置 security.json；解析失败时显式记录错误并回退到各组件默认值
        let config = match super::config::SecurityConfig::load_default() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to load bundled security.json; using component defaults"
                );
                None
            }
        };

        let policy = match &config {
            Some(c) => PolicyEngine::with_global_policy(c.global_policy.clone()),
            None => PolicyEngine::new(),
        };

        let rate_limiter = match &config {
            Some(c) => RateLimiter::with_policies(c.rate_policies.clone()),
            None => RateLimiter::new(),
        };

        let mut resource_manager = ResourceManager::new();
        if let Some(c) = &config {
            // 默认配额挂到系统工作区（nil UUID），系统级操作（如 ai_http_stream）受其约束
            resource_manager.set_quota(
                WorkspaceId(uuid::Uuid::nil()),
                c.resource_quota.clone(),
            );
        }

        let audit_bus_for_hooks = audit_bus.clone();
        let hook_engine = Arc::new(HookEngine::new(
            Arc::new(HookRegistry::new()),
            audit_bus_for_hooks,
        ));

        Self {
            policy: Arc::new(RwLock::new(policy)),
            permission: Arc::new(Mutex::new(PermissionManager::new())),
            approval: Arc::new(Mutex::new(ApprovalManager::new())),
            validation: Arc::new(RwLock::new(ValidationLayer::new())),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            audit_bus,
            secrets: Arc::new(Mutex::new(SecretManager::memory_only())),
            hook_engine,
        }
    }

    pub fn with_config(
        policy: PolicyEngine,
        permission_manager: PermissionManager,
        approval_manager: ApprovalManager,
        validation_layer: ValidationLayer,
        rate_limiter: RateLimiter,
        resource_manager: ResourceManager,
        audit_bus: AuditEventBus,
        secret_manager: SecretManager,
    ) -> Self {
        let shared_audit_bus = SharedAuditEventBus::from(audit_bus);
        shared_audit_bus.subscribe(Box::new(LoggingHandler));
        shared_audit_bus.subscribe(Box::new(MetricsHandler::new()));

        let audit_bus_for_hooks = shared_audit_bus.clone();
        let hook_engine = Arc::new(HookEngine::new(
            Arc::new(HookRegistry::new()),
            audit_bus_for_hooks,
        ));

        Self {
            policy: Arc::new(RwLock::new(policy)),
            permission: Arc::new(Mutex::new(permission_manager)),
            approval: Arc::new(Mutex::new(approval_manager)),
            validation: Arc::new(RwLock::new(validation_layer)),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
            resource_manager: Arc::new(Mutex::new(resource_manager)),
            audit_bus: shared_audit_bus,
            secrets: Arc::new(Mutex::new(secret_manager)),
            hook_engine,
        }
    }

    pub fn execute<F, T>(
        &self,
        operation_name: &str,
        op: &Operation,
        ctx: &OperationContext,
        executor: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError>,
    {
        self.execute_internal(operation_name, op, ctx, executor, true)
    }

    pub fn execute_without_quota<F, T>(
        &self,
        operation_name: &str,
        op: &Operation,
        ctx: &OperationContext,
        executor: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError>,
    {
        self.execute_internal(operation_name, op, ctx, executor, false)
    }

    /// 统一执行流程：Validation → Policy → RateLimiter →（可选）ResourceManager →
    /// Permission → executor → Audit。
    ///
    /// `enforce_quota` 为 false 时跳过资源配额检查（用于不消耗配额的系统操作）。
    /// 审计事件（OperationStart/PolicyDenied/ApprovalRequested/OperationComplete）
    /// 与频率记录行为在两种入口下保持一致。
    ///
    /// Hook 集成：
    /// - PreToolUse：在 OperationStart emit 之后、Validation 之前派发；aborted 则直接返回 Err。
    /// - PermissionRequest：在 PolicyDecision::RequireApproval 命中时派发（在 approval_token
    ///   校验之前）；aborted 则返回 Err。
    /// - PostToolUse：在 executor 完成后派发（无论成功/失败）；aborted 仅记录警告，
    ///   不覆盖 executor 的结果（操作已经发生）。
    ///
    /// 同步派发 async hook：使用 `tokio::task::block_in_place` + `Handle::current().block_on`。
    /// 若 registry 为空则跳过派发（避免无 tokio runtime 时 panic）。
    ///
    /// **约束**：hook handler 不得回调 SecurityKernel 的任何加锁方法（validation/policy/rate/
    /// resource_manager/permission），否则会死锁。内置 action（Log/Audit/Block/Custom）均不回调。
    fn execute_internal<F, T>(
        &self,
        operation_name: &str,
        op: &Operation,
        ctx: &OperationContext,
        executor: F,
        enforce_quota: bool,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError>,
    {
        let workspace_id_str = ctx.workspace.0.to_string();

        self.audit_bus.emit(SecurityEvent::OperationStart {
            operation: operation_name.to_string(),
            workspace: ctx.workspace,
            timestamp: Utc::now(),
        });

        // PreToolUse hook —— aborted 则直接拒绝操作。
        self.dispatch_hook_sync(HookEvent::PreToolUse, operation_name, ctx, None)?;

        let start = Instant::now();

        let validation = self.validation.read().unwrap();
        validation.validate_operation(op)?;

        let policy = self.policy.read().unwrap();
        let evaluation = policy.evaluate(op, ctx)?;

        match evaluation.decision {
            PolicyDecision::Deny => {
                self.audit_bus.emit(SecurityEvent::PolicyDenied {
                    operation: operation_name.to_string(),
                    workspace: ctx.workspace,
                    reason: evaluation.reason.clone(),
                });
                return Err(AppError::forbidden(format!(
                    "Policy denied: {}",
                    evaluation.reason
                )));
            }
            PolicyDecision::RequireApproval => {
                // PermissionRequest hook —— 在 approval_token 校验之前派发，
                // 让 hook 有机会记录或拦截审批请求。aborted 则返回 Err。
                self.dispatch_hook_sync(
                    HookEvent::PermissionRequest,
                    operation_name,
                    ctx,
                    None,
                )?;

                if let Some(token_id) = &ctx.approval_token {
                    let approval = self.approval.lock().unwrap();
                    approval.validate(&ApprovalId(token_id.0), op, &ctx.workspace)?;
                } else {
                    self.audit_bus.emit(SecurityEvent::ApprovalRequested {
                        approval_id: uuid::Uuid::new_v4(),
                        operation: operation_name.to_string(),
                        risk_level: super::types::RiskLevel::High,
                        workspace: ctx.workspace,
                    });
                    return Err(AppError::forbidden(
                        "Operation requires approval but no approval token provided"
                    ));
                }
            }
            PolicyDecision::Allow => {}
        }

        let rate = self.rate_limiter.read().unwrap();
        if let Err(e) = rate.check_and_fail(operation_name, ctx.workspace) {
            self.audit_bus.emit(SecurityEvent::RateLimited {
                operation: operation_name.to_string(),
                workspace: ctx.workspace,
                reason: e.to_string(),
            });
            return Err(e);
        }

        if enforce_quota {
            let rm = self.resource_manager.lock().unwrap();
            if let Err(e) = rm.check_quota(&ctx.workspace) {
                self.audit_bus.emit(SecurityEvent::ResourceExceeded {
                    workspace: ctx.workspace,
                    resource: "quota".to_string(),
                    quota: e.to_string(),
                });
                return Err(e);
            }
        }

        let permission = self.permission.lock().unwrap();
        if let Err(e) = permission.check(op, &workspace_id_str) {
            // Permission 拒绝目前复用 PolicyDenied 事件 —— reason 区分
            self.audit_bus.emit(SecurityEvent::PolicyDenied {
                operation: operation_name.to_string(),
                workspace: ctx.workspace,
                reason: format!("Permission denied: {}", e),
            });
            return Err(e);
        }

        let result = executor();

        let duration_ms = start.elapsed().as_millis() as u64;

        self.audit_bus.emit(SecurityEvent::OperationComplete {
            operation: operation_name.to_string(),
            workspace: ctx.workspace,
            duration_ms,
            success: result.is_ok(),
        });

        if result.is_ok() {
            rate.record(operation_name, ctx.workspace);
        }

        // PostToolUse hook —— 操作已发生，aborted 仅记录警告，不覆盖 result。
        let success = result.is_ok();
        self.dispatch_hook_sync_post(operation_name, ctx, success);

        result
    }

    /// 同步派发 hook（用于 PreToolUse / PermissionRequest）—— aborted 时返回 Err。
    ///
    /// 快速路径：registry 为空时直接返回 Ok（避免无 tokio runtime 时 panic）。
    ///
    /// `success_meta`：可选的 `success: bool` 元数据，PostToolUse 用。
    fn dispatch_hook_sync(
        &self,
        event: HookEvent,
        operation_name: &str,
        ctx: &OperationContext,
        success_meta: Option<bool>,
    ) -> Result<(), AppError> {
        if self.hook_engine.registry().count() == 0 {
            return Ok(());
        }

        let mut payload = HookPayload::new(event)
            .with_tool_name(operation_name)
            .with_workspace(ctx.workspace.0.to_string())
            .with_session(ctx.session.0.to_string());
        if let Some(s) = success_meta {
            payload = payload.with_metadata(
                "success",
                serde_json::Value::Bool(s),
            );
        }

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.hook_engine.dispatch(&payload).await
            })
        })?;
        Ok(())
    }

    /// PostToolUse 专用派发 —— 不传播 abort 错误（操作已发生）。
    fn dispatch_hook_sync_post(&self, operation_name: &str, ctx: &OperationContext, success: bool) {
        if self.hook_engine.registry().count() == 0 {
            return;
        }

        match self.dispatch_hook_sync(HookEvent::PostToolUse, operation_name, ctx, Some(success)) {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    operation = operation_name,
                    "PostToolUse hook aborted (operation already completed — abort ignored)"
                );
            }
        }
    }

    pub fn request_approval(
        &self,
        operation_name: &str,
        op: &Operation,
        ctx: &OperationContext,
        _reason: String,
    ) -> Result<ApprovalId, AppError> {
        let approval = self.approval.lock().unwrap();
        let token = approval.create_token(op, ctx.workspace);

        let risk = super::policy::calculate_operation_risk(op);
        let risk_level = operation_risk_to_level(risk);

        self.audit_bus.emit(SecurityEvent::ApprovalRequested {
            approval_id: token.id.0,
            operation: operation_name.to_string(),
            risk_level,
            workspace: ctx.workspace,
        });

        Ok(token.id)
    }

    pub fn approve_operation(
        &self,
        approval_id: ApprovalId,
        approved_by: String,
    ) -> Result<(), AppError> {
        let approval = self.approval.lock().unwrap();
        let token = approval.approve(approval_id, approved_by.clone())?;

        self.audit_bus.emit(SecurityEvent::ApprovalGranted {
            approval_id: approval_id.0,
            approved_by,
            workspace: token.workspace,
        });

        Ok(())
    }

    pub fn reject_approval(
        &self,
        approval_id: ApprovalId,
        rejected_by: String,
        reason: String,
    ) -> Result<(), AppError> {
        let approval = self.approval.lock().unwrap();
        let token = approval.reject(approval_id, rejected_by.clone())?;

        self.audit_bus.emit(SecurityEvent::ApprovalRejected {
            approval_id: approval_id.0,
            rejected_by,
            reason,
            workspace: token.workspace,
        });

        Ok(())
    }

    pub fn policy(&self) -> &Arc<RwLock<PolicyEngine>> {
        &self.policy
    }

    pub fn permission_manager(&self) -> &Arc<Mutex<PermissionManager>> {
        &self.permission
    }

    pub fn approval_manager(&self) -> &Arc<Mutex<ApprovalManager>> {
        &self.approval
    }

    pub fn validation_layer(&self) -> &Arc<RwLock<ValidationLayer>> {
        &self.validation
    }

    pub fn rate_limiter(&self) -> &Arc<RwLock<RateLimiter>> {
        &self.rate_limiter
    }

    pub fn resource_manager(&self) -> &Arc<Mutex<ResourceManager>> {
        &self.resource_manager
    }

    pub fn audit_bus(&self) -> &SharedAuditEventBus {
        &self.audit_bus
    }

    pub fn secret_manager(&self) -> &Arc<Mutex<SecretManager>> {
        &self.secrets
    }

    /// Hook 引擎引用 —— 供 AgentEngine / SubAgentExecutor 派发 SessionStart / Stop / Subagent* 事件，
    /// 以及 IPC 命令通过 HookEngineState 操作 registry。
    pub fn hook_engine(&self) -> &Arc<HookEngine> {
        &self.hook_engine
    }

    pub fn cleanup(&self) {
        let policy = self.policy.read().unwrap();
        policy.cleanup_expired();

        let rate = self.rate_limiter.read().unwrap();
        rate.cleanup_expired();

        self.audit_bus.cleanup_expired();

        let approval = self.approval.lock().unwrap();
        approval.cleanup_expired();
    }

    pub fn stats(&self) -> KernelStats {
        let policy = self.policy.read().unwrap();
        let rate = self.rate_limiter.read().unwrap();
        let approval = self.approval.lock().unwrap();
        let rm = self.resource_manager.lock().unwrap();
        let permission = self.permission.lock().unwrap();

        KernelStats {
            policy_workspaces: policy.get_workspace_count(),
            policy_users: policy.get_user_count(),
            policy_temporary: policy.get_temporary_count(),
            rate_policies: rate.policies_count(),
            rate_records: rate.total_records(),
            approval_pending: approval.pending_count(),
            approval_approved: approval.approved_count(),
            resource_quotas: rm.get_all_quotas().len(),
            resource_usage_entries: rm.get_all_usage().len(),
            permission_sessions: permission.session_count(),
            audit_entries: self.audit_bus.total_entries(),
        }
    }
}

impl Default for SecurityKernel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelStats {
    pub policy_workspaces: usize,
    pub policy_users: usize,
    pub policy_temporary: usize,
    pub rate_policies: usize,
    pub rate_records: usize,
    pub approval_pending: usize,
    pub approval_approved: usize,
    pub resource_quotas: usize,
    pub resource_usage_entries: usize,
    pub permission_sessions: usize,
    pub audit_entries: usize,
}

fn operation_risk_to_level(risk: super::policy::OperationRisk) -> super::types::RiskLevel {
    use super::policy::OperationRisk;
    use super::types::RiskLevel;
    match risk {
        OperationRisk::Low => RiskLevel::Low,
        OperationRisk::Medium => RiskLevel::Medium,
        OperationRisk::High => RiskLevel::High,
        OperationRisk::Critical => RiskLevel::Critical,
    }
}