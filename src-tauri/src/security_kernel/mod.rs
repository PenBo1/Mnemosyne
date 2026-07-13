pub mod approval;
pub mod audit;
pub mod commands;
pub mod config;
pub mod kernel;
pub mod permission;
pub mod plugin;
pub mod policy;
pub mod rate_limiter;
pub mod resource_manager;
pub mod secrets;
pub mod state;
pub mod types;
pub mod validation;

pub use approval::{
    ApprovalManager, ApprovalStore, ApprovalStats,
    ApprovalId, ApprovalToken, ApprovalRequest, ApprovalResult,
    calculate_action_hash,
    ApprovalValidator, ValidationError, ValidationResult,
};
pub use audit::{
    AuditEventBus, EventHandler, SharedAuditEventBus,
    SecurityEvent, AuditEntry, AuditFilter,
    AuditStore,
    LoggingHandler, MetricsHandler,
};
pub use kernel::SecurityKernel;
pub use state::SecurityKernelState;
pub use plugin::{
    PluginId, PluginManifest, PluginManifestFile, PluginManifestError, PluginRiskLevel,
    PluginPermission, FsPermission, ShellPermission, NetworkPermission, 
    ClipboardPermission, NotificationPermission,
    PluginRegistry, PluginRecord, PermissionCheckResult, SharedPluginRegistry, create_shared_registry,
    PluginSecurity, PluginOperation, PluginInfo, PluginLoadError,
    PermissionApprovalResult,
};
pub use policy::{
    calculate_operation_risk, OperationRisk, PolicyDecision, PolicyEvaluation, PolicySource,
    PolicyEngine, GlobalPolicy, WorkspacePolicy, DefaultRiskDecisions,
    TemporaryOverride, TemporaryOverrideRegistry, TemporaryOperationOverride,
    UserOverride, UserOverrideRegistry,
    OverrideConditions, OverrideDecision, OperationOverride, WorkspaceOverrideRegistry,
    TimeWindow,
};
pub use rate_limiter::{
    RateLimiter, RateLimitResult, RatePolicy, RateStore, DefaultPolicies,
};
pub use resource_manager::{
    ResourceEnforcer,
    ResourceMonitor,
    ResourceManager,
    ResourceQuota,
    ResourceType,
    ResourceUsage,
    SystemInfo,
};
pub use types::{
    Action,
    Capability,
    CapabilityId,
    Constraint,
    OperationContext,
    Permission,
    Resource,
    RiskLevel,
    SessionId,
    TrustLevel,
    UserId,
    WorkspaceId,
};