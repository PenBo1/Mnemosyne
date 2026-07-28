//! ═══════════════════════════════════════════════════════════════════════════
//! rule_parser - 规则文件解析器模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::decision::PolicyDecision;
use super::network_rule::{normalize_network_rule_host, NetworkRule, NetworkRuleProtocol};
use super::prefix_rule::{PatternToken, PrefixPattern, PrefixRule, RuleMatch, RuleRef};

// ── JSON DSL 结构 ────────────────────────────────────────────────────────────────

/// 规则文件根结构（JSON DSL）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PolicyFile {
    /// 前缀规则列表。
    #[serde(default)]
    pub rules: Vec<PrefixRuleSpec>,
    /// 网络规则列表。
    #[serde(default)]
    pub network_rules: Vec<NetworkRuleSpec>,
}

/// 前缀规则规格（JSON DSL）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefixRuleSpec {
    /// 命令前缀 token 数组，如 `["git", "push"]`。
    pub prefix: Vec<String>,
    /// 决策：allow / require_approval / deny。
    pub decision: String,
    /// 规则说明（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
}

/// 网络规则规格（JSON DSL）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkRuleSpec {
    /// host（会被规范化）。
    pub host: String,
    /// 协议：http / https / socks5_tcp / socks5_udp。
    pub protocol: String,
    /// 决策：allow / require_approval / deny。
    pub decision: String,
    /// 规则说明（可选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
}

// ── Policy 结构 ────────────────────────────────────────────────────────────────

/// ExecPolicy：前缀规则 + 网络规则 + host 可执行文件映射。
#[derive(Clone, Debug, Default)]
pub struct Policy {
    /// 按首 token 分组的前缀规则。
    pub rules_by_program: HashMap<String, Vec<RuleRef>>,
    /// 网络规则列表。
    pub network_rules: Vec<NetworkRule>,
    /// host 可执行文件路径映射（name → 已知路径列表）。
    /// 用于解析绝对路径形式的命令（如 `/usr/bin/git` → 按 `git` 查找规则）。
    pub host_executables_by_name: HashMap<String, Vec<PathBuf>>,
}

impl Policy {
    /// 构造空策略。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 从规则列表构造（无网络规则、无 host 可执行文件）。
    pub fn from_rules(rules_by_program: HashMap<String, Vec<RuleRef>>) -> Self {
        Self {
            rules_by_program,
            network_rules: Vec::new(),
            host_executables_by_name: HashMap::new(),
        }
    }

    /// 添加前缀规则。
    pub fn add_prefix_rule(
        &mut self,
        prefix: &[String],
        decision: PolicyDecision,
        justification: Option<String>,
    ) -> Result<(), String> {
        let (first_token, rest) = prefix
            .split_first()
            .ok_or_else(|| "prefix 不能为空".to_string())?;

        let rule: RuleRef = Arc::new(PrefixRule {
            pattern: PrefixPattern {
                first: Arc::from(first_token.as_str()),
                rest: rest
                    .iter()
                    .map(|token| PatternToken::Single(token.clone()))
                    .collect::<Vec<_>>()
                    .into(),
            },
            decision,
            justification,
        });

        self.rules_by_program
            .entry(first_token.clone())
            .or_default()
            .push(rule);
        Ok(())
    }

    /// 添加网络规则。
    pub fn add_network_rule(
        &mut self,
        host: &str,
        protocol: NetworkRuleProtocol,
        decision: PolicyDecision,
        justification: Option<String>,
    ) -> Result<(), String> {
        let host = normalize_network_rule_host(host)?;
        if let Some(raw) = justification.as_deref() {
            if raw.trim().is_empty() {
                return Err("justification 不能为空".to_string());
            }
        }
        self.network_rules.push(NetworkRule {
            host,
            protocol,
            decision,
            justification,
        });
        Ok(())
    }

    /// 设置 host 可执行文件路径。
    pub fn set_host_executable_paths(&mut self, name: String, paths: Vec<PathBuf>) {
        self.host_executables_by_name.insert(name, paths);
    }

    /// 获取所有 Allow 决策的前缀（用于权限指令片段）。
    pub fn get_allowed_prefixes(&self) -> Vec<Vec<String>> {
        let mut prefixes = Vec::new();
        for rules in self.rules_by_program.values() {
            for rule in rules {
                let Some(prefix_rule) = rule.as_any().downcast_ref::<PrefixRule>() else {
                    continue;
                };
                if prefix_rule.decision != PolicyDecision::Allow {
                    continue;
                }
                let mut prefix = Vec::with_capacity(prefix_rule.pattern.rest.len() + 1);
                prefix.push(prefix_rule.pattern.first.as_ref().to_string());
                prefix.extend(prefix_rule.pattern.rest.iter().map(render_pattern_token));
                prefixes.push(prefix);
            }
        }
        prefixes.sort();
        prefixes.dedup();
        prefixes
    }

    /// 聚合网络规则为 (allowed, denied) 列表。
    ///
    /// - Allow：从 denied 移除，加入 allowed
    /// - Deny：从 allowed 移除，加入 denied
    /// - RequireApproval：不进入任一列表
    pub fn compiled_network_domains(&self) -> (Vec<String>, Vec<String>) {
        let mut allowed = Vec::new();
        let mut denied = Vec::new();
        for rule in &self.network_rules {
            match rule.decision {
                PolicyDecision::Allow => {
                    denied.retain(|entry| entry != &rule.host);
                    upsert_domain(&mut allowed, &rule.host);
                }
                PolicyDecision::Deny => {
                    allowed.retain(|entry| entry != &rule.host);
                    upsert_domain(&mut denied, &rule.host);
                }
                PolicyDecision::RequireApproval => {}
            }
        }
        (allowed, denied)
    }

    /// 查找命令的所有匹配规则。
    ///
    /// 若无显式规则匹配且提供了 heuristics_fallback，则返回单条 HeuristicsRuleMatch。
    pub fn matches_for_command(
        &self,
        cmd: &[String],
        heuristics_fallback: Option<&dyn Fn(&[String]) -> PolicyDecision>,
    ) -> Vec<RuleMatch> {
        let matched_rules = self.match_exact_rules(cmd);
        if !matched_rules.is_empty() {
            return matched_rules;
        }
        if let Some(fallback) = heuristics_fallback {
            vec![RuleMatch::HeuristicsRuleMatch {
                command: cmd.to_vec(),
                decision: fallback(cmd),
            }]
        } else {
            Vec::new()
        }
    }

    fn match_exact_rules(&self, cmd: &[String]) -> Vec<RuleMatch> {
        let Some(first) = cmd.first() else {
            return Vec::new();
        };
        match self.rules_by_program.get(first) {
            Some(rules) => rules.iter().filter_map(|rule| rule.matches(cmd)).collect(),
            None => Vec::new(),
        }
    }

    /// 合并 overlay 策略（用于多层合并）。
    ///
    /// 返回新 Policy，包含 self + overlay 的所有规则。
    /// overlay 中同 program 的规则会追加到 self 的规则列表之后（后匹配但同等优先级）。
    pub fn merge_overlay(&self, overlay: &Policy) -> Policy {
        let mut combined_rules = self.rules_by_program.clone();
        for (program, rules) in &overlay.rules_by_program {
            combined_rules
                .entry(program.clone())
                .or_default()
                .extend(rules.iter().cloned());
        }
        let mut combined_network_rules = self.network_rules.clone();
        combined_network_rules.extend(overlay.network_rules.iter().cloned());
        let mut host_executables = self.host_executables_by_name.clone();
        host_executables.extend(overlay.host_executables_by_name.clone());
        Policy {
            rules_by_program: combined_rules,
            network_rules: combined_network_rules,
            host_executables_by_name: host_executables,
        }
    }
}

fn upsert_domain(entries: &mut Vec<String>, host: &str) {
    entries.retain(|entry| entry != host);
    entries.push(host.to_string());
}

fn render_pattern_token(token: &PatternToken) -> String {
    match token {
        PatternToken::Single(value) => value.clone(),
        PatternToken::Alts(alternatives) => format!("[{}]", alternatives.join("|")),
    }
}

// ── 规则文件解析 ────────────────────────────────────────────────────────────────

/// 解析单个 JSON 规则文件为 PolicyFile。
pub fn parse_policy_file(content: &str) -> Result<PolicyFile, String> {
    serde_json::from_str(content).map_err(|e| format!("解析策略文件失败: {e}"))
}

/// 将 PolicyFile 转换为 Policy（构造 RuleRef + 规范化 host）。
pub fn policy_file_to_policy(file: &PolicyFile) -> Result<Policy, String> {
    let mut policy = Policy::empty();
    for spec in &file.rules {
        let decision = parse_decision(&spec.decision)?;
        policy.add_prefix_rule(&spec.prefix, decision, spec.justification.clone())?;
    }
    for spec in &file.network_rules {
        let protocol = NetworkRuleProtocol::parse(&spec.protocol)?;
        let decision = parse_decision(&spec.decision)?;
        policy.add_network_rule(
            &spec.host,
            protocol,
            decision,
            spec.justification.clone(),
        )?;
    }
    Ok(policy)
}

fn parse_decision(raw: &str) -> Result<PolicyDecision, String> {
    match raw {
        "allow" => Ok(PolicyDecision::Allow),
        "require_approval" | "prompt" => Ok(PolicyDecision::RequireApproval),
        "deny" | "forbidden" => Ok(PolicyDecision::Deny),
        other => Err(format!(
            "无效决策 '{other}': 必须是 allow / require_approval / deny"
        )),
    }
}

// ── 多层规则合并 ────────────────────────────────────────────────────────────────

/// 多层规则源：enterprise → user → project（优先级从低到高）。
#[derive(Clone, Debug, Default)]
pub struct LayeredPolicySources {
    pub enterprise: Option<PolicyFile>,
    pub user: Option<PolicyFile>,
    pub project: Option<PolicyFile>,
}

/// 多层合并选项。
#[derive(Clone, Debug, Default)]
pub struct MergeOptions {
    /// 若为 true，忽略 user 和 project 层（仅 enterprise 生效）。
    pub ignore_user_and_project_exec_policy_rules: bool,
}

/// 多层合并：enterprise → user → project。
///
/// 合并顺序：先 enterprise，再 user（可选），再 project（可选）。
/// 每层通过 `merge_overlay` 叠加，后叠加的规则与先前的同等优先级（追加而非覆盖）。
///
/// 若 `ignore_user_and_project_exec_policy_rules` 为 true，仅使用 enterprise 层。
pub fn merge_layered_policies(
    sources: &LayeredPolicySources,
    options: &MergeOptions,
) -> Result<Policy, String> {
    let mut policy = Policy::empty();

    if let Some(enterprise) = &sources.enterprise {
        let p = policy_file_to_policy(enterprise)?;
        policy = policy.merge_overlay(&p);
    }

    if options.ignore_user_and_project_exec_policy_rules {
        return Ok(policy);
    }

    if let Some(user) = &sources.user {
        let p = policy_file_to_policy(user)?;
        policy = policy.merge_overlay(&p);
    }
    if let Some(project) = &sources.project {
        let p = policy_file_to_policy(project)?;
        policy = policy.merge_overlay(&p);
    }

    Ok(policy)
}

// ── 决策流 ────────────────────────────────────────────────────────────────────────

/// 命令审批需求结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecApprovalRequirement {
    /// 命令被禁止执行。
    Forbidden { reason: String },
    /// 命令需要用户审批。
    NeedsApproval { reason: String },
    /// 命令可直接执行（可跳过沙箱）。
    Skip { bypass_sandbox: bool },
}

/// 审批策略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalPolicyForExec {
    /// 从不审批（所有需审批的命令直接拒绝）。
    Never,
    /// 允许审批（需审批时走审批流）。
    AllowApproval,
}

/// 为单条命令推导审批需求。
///
/// 决策流：
/// 1. 在 policy 中查找匹配规则
/// 2. 无匹配时调用 heuristics_fallback（如 dangerous/safe 启发式）
/// 3. 取所有匹配规则的最大决策（Deny > RequireApproval > Allow）
/// 4. Deny → Forbidden；RequireApproval + Never → Forbidden；RequireApproval + AllowApproval → NeedsApproval；
///    Allow → Skip（仅当所有匹配均为 Allow 时 bypass_sandbox=true）
pub fn create_exec_approval_requirement_for_command(
    policy: &Policy,
    cmd: &[String],
    approval_policy: ApprovalPolicyForExec,
    heuristics_fallback: &dyn Fn(&[String]) -> PolicyDecision,
) -> ExecApprovalRequirement {
    let matches = policy.matches_for_command(cmd, Some(heuristics_fallback));
    // 取最大决策：Deny > RequireApproval > Allow
    let decision = matches
        .iter()
        .map(|m| m.decision())
        .max()
        .unwrap_or(PolicyDecision::Deny); // 无匹配时默认 Deny（保守）

    match decision {
        PolicyDecision::Deny => ExecApprovalRequirement::Forbidden {
            reason: derive_forbidden_reason(cmd, &matches),
        },
        PolicyDecision::RequireApproval => {
            // Never 策略下，需审批的命令直接拒绝
            if matches!(approval_policy, ApprovalPolicyForExec::Never) {
                ExecApprovalRequirement::Forbidden {
                    reason: "命令需要审批但审批策略为 Never".to_string(),
                }
            } else {
                ExecApprovalRequirement::NeedsApproval {
                    reason: derive_prompt_reason(cmd, &matches),
                }
            }
        }
        PolicyDecision::Allow => {
            // 仅当所有匹配均为显式 Allow 规则时 bypass_sandbox=true
            let bypass_sandbox = matches.iter().all(|m| {
                m.is_policy_match() && m.decision() == PolicyDecision::Allow
            });
            ExecApprovalRequirement::Skip { bypass_sandbox }
        }
    }
}

fn derive_forbidden_reason(cmd: &[String], matches: &[RuleMatch]) -> String {
    // 优先取显式 Deny 规则的 justification
    for m in matches {
        if let RuleMatch::PrefixRuleMatch {
            decision: PolicyDecision::Deny,
            justification: Some(j),
            ..
        } = m
        {
            return j.clone();
        }
    }
    format!("命令 {:?} 被策略禁止", cmd)
}

fn derive_prompt_reason(cmd: &[String], matches: &[RuleMatch]) -> String {
    for m in matches {
        if let RuleMatch::PrefixRuleMatch {
            decision: PolicyDecision::RequireApproval,
            justification: Some(j),
            ..
        } = m
        {
            return j.clone();
        }
    }
    format!("命令 {:?} 需要审批", cmd)
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    fn deny_heuristic(_cmd: &[String]) -> PolicyDecision {
        PolicyDecision::Deny
    }

    fn allow_heuristic(_cmd: &[String]) -> PolicyDecision {
        PolicyDecision::Allow
    }

    #[test]
    fn test_parse_policy_file_valid() {
        let json = r#"{
            "rules": [
                { "prefix": ["git", "status"], "decision": "allow" },
                { "prefix": ["git", "push"], "decision": "require_approval", "justification": "需审批" }
            ],
            "network_rules": [
                { "host": "api.openai.com", "protocol": "https", "decision": "allow" }
            ]
        }"#;
        let file = parse_policy_file(json).unwrap();
        assert_eq!(file.rules.len(), 2);
        assert_eq!(file.network_rules.len(), 1);
    }

    #[test]
    fn test_parse_policy_file_invalid_json() {
        assert!(parse_policy_file("{invalid").is_err());
    }

    #[test]
    fn test_parse_decision() {
        assert_eq!(parse_decision("allow").unwrap(), PolicyDecision::Allow);
        assert_eq!(parse_decision("require_approval").unwrap(), PolicyDecision::RequireApproval);
        assert_eq!(parse_decision("prompt").unwrap(), PolicyDecision::RequireApproval);
        assert_eq!(parse_decision("deny").unwrap(), PolicyDecision::Deny);
        assert_eq!(parse_decision("forbidden").unwrap(), PolicyDecision::Deny);
        assert!(parse_decision("maybe").is_err());
    }

    #[test]
    fn test_policy_file_to_policy() {
        let file = PolicyFile {
            rules: vec![
                PrefixRuleSpec {
                    prefix: vec!["git".to_string(), "status".to_string()],
                    decision: "allow".to_string(),
                    justification: None,
                },
                PrefixRuleSpec {
                    prefix: vec!["rm".to_string()],
                    decision: "deny".to_string(),
                    justification: Some("危险".to_string()),
                },
            ],
            network_rules: vec![NetworkRuleSpec {
                host: "api.openai.com".to_string(),
                protocol: "https".to_string(),
                decision: "allow".to_string(),
                justification: None,
            }],
        };
        let policy = policy_file_to_policy(&file).unwrap();
        assert_eq!(policy.rules_by_program.len(), 2);
        assert!(policy.rules_by_program.contains_key("git"));
        assert!(policy.rules_by_program.contains_key("rm"));
        assert_eq!(policy.network_rules.len(), 1);
    }

    #[test]
    fn test_policy_add_prefix_rule_empty_rejected() {
        let mut policy = Policy::empty();
        assert!(policy
            .add_prefix_rule(&[], PolicyDecision::Allow, None)
            .is_err());
    }

    #[test]
    fn test_policy_get_allowed_prefixes() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(&["git".to_string(), "status".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        policy
            .add_prefix_rule(&["rm".to_string()], PolicyDecision::Deny, None)
            .unwrap();
        policy
            .add_prefix_rule(&["ls".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        let allowed = policy.get_allowed_prefixes();
        assert_eq!(allowed.len(), 2);
        assert!(allowed.contains(&vec!["git".to_string(), "status".to_string()]));
        assert!(allowed.contains(&vec!["ls".to_string()]));
    }

    #[test]
    fn test_policy_compiled_network_domains() {
        let mut policy = Policy::empty();
        policy
            .add_network_rule("api.openai.com", NetworkRuleProtocol::Https, PolicyDecision::Allow, None)
            .unwrap();
        policy
            .add_network_rule("evil.com", NetworkRuleProtocol::Https, PolicyDecision::Deny, None)
            .unwrap();
        // 同 host 有 Allow 和 Deny：Deny 优先
        policy
            .add_network_rule("api.openai.com", NetworkRuleProtocol::Https, PolicyDecision::Deny, None)
            .unwrap();
        let (allowed, denied) = policy.compiled_network_domains();
        assert!(!allowed.contains(&"api.openai.com".to_string()));
        assert!(denied.contains(&"api.openai.com".to_string()));
        assert!(denied.contains(&"evil.com".to_string()));
    }

    #[test]
    fn test_policy_matches_for_command() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(&["git".to_string(), "status".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        // 匹配显式规则
        let matches = policy.matches_for_command(&cmd(&["git", "status"]), None);
        assert_eq!(matches.len(), 1);
        assert!(matches[0].is_policy_match());
        // 无匹配 + 无 fallback → 空
        let no_match = policy.matches_for_command(&cmd(&["npm", "install"]), None);
        assert!(no_match.is_empty());
        // 无匹配 + fallback → 启发式匹配
        let heuristic = policy.matches_for_command(
            &cmd(&["npm", "install"]),
            Some(&deny_heuristic),
        );
        assert_eq!(heuristic.len(), 1);
        assert!(!heuristic[0].is_policy_match());
        assert_eq!(heuristic[0].decision(), PolicyDecision::Deny);
    }

    #[test]
    fn test_policy_merge_overlay() {
        let mut base = Policy::empty();
        base.add_prefix_rule(&["git".to_string(), "status".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        let mut overlay = Policy::empty();
        overlay
            .add_prefix_rule(&["git".to_string(), "push".to_string()], PolicyDecision::RequireApproval, None)
            .unwrap();
        let merged = base.merge_overlay(&overlay);
        // 同 program 的规则应合并
        assert_eq!(merged.rules_by_program.get("git").unwrap().len(), 2);
    }

    #[test]
    fn test_merge_layered_policies_all_layers() {
        let enterprise = PolicyFile {
            rules: vec![PrefixRuleSpec {
                prefix: vec!["ls".to_string()],
                decision: "allow".to_string(),
                justification: None,
            }],
            network_rules: vec![],
        };
        let user = PolicyFile {
            rules: vec![PrefixRuleSpec {
                prefix: vec!["git".to_string(), "status".to_string()],
                decision: "allow".to_string(),
                justification: None,
            }],
            network_rules: vec![],
        };
        let project = PolicyFile {
            rules: vec![PrefixRuleSpec {
                prefix: vec!["rm".to_string()],
                decision: "deny".to_string(),
                justification: None,
            }],
            network_rules: vec![],
        };
        let sources = LayeredPolicySources {
            enterprise: Some(enterprise),
            user: Some(user),
            project: Some(project),
        };
        let policy = merge_layered_policies(&sources, &MergeOptions::default()).unwrap();
        assert!(policy.rules_by_program.contains_key("ls"));
        assert!(policy.rules_by_program.contains_key("git"));
        assert!(policy.rules_by_program.contains_key("rm"));
    }

    #[test]
    fn test_merge_layered_policies_ignore_user_and_project() {
        let enterprise = PolicyFile {
            rules: vec![PrefixRuleSpec {
                prefix: vec!["ls".to_string()],
                decision: "allow".to_string(),
                justification: None,
            }],
            network_rules: vec![],
        };
        let user = PolicyFile {
            rules: vec![PrefixRuleSpec {
                prefix: vec!["git".to_string()],
                decision: "allow".to_string(),
                justification: None,
            }],
            network_rules: vec![],
        };
        let sources = LayeredPolicySources {
            enterprise: Some(enterprise),
            user: Some(user),
            project: None,
        };
        let options = MergeOptions {
            ignore_user_and_project_exec_policy_rules: true,
        };
        let policy = merge_layered_policies(&sources, &options).unwrap();
        // 仅 enterprise 生效
        assert!(policy.rules_by_program.contains_key("ls"));
        assert!(!policy.rules_by_program.contains_key("git"));
    }

    #[test]
    fn test_create_exec_approval_allow_bypass_sandbox() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(&["git".to_string(), "status".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["git", "status"]),
            ApprovalPolicyForExec::AllowApproval,
            &deny_heuristic,
        );
        match req {
            ExecApprovalRequirement::Skip { bypass_sandbox } => assert!(bypass_sandbox),
            _ => panic!("expected Skip"),
        }
    }

    #[test]
    fn test_create_exec_approval_deny() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(&["rm".to_string()], PolicyDecision::Deny, Some("危险".to_string()))
            .unwrap();
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["rm", "-rf"]),
            ApprovalPolicyForExec::AllowApproval,
            &allow_heuristic,
        );
        match req {
            ExecApprovalRequirement::Forbidden { reason } => {
                assert_eq!(reason, "危险");
            }
            _ => panic!("expected Forbidden"),
        }
    }

    #[test]
    fn test_create_exec_approval_needs_approval() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(
                &["git".to_string(), "push".to_string()],
                PolicyDecision::RequireApproval,
                Some("需审批".to_string()),
            )
            .unwrap();
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["git", "push", "origin"]),
            ApprovalPolicyForExec::AllowApproval,
            &deny_heuristic,
        );
        match req {
            ExecApprovalRequirement::NeedsApproval { reason } => {
                assert_eq!(reason, "需审批");
            }
            _ => panic!("expected NeedsApproval"),
        }
    }

    #[test]
    fn test_create_exec_approval_never_policy_rejects_prompt() {
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(
                &["git".to_string(), "push".to_string()],
                PolicyDecision::RequireApproval,
                None,
            )
            .unwrap();
        // Never 策略下，需审批的命令直接 Forbidden
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["git", "push"]),
            ApprovalPolicyForExec::Never,
            &allow_heuristic,
        );
        assert!(matches!(req, ExecApprovalRequirement::Forbidden { .. }));
    }

    #[test]
    fn test_create_exec_approval_no_match_uses_heuristic() {
        let policy = Policy::empty();
        // 无匹配规则，heuristic 返回 Deny
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["unknown", "cmd"]),
            ApprovalPolicyForExec::AllowApproval,
            &deny_heuristic,
        );
        match req {
            ExecApprovalRequirement::Forbidden { .. } => {}
            _ => panic!("expected Forbidden from heuristic"),
        }
    }

    #[test]
    fn test_create_exec_approval_allow_with_heuristic_fallback_mixed() {
        // 显式 Allow + 启发式 Deny：取最大（Deny）
        let mut policy = Policy::empty();
        policy
            .add_prefix_rule(&["git".to_string(), "status".to_string()], PolicyDecision::Allow, None)
            .unwrap();
        let req = create_exec_approval_requirement_for_command(
            &policy,
            &cmd(&["git", "status"]),
            ApprovalPolicyForExec::AllowApproval,
            &deny_heuristic,
        );
        // 显式 Allow 规则匹配，heuristic 不触发（已有显式匹配）
        match req {
            ExecApprovalRequirement::Skip { bypass_sandbox } => assert!(bypass_sandbox),
            _ => panic!("expected Skip"),
        }
    }
}