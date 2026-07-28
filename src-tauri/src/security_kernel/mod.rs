//! ═══════════════════════════════════════════════════════════════════════════
//! security_kernel - 安全内核模块
//! ═══════════════════════════════════════════════════════════════════════════

pub mod approval;
pub mod audit;
pub mod commands;
pub mod config;
pub mod hooks;
pub mod kernel;
pub mod permission;
pub mod plugin;
pub mod policy;
pub mod pve;
pub mod rate_limiter;
pub mod resource_manager;
pub mod sandbox;
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
pub use hooks::{
    HookEngine, HookEngineState, HookRegistry, HookDispatchOutcome,
    ConfiguredHook, HookAction, HookConfig, HookEvent, HookFn, HookHandler, HookInfo,
    HookMatcher, HookPayload, HookResult, HookSpec, HookTestRequest, HookTestResult,
    MatcherPattern,
    HookDispatcher, OptionalHookDispatcher, try_dispatch,
    session_start_payload, user_prompt_submit_payload, stop_payload,
    subagent_start_payload, subagent_stop_payload,
    pre_compact_payload, post_compact_payload,
    handler_for_action,
    hook_list, hook_register, hook_test_dispatch, hook_unregister,
    HookDiscovery, HookLoader, HookRunner, CommandRunner, HttpRunner,
    validate_url_for_ssrf,
    DEFAULT_HOOK_CONFIG_FILE, DEFAULT_HOOK_DIR,
    default_hook_config_path, default_hook_dir_path,
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
pub use pve::{
    ParameterValidator, ParameterSchema, FieldType,
    InjectionScanner, InjectionPattern, InjectionMatch, ContentSegment,
    ContentSource, InjectionCategory, InjectionSeverity,
    get_default_injection_patterns,
    IntentChecker, SensitiveSurface, SensitiveSurfaceType,
    SensitiveSurfaceMatch, OperationIntent, IntentCheckResult,
    is_path_sensitive,
    PveConfig, ToolCallContext, PveResult, PromptValidatorExecutor,
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