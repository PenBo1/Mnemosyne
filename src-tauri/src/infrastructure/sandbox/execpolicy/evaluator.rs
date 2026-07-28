//! ═══════════════════════════════════════════════════════════════════════════
//! 规则求值器 - 按 priority 降序遍历规则
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

use super::types::{ExecPolicy, NetworkProtocol, PolicyDecision};

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
    /// 规范化：将三类规则按 priority 降序（稳定）排序。
    ///
    /// 在策略加载/更新时调用一次，使 evaluate_* 不必每次重新排序。
    /// 原实现每次评估都 `collect + sort_by`，对热路径有重复开销。
    pub fn normalize(&mut self) {
        self.command_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.path_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.network_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 评估命令 —— 按 priority 降序遍历 command_rules，返回首个匹配的决策。
    ///
    /// 规则须在加载时已通过 `normalize()` 按 priority 降序排列，
    /// 此处直接顺序遍历，避免每次评估重新 sort。
    pub fn evaluate_command(&self, command: &str) -> Evaluation {
        for (idx, rule) in self.command_rules.iter().enumerate() {
            if command_tokens_match(&rule.pattern, command) {
                return Evaluation::matched(rule.decision, idx);
            }
        }
        Evaluation::default_decision(self.default_decision)
    }

    /// 评估路径 —— 按 priority 降序遍历 path_rules，返回首个匹配的决策。
    pub fn evaluate_path(&self, path: &str) -> Evaluation {
        let normalized = normalize_path(path);
        for (idx, rule) in self.path_rules.iter().enumerate() {
            let pat = normalize_path(&rule.pattern);
            // 三种匹配：前缀（目录树）/ 后缀（路径末尾的文件名，如 /.env）/ 精确（纯文件名）
            if normalized.starts_with(&pat)
                || normalized.ends_with(&format!("/{}", pat))
                || normalized == pat
            {
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
        for (idx, rule) in self.network_rules.iter().enumerate() {
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
///
/// 使用 `reqwest::Url::parse` 而非手写 split,正确处理 userinfo:
/// `https://user:pass@host/path` → host="host"(而非被误判为 "user")。
/// userinfo 本身由 SecurityKernel 的 url 校验层拦截,此处仅提取 host。
pub fn extract_host_and_protocol(url: &str) -> (String, NetworkProtocol) {
    let parsed = match reqwest::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return (String::new(), NetworkProtocol::Http),
    };
    let protocol = match parsed.scheme() {
        "https" => NetworkProtocol::Https,
        "socks5" => NetworkProtocol::Socks5Tcp,
        // http 及其他 scheme 统一按 Http 处理(对齐原 fallback 行为)
        _ => NetworkProtocol::Http,
    };
    let host = parsed.host_str().unwrap_or("").to_string();
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

    #[test]
    fn extract_host_ignores_userinfo() {
        // userinfo 不应被误判为 host(原 split 实现会返回 "user")
        let (host, proto) = extract_host_and_protocol("https://user:pass@api.openai.com/v1/chat");
        assert_eq!(host, "api.openai.com");
        assert_eq!(proto, NetworkProtocol::Https);
    }
}
