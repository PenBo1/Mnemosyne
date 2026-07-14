// ExecPolicy 规则求值器 —— 按 priority 降序遍历规则，返回首个匹配的决策。
//
// 匹配语义：
// - 命令规则：token 前缀匹配（pattern 的每个 token 须等于 command 对应 token）
// - 路径规则：字符串前缀匹配（path.starts_with(pattern)，大小写敏感）
// - 网络规则：host 匹配（精确或 *.example.com 通配）+ protocol 精确匹配
// - 无规则匹配时返回 default_decision

use serde::{Deserialize, Serialize};

use super::types::{ExecPolicy, NetworkProtocol, PolicyDecision, PrefixRule};

/// 规则求值结果 —— 包含决策与匹配的规则索引（用于审计）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    pub decision: PolicyDecision,
    /// 匹配到的规则索引（无匹配时为 None）。
    pub matched_index: Option<usize>,
}

impl Evaluation {
    pub fn matched(decision: PolicyDecision, index: usize) -> Self {
        Self {
            decision,
            matched_index: Some(index),
        }
    }

    pub fn default_decision(decision: PolicyDecision) -> Self {
        Self {
            decision,
            matched_index: None,
        }
    }
}

impl ExecPolicy {
    /// 评估命令 —— 按 priority 降序遍历 command_rules，返回首个匹配的决策。
    pub fn evaluate_command(&self, command: &str) -> Evaluation {
        let mut rules: Vec<(usize, &PrefixRule)> =
            self.command_rules.iter().enumerate().collect();
        // priority 降序（稳定排序保留同 priority 的声明顺序）
        rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        for (idx, rule) in rules {
            if command_tokens_match(&rule.pattern, command) {
                return Evaluation::matched(rule.decision, idx);
            }
        }
        Evaluation::default_decision(self.default_decision)
    }

    /// 评估路径 —— 按 priority 降序遍历 path_rules，返回首个匹配的决策。
    pub fn evaluate_path(&self, path: &str) -> Evaluation {
        let normalized = normalize_path(path);
        let mut rules: Vec<(usize, &PrefixRule)> =
            self.path_rules.iter().enumerate().collect();
        rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        for (idx, rule) in rules {
            let pat = normalize_path(&rule.pattern);
            if normalized.starts_with(&pat) {
                return Evaluation::matched(rule.decision, idx);
            }
        }
        Evaluation::default_decision(self.default_decision)
    }

    /// 评估网络请求 —— 按 priority 降序遍历 network_rules，返回首个匹配的决策。
    ///
    /// host 匹配：精确（大小写不敏感）或 `*.example.com` 通配子域。
    /// protocol 匹配：精确枚举相等。
    pub fn evaluate_network(&self, host: &str, protocol: NetworkProtocol) -> Evaluation {
        let host_lower = host.to_lowercase();
        let mut rules: Vec<(usize, &super::types::NetworkRule)> =
            self.network_rules.iter().enumerate().collect();
        rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        for (idx, rule) in rules {
            if rule.protocol != protocol {
                continue;
            }
            let rule_host = rule.host.to_lowercase();
            if rule_host == "*" || rule_host == host_lower {
                return Evaluation::matched(rule.decision, idx);
            }
            // *.example.com 通配：匹配 example.com 的任意子域
            if let Some(suffix) = rule_host.strip_prefix("*.") {
                if host_lower == suffix || host_lower.ends_with(&format!(".{}", suffix)) {
                    return Evaluation::matched(rule.decision, idx);
                }
            }
        }
        Evaluation::default_decision(self.default_decision)
    }
}

/// 命令 token 前缀匹配。
///
/// pattern 和 command 均按空白分词。若 command 的 token 数 >= pattern 的 token 数，
/// 且 pattern 的每个 token 等于 command 对应位置 token（大小写不敏感），则匹配。
///
/// 示例：
/// - pattern "git status" 匹配 "git status --short" ✓
/// - pattern "git" 匹配 "git log" ✓
/// - pattern "git" 不匹配 "github"（不同 token）✓
fn command_tokens_match(pattern: &str, command: &str) -> bool {
    let pat_tokens: Vec<&str> = pattern.split_whitespace().collect();
    if pat_tokens.is_empty() {
        return false;
    }
    let cmd_tokens: Vec<&str> = command.split_whitespace().collect();
    if cmd_tokens.len() < pat_tokens.len() {
        return false;
    }
    pat_tokens
        .iter()
        .zip(cmd_tokens.iter())
        .all(|(p, c)| p.eq_ignore_ascii_case(c))
}

/// 路径规范化 —— 统一分隔符为 '/'，去除末尾 '/'，便于前缀匹配。
fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 从 URL 中提取 host（用于网络规则评估的辅助函数）。
///
/// 返回 (host, protocol)。解析失败时返回空 host + Http 默认协议。
pub fn extract_host_and_protocol(url: &str) -> (String, NetworkProtocol) {
    let lower = url.to_lowercase();
    let protocol = if lower.starts_with("https://") {
        NetworkProtocol::Https
    } else if lower.starts_with("http://") {
        NetworkProtocol::Http
    } else if lower.starts_with("socks5://") {
        // socks5 默认走 tcp
        NetworkProtocol::Socks5Tcp
    } else {
        NetworkProtocol::Http
    };

    // 去除 scheme 前缀
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .or_else(|| lower.strip_prefix("socks5://"))
        .unwrap_or(url);

    // host 在第一个 '/' 或 ':' 之前
    let host: String = rest
        .split(|c| c == '/' || c == ':')
        .next()
        .unwrap_or("")
        .to_string();

    (host, protocol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sandbox::execpolicy::builtin::default_policy;

    #[test]
    fn command_prefix_match() {
        assert!(command_tokens_match("git status", "git status --short"));
        assert!(command_tokens_match("git", "git log"));
        assert!(!command_tokens_match("git", "github"));
        assert!(!command_tokens_match("rm -rf", "rm file.txt"));
    }

    #[test]
    fn evaluate_command_default() {
        let policy = default_policy();
        // git status 应被允许
        let ev = policy.evaluate_command("git status");
        assert_eq!(ev.decision, PolicyDecision::Allow);
        // rm -rf 应被拒绝
        let ev = policy.evaluate_command("rm -rf /");
        assert_eq!(ev.decision, PolicyDecision::Deny);
    }

    #[test]
    fn evaluate_path_protected() {
        let policy = default_policy();
        // .env 路径应被拒绝
        let ev = policy.evaluate_path("/home/user/project/.env");
        assert_eq!(ev.decision, PolicyDecision::Deny);
    }

    #[test]
    fn evaluate_network_allowed() {
        let policy = default_policy();
        let ev = policy.evaluate_network("api.openai.com", NetworkProtocol::Https);
        assert_eq!(ev.decision, PolicyDecision::Allow);
    }

    #[test]
    fn evaluate_network_metadata_denied() {
        let policy = default_policy();
        let ev = policy.evaluate_network("169.254.169.254", NetworkProtocol::Http);
        assert_eq!(ev.decision, PolicyDecision::Deny);
    }

    #[test]
    fn extract_host_from_url() {
        let (host, proto) = extract_host_and_protocol("https://api.openai.com/v1/chat");
        assert_eq!(host, "api.openai.com");
        assert_eq!(proto, NetworkProtocol::Https);

        let (host, proto) = extract_host_and_protocol("http://169.254.169.254/latest");
        assert_eq!(host, "169.254.169.254");
        assert_eq!(proto, NetworkProtocol::Http);
    }
}
