//! ═══════════════════════════════════════════════════════════════════════════
//! policy - 策略引擎模块
//! ═══════════════════════════════════════════════════════════════════════════

mod decision;
mod engine;
mod global_policy;
mod network_rule;
mod prefix_rule;
mod rule_parser;
mod temporary_override;
mod user_override;
mod workspace_override;

pub use decision::{
    calculate_operation_risk, OperationRisk, PolicyDecision, PolicyEvaluation, PolicySource,
};
pub use engine::PolicyEngine;
pub use global_policy::{DefaultRiskDecisions, GlobalPolicy, WorkspacePolicy};
pub use network_rule::{normalize_network_rule_host, NetworkRule, NetworkRuleProtocol};
pub use prefix_rule::{
    PatternToken, PrefixPattern, PrefixRule, Rule, RuleMatch, RuleRef,
};
pub use rule_parser::{
    create_exec_approval_requirement_for_command, merge_layered_policies, parse_policy_file,
    policy_file_to_policy, ApprovalPolicyForExec, ExecApprovalRequirement, LayeredPolicySources,
    MergeOptions, NetworkRuleSpec, Policy as ExecPolicy, PolicyFile, PrefixRuleSpec,
};
pub use temporary_override::{TemporaryOverride, TemporaryOverrideRegistry, TemporaryOperationOverride};
pub use user_override::{UserOverride, UserOverrideRegistry};
pub use workspace_override::{
    OverrideConditions, OverrideDecision, OperationOverride, WorkspaceOverride, WorkspaceOverrideRegistry,
    TimeWindow,
};