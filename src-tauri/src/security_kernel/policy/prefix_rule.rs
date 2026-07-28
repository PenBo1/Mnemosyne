//! ═══════════════════════════════════════════════════════════════════════════
//! prefix_rule - 前缀匹配规则模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::decision::PolicyDecision;

// ── 模式令牌 ────────────────────────────────────────────────────────────────

/// 单个命令 token 的匹配模式。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatternToken {
    /// 精确匹配单个字符串。
    Single(String),
    /// 匹配候选列表中的任意一个。
    Alts(Vec<String>),
}

impl PatternToken {
    /// 判断此 token 是否匹配给定的命令 token。
    pub fn matches(&self, token: &str) -> bool {
        match self {
            Self::Single(expected) => expected == token,
            Self::Alts(alternatives) => alternatives.iter().any(|alt| alt == token),
        }
    }

    /// 返回所有候选（Single 返回单元素切片）。
    pub fn alternatives(&self) -> &[String] {
        match self {
            Self::Single(expected) => std::slice::from_ref(expected),
            Self::Alts(alternatives) => alternatives,
        }
    }
}

// ── 前缀匹配模式 ────────────────────────────────────────────────────────────────

/// 前缀匹配模式：first（固定首 token）+ rest（后续 PatternToken 列表）。
///
/// first 单独存储是因为 rules_by_program 用 first 作为 key 进行快速查找。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrefixPattern {
    pub first: Arc<str>,
    pub rest: Arc<[PatternToken]>,
}

impl PrefixPattern {
    /// 判断 cmd 是否匹配此前缀模式。
    ///
    /// 返回 Some(matched_prefix) 表示匹配，matched_prefix 是 cmd 中被模式覆盖的前缀部分；
    /// 返回 None 表示不匹配。
    pub fn matches_prefix(&self, cmd: &[String]) -> Option<Vec<String>> {
        let pattern_length = self.rest.len() + 1;
        if cmd.len() < pattern_length || cmd[0] != self.first.as_ref() {
            return None;
        }
        for (pattern_token, cmd_token) in self.rest.iter().zip(&cmd[1..pattern_length]) {
            if !pattern_token.matches(cmd_token) {
                return None;
            }
        }
        Some(cmd[..pattern_length].to_vec())
    }
}

// ── 前缀规则 ────────────────────────────────────────────────────────────────

/// 命令前缀规则。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrefixRule {
    pub pattern: PrefixPattern,
    pub decision: PolicyDecision,
    /// 规则存在的原因（可选），用于提示用户或拒绝消息。
    pub justification: Option<String>,
}

// ── 规则匹配结果 ────────────────────────────────────────────────────────────────

/// 规则匹配结果。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleMatch {
    /// 前缀规则匹配。
    PrefixRuleMatch {
        #[serde(rename = "matchedPrefix")]
        matched_prefix: Vec<String>,
        decision: PolicyDecision,
        /// 规则说明。
        #[serde(skip_serializing_if = "Option::is_none")]
        justification: Option<String>,
    },
    /// 启发式回退匹配（无显式规则匹配时，由 heuristics_fallback 给出决策）。
    HeuristicsRuleMatch {
        command: Vec<String>,
        decision: PolicyDecision,
    },
}

impl RuleMatch {
    /// 获取此匹配的决策。
    pub fn decision(&self) -> PolicyDecision {
        match self {
            Self::PrefixRuleMatch { decision, .. } => *decision,
            Self::HeuristicsRuleMatch { decision, .. } => *decision,
        }
    }

    /// 是否为显式规则匹配（非启发式）。
    pub fn is_policy_match(&self) -> bool {
        matches!(self, Self::PrefixRuleMatch { .. })
    }
}

// ── 规则 Trait ────────────────────────────────────────────────────────────────

/// 规则 trait：所有规则类型的统一接口。
pub trait Rule: Any + Debug + Send + Sync {
    /// 返回首 token（用作 rules_by_program 的 key）。
    fn program(&self) -> &str;

    /// 判断此规则是否匹配 cmd，匹配则返回 RuleMatch。
    fn matches(&self, cmd: &[String]) -> Option<RuleMatch>;

    /// 转为 Any（用于 downcast）。
    fn as_any(&self) -> &dyn Any;
}

/// 规则引用类型（Arc<dyn Rule>）。
pub type RuleRef = Arc<dyn Rule>;

impl Rule for PrefixRule {
    fn program(&self) -> &str {
        self.pattern.first.as_ref()
    }

    fn matches(&self, cmd: &[String]) -> Option<RuleMatch> {
        self.pattern.matches_prefix(cmd).map(|matched_prefix| {
            RuleMatch::PrefixRuleMatch {
                matched_prefix,
                decision: self.decision,
                justification: self.justification.clone(),
            }
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_pattern_token_single_match() {
        let token = PatternToken::Single("push".to_string());
        assert!(token.matches("push"));
        assert!(!token.matches("pull"));
    }

    #[test]
    fn test_pattern_token_alts_match() {
        let token = PatternToken::Alts(vec!["push".to_string(), "pull".to_string()]);
        assert!(token.matches("push"));
        assert!(token.matches("pull"));
        assert!(!token.matches("fetch"));
        assert_eq!(token.alternatives().len(), 2);
    }

    #[test]
    fn test_prefix_pattern_exact_match() {
        let pattern = PrefixPattern {
            first: Arc::from("git"),
            rest: Arc::from([
                PatternToken::Single("push".to_string()),
            ]),
        };
        // 完全匹配
        assert!(pattern.matches_prefix(&cmd(&["git", "push"])).is_some());
        // cmd 更长仍匹配（前缀匹配）
        let matched = pattern.matches_prefix(&cmd(&["git", "push", "origin"])).unwrap();
        assert_eq!(matched, cmd(&["git", "push"]));
        // 首 token 不匹配
        assert!(pattern.matches_prefix(&cmd(&["npm", "push"])).is_none());
        // 长度不足
        assert!(pattern.matches_prefix(&cmd(&["git"])).is_none());
        // 第二 token 不匹配
        assert!(pattern.matches_prefix(&cmd(&["git", "pull"])).is_none());
    }

    #[test]
    fn test_prefix_pattern_with_alts() {
        let pattern = PrefixPattern {
            first: Arc::from("git"),
            rest: Arc::from([
                PatternToken::Alts(vec!["push".to_string(), "pull".to_string()]),
            ]),
        };
        assert!(pattern.matches_prefix(&cmd(&["git", "push"])).is_some());
        assert!(pattern.matches_prefix(&cmd(&["git", "pull"])).is_some());
        assert!(pattern.matches_prefix(&cmd(&["git", "fetch"])).is_none());
    }

    #[test]
    fn test_prefix_rule_matches() {
        let rule = PrefixRule {
            pattern: PrefixPattern {
                first: Arc::from("git"),
                rest: Arc::from([PatternToken::Single("status".to_string())]),
            },
            decision: PolicyDecision::Allow,
            justification: Some("安全的只读 git 命令".to_string()),
        };
        let matched = rule.matches(&cmd(&["git", "status"])).unwrap();
        match matched {
            RuleMatch::PrefixRuleMatch {
                matched_prefix,
                decision,
                justification,
            } => {
                assert_eq!(matched_prefix, cmd(&["git", "status"]));
                assert_eq!(decision, PolicyDecision::Allow);
                assert!(justification.is_some());
            }
            _ => panic!("expected PrefixRuleMatch"),
        }
    }

    #[test]
    fn test_rule_trait_program() {
        let rule = PrefixRule {
            pattern: PrefixPattern {
                first: Arc::from("npm"),
                rest: Arc::from([PatternToken::Single("install".to_string())]),
            },
            decision: PolicyDecision::RequireApproval,
            justification: None,
        };
        let rule_ref: RuleRef = Arc::new(rule);
        assert_eq!(rule_ref.program(), "npm");
    }

    #[test]
    fn test_rule_match_decision() {
        let m1 = RuleMatch::PrefixRuleMatch {
            matched_prefix: cmd(&["ls"]),
            decision: PolicyDecision::Allow,
            justification: None,
        };
        assert_eq!(m1.decision(), PolicyDecision::Allow);
        assert!(m1.is_policy_match());

        let m2 = RuleMatch::HeuristicsRuleMatch {
            command: cmd(&["rm", "-rf"]),
            decision: PolicyDecision::Deny,
        };
        assert_eq!(m2.decision(), PolicyDecision::Deny);
        assert!(!m2.is_policy_match());
    }
}