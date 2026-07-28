//! ═══════════════════════════════════════════════════════════════════════════
//! policy - 审批策略配置
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── 顶层策略 ────────────────────────────────────────────────────────────────

/// 审批策略，控制 Security Kernel 何时就操作向用户请求审批。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AskForApproval {
    /// 从不审批：所有操作自动执行，不提示用户。
    ///
    /// 仅适用于完全受信任的只读上下文。高风险操作仍受 hardline patterns
    /// 和 sandbox 约束。
    Never,

    /// 仅在失败时审批：操作自动执行，仅在失败或被策略拒绝时提示用户。
    ///
    /// 适用于大多数开发场景——沙箱内的写操作自动放行，越界操作才提示。
    #[default]
    OnFailure,

    /// 按需审批：每个非平凡操作都需用户显式批准。
    ///
    /// 最保守策略，适用于敏感环境。
    OnRequest,

    /// 除非受信任，否则审批：受信任路径（writable_roots 内）自动放行，
    /// 非信任路径需审批。
    UnlessTrusted,

    /// 细粒度审批：按 5 个子开关分别控制不同场景。
    ///
    /// 见 `GranularConfig`。
    Granular,
}

impl AskForApproval {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::OnFailure => "on_failure",
            Self::OnRequest => "on_request",
            Self::UnlessTrusted => "unless_trusted",
            Self::Granular => "granular",
        }
    }

    /// 渲染为权限指令片段用的模板文本。
    pub fn render_template(self) -> &'static str {
        match self {
            Self::Never => {
                "Approval policy: never. Operations proceed without user prompts. \
                 Use only in fully trusted, read-only contexts."
            }
            Self::OnFailure => {
                "Approval policy: on_failure. Operations proceed automatically; \
                 user is prompted only when an operation fails or is denied by policy."
            }
            Self::OnRequest => {
                "Approval policy: on_request. Every non-trivial operation requires \
                 explicit user approval before execution."
            }
            Self::UnlessTrusted => {
                "Approval policy: unless_trusted. Operations on trusted paths \
                 proceed automatically; untrusted operations require approval."
            }
            Self::Granular => {
                "Approval policy: granular. Approval is controlled by 5 sub-toggles: \
                 sandbox_approval, rules, skill_approval, request_permissions, \
                 and mcp_elicitations. See Security Kernel config for details."
            }
        }
    }
}

// ── Granular 子开关 ────────────────────────────────────────────────────────

/// Granular 模式下的 5 个子开关。
///
/// 每个子开关控制一类场景是否需要审批：
/// - `sandbox_approval`：沙箱逃逸请求（如写入 writable_roots 外）
/// - `rules`：ExecPolicy 规则触发的审批
/// - `skill_approval`：技能调用审批
/// - `request_permissions`：权限提升请求
/// - `mcp_elicitations`：MCP 服务器 elicitation 请求
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GranularConfig {
    pub sandbox_approval: bool,
    pub rules: bool,
    pub skill_approval: bool,
    pub request_permissions: bool,
    pub mcp_elicitations: bool,
}

impl Default for GranularConfig {
    fn default() -> Self {
        // 默认全部启用审批（保守）
        Self {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }
    }
}

impl GranularConfig {
    /// 全部禁用审批（等同 Never）。
    pub fn all_disabled() -> Self {
        Self {
            sandbox_approval: false,
            rules: false,
            skill_approval: false,
            request_permissions: false,
            mcp_elicitations: false,
        }
    }

    /// 全部启用审批（最保守）。
    pub fn all_enabled() -> Self {
        Self::default()
    }
}

// ── 审批场景 ────────────────────────────────────────────────────────────────

/// 触发审批的场景类型（用于 Granular 子开关路由）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalScene {
    /// 沙箱逃逸请求。
    SandboxEscape,
    /// ExecPolicy 规则触发。
    ExecPolicyRule,
    /// 技能调用。
    SkillInvocation,
    /// 权限提升请求。
    RequestPermission,
    /// MCP elicitation 请求。
    McpElicitation,
}

// ── 决策入口 ────────────────────────────────────────────────────────────────

/// 审批策略决策器：封装 AskForApproval + GranularConfig 的组合决策。
#[derive(Debug, Clone, Default)]
pub struct ApprovalPolicyEngine {
    pub policy: AskForApproval,
    pub granular: GranularConfig,
}

impl ApprovalPolicyEngine {
    pub fn new(policy: AskForApproval, granular: GranularConfig) -> Self {
        Self { policy, granular }
    }

    /// 判断给定场景是否需要提示用户审批。
    ///
    /// 决策逻辑：
    /// - Never → 永不提示
    /// - OnFailure → 不提示（仅在失败时才提示，由调用方在失败后调用 `should_prompt_on_failure`）
    /// - OnRequest → 总是提示
    /// - UnlessTrusted → 由调用方判断是否 trusted，trusted 时不提示
    /// - Granular → 按子开关路由
    pub fn should_prompt(&self, scene: ApprovalScene) -> bool {
        match self.policy {
            AskForApproval::Never => false,
            AskForApproval::OnFailure => false,
            AskForApproval::OnRequest => true,
            AskForApproval::UnlessTrusted => true, // 调用方需先检查 trusted 状态
            AskForApproval::Granular => self.granular_enabled(scene),
        }
    }

    /// OnFailure 策略下，操作失败后是否需要提示。
    pub fn should_prompt_on_failure(&self) -> bool {
        match self.policy {
            AskForApproval::Never => false,
            AskForApproval::OnFailure => true,
            AskForApproval::OnRequest => true,
            AskForApproval::UnlessTrusted => true,
            AskForApproval::Granular => true,
        }
    }

    /// Granular 模式下，检查子开关是否启用。
    fn granular_enabled(&self, scene: ApprovalScene) -> bool {
        match scene {
            ApprovalScene::SandboxEscape => self.granular.sandbox_approval,
            ApprovalScene::ExecPolicyRule => self.granular.rules,
            ApprovalScene::SkillInvocation => self.granular.skill_approval,
            ApprovalScene::RequestPermission => self.granular.request_permissions,
            ApprovalScene::McpElicitation => self.granular.mcp_elicitations,
        }
    }
}

// ── ReviewDecision（缓存条目） ────────────────────────────────────────────────

/// 审批决策结果（用于 ApprovalStore 缓存）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    /// 用户批准操作。
    Approved,
    /// 用户拒绝操作。
    Denied,
    /// 延迟决策（用户未响应）。
    Deferred,
}

impl ReviewDecision {
    pub fn is_approved(self) -> bool {
        matches!(self, Self::Approved)
    }

    pub fn is_denied(self) -> bool {
        matches!(self, Self::Denied)
    }
}

// ── 会话级缓存条目 ────────────────────────────────────────────────────────

/// 会话级审批缓存条目。
///
/// 当用户对某操作选择"本次会话内始终允许"时，记录此条目，
/// 后续相同操作（按 action_hash 匹配）直接放行，不再提示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalCacheEntry {
    /// 审批决策。
    pub decision: ReviewDecision,
    /// 是否"本次会话内始终允许"（true 时缓存生效到会话结束）。
    pub approved_for_session: bool,
}

impl ApprovalCacheEntry {
    pub fn approved_for_session() -> Self {
        Self {
            decision: ReviewDecision::Approved,
            approved_for_session: true,
        }
    }

    pub fn denied() -> Self {
        Self {
            decision: ReviewDecision::Denied,
            approved_for_session: false,
        }
    }

    pub fn is_session_approved(&self) -> bool {
        self.decision.is_approved() && self.approved_for_session
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_for_approval_as_str() {
        assert_eq!(AskForApproval::Never.as_str(), "never");
        assert_eq!(AskForApproval::OnFailure.as_str(), "on_failure");
        assert_eq!(AskForApproval::OnRequest.as_str(), "on_request");
        assert_eq!(AskForApproval::UnlessTrusted.as_str(), "unless_trusted");
        assert_eq!(AskForApproval::Granular.as_str(), "granular");
    }

    #[test]
    fn test_ask_for_approval_default_is_on_failure() {
        assert_eq!(AskForApproval::default(), AskForApproval::OnFailure);
    }

    #[test]
    fn test_ask_for_approval_render_templates_differ() {
        let templates = [
            AskForApproval::Never.render_template(),
            AskForApproval::OnFailure.render_template(),
            AskForApproval::OnRequest.render_template(),
            AskForApproval::UnlessTrusted.render_template(),
            AskForApproval::Granular.render_template(),
        ];
        // 所有模板应互不相同
        for i in 0..templates.len() {
            for j in (i + 1)..templates.len() {
                assert_ne!(templates[i], templates[j], "templates {} and {} are identical", i, j);
            }
        }
    }

    #[test]
    fn test_granular_config_default_all_enabled() {
        let cfg = GranularConfig::default();
        assert!(cfg.sandbox_approval);
        assert!(cfg.rules);
        assert!(cfg.skill_approval);
        assert!(cfg.request_permissions);
        assert!(cfg.mcp_elicitations);
    }

    #[test]
    fn test_granular_config_all_disabled() {
        let cfg = GranularConfig::all_disabled();
        assert!(!cfg.sandbox_approval);
        assert!(!cfg.rules);
        assert!(!cfg.skill_approval);
        assert!(!cfg.request_permissions);
        assert!(!cfg.mcp_elicitations);
    }

    #[test]
    fn test_policy_engine_never_never_prompts() {
        let engine = ApprovalPolicyEngine::new(AskForApproval::Never, GranularConfig::default());
        for scene in [
            ApprovalScene::SandboxEscape,
            ApprovalScene::ExecPolicyRule,
            ApprovalScene::SkillInvocation,
            ApprovalScene::RequestPermission,
            ApprovalScene::McpElicitation,
        ] {
            assert!(!engine.should_prompt(scene), "Never should not prompt for {:?}", scene);
        }
        assert!(!engine.should_prompt_on_failure());
    }

    #[test]
    fn test_policy_engine_on_request_always_prompts() {
        let engine = ApprovalPolicyEngine::new(AskForApproval::OnRequest, GranularConfig::default());
        assert!(engine.should_prompt(ApprovalScene::SandboxEscape));
        assert!(engine.should_prompt_on_failure());
    }

    #[test]
    fn test_policy_engine_on_failure_prompts_only_on_failure() {
        let engine = ApprovalPolicyEngine::new(AskForApproval::OnFailure, GranularConfig::default());
        assert!(!engine.should_prompt(ApprovalScene::SandboxEscape));
        assert!(engine.should_prompt_on_failure());
    }

    #[test]
    fn test_policy_engine_granular_respects_sub_toggles() {
        let cfg = GranularConfig {
            sandbox_approval: true,
            rules: false,
            skill_approval: true,
            request_permissions: false,
            mcp_elicitations: true,
        };
        let engine = ApprovalPolicyEngine::new(AskForApproval::Granular, cfg);

        assert!(engine.should_prompt(ApprovalScene::SandboxEscape));
        assert!(!engine.should_prompt(ApprovalScene::ExecPolicyRule));
        assert!(engine.should_prompt(ApprovalScene::SkillInvocation));
        assert!(!engine.should_prompt(ApprovalScene::RequestPermission));
        assert!(engine.should_prompt(ApprovalScene::McpElicitation));
    }

    #[test]
    fn test_policy_engine_unless_trusted_prompts_by_default() {
        let engine = ApprovalPolicyEngine::new(AskForApproval::UnlessTrusted, GranularConfig::default());
        // UnlessTrusted 默认提示（调用方需先检查 trusted 状态）
        assert!(engine.should_prompt(ApprovalScene::SandboxEscape));
    }

    #[test]
    fn test_review_decision_helpers() {
        assert!(ReviewDecision::Approved.is_approved());
        assert!(!ReviewDecision::Approved.is_denied());
        assert!(ReviewDecision::Denied.is_denied());
        assert!(!ReviewDecision::Denied.is_approved());
        assert!(!ReviewDecision::Deferred.is_approved());
        assert!(!ReviewDecision::Deferred.is_denied());
    }

    #[test]
    fn test_approval_cache_entry_approved_for_session() {
        let entry = ApprovalCacheEntry::approved_for_session();
        assert!(entry.is_session_approved());
        assert_eq!(entry.decision, ReviewDecision::Approved);
    }

    #[test]
    fn test_approval_cache_entry_denied() {
        let entry = ApprovalCacheEntry::denied();
        assert!(!entry.is_session_approved());
        assert_eq!(entry.decision, ReviewDecision::Denied);
    }

    #[test]
    fn test_approval_cache_entry_approved_not_for_session() {
        // Approved 但 approved_for_session=false → 不是会话级缓存
        let entry = ApprovalCacheEntry {
            decision: ReviewDecision::Approved,
            approved_for_session: false,
        };
        assert!(!entry.is_session_approved());
    }
}