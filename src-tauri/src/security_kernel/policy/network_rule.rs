//! ═══════════════════════════════════════════════════════════════════════════
//! network_rule - 网络规则定义模块
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

use super::decision::PolicyDecision;

// ── 网络协议类型 ────────────────────────────────────────────────────────────────

/// 网络协议类型。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRuleProtocol {
    Http,
    Https,
    Socks5Tcp,
    Socks5Udp,
}

impl NetworkRuleProtocol {
    /// 从字符串解析协议。
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "http" => Ok(Self::Http),
            "https" | "https_connect" | "http-connect" => Ok(Self::Https),
            "socks5_tcp" => Ok(Self::Socks5Tcp),
            "socks5_udp" => Ok(Self::Socks5Udp),
            other => Err(format!(
                "network_rule 协议必须是 http, https, socks5_tcp, socks5_udp 之一 (收到 {other})"
            )),
        }
    }

    /// 返回策略字符串表示。
    pub fn as_policy_string(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5Tcp => "socks5_tcp",
            Self::Socks5Udp => "socks5_udp",
        }
    }
}

// ── 网络访问规则 ────────────────────────────────────────────────────────────────

/// 网络访问规则。
///
/// 设计要点：
/// - 每条 NetworkRule 绑定 (host, protocol) → Decision
/// - host 必须是规范化的 hostname 或 IP literal（不含 scheme/path/port，小写）
/// - protocol 支持 http/https/socks5_tcp/socks5_udp
/// - compiled_network_domains() 聚合所有 network_rules 为 (allowed, denied) 两个列表
///
/// 决策语义：
/// - Allow：允许访问此 host（若同 host 有 Deny 规则，Deny 优先，从 allowed 移除）
/// - Deny：禁止访问此 host（若同 host 有 Allow 规则，Deny 优先，从 allowed 移除）
/// - RequireApproval：需审批（不进入 allowed/denied 列表）
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkRule {
    pub host: String,
    pub protocol: NetworkRuleProtocol,
    pub decision: PolicyDecision,
    /// 规则说明（可选）。
    pub justification: Option<String>,
}

/// 规范化 network_rule host：
/// - 去除前后空白
/// - 拒绝空 host
/// - 拒绝含 scheme (`://`) / path (`/`) / query (`?`) / fragment (`#`) 的输入
/// - 支持 IPv6 字面量（`[::1]` 或 `[::1]:443`）
/// - 支持 host:port（剥离 port）
/// - 去除末尾 `.`，转小写
/// - 拒绝通配符 `*` 和空白字符
pub fn normalize_network_rule_host(raw: &str) -> Result<String, String> {
    let mut host = raw.trim();
    if host.is_empty() {
        return Err("network_rule host 不能为空".to_string());
    }
    if host.contains("://") || host.contains('/') || host.contains('?') || host.contains('#') {
        return Err(
            "network_rule host 必须是 hostname 或 IP 字面量（不含 scheme 或 path）"
                .to_string(),
        );
    }

    // IPv6 字面量处理：[::1] 或 [::1]:443
    if let Some(stripped) = host.strip_prefix('[') {
        let Some((inside, rest)) = stripped.split_once(']') else {
            return Err("network_rule host 包含无效的方括号 IPv6 字面量".to_string());
        };
        let port_ok = rest
            .strip_prefix(':')
            .is_some_and(|port| !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()));
        if !rest.is_empty() && !port_ok {
            return Err(format!(
                "network_rule host 包含不支持的后缀: {raw}"
            ));
        }
        host = inside;
    } else if host.matches(':').count() == 1 {
        // host:port 形式（非 IPv6）
        if let Some((candidate, port)) = host.rsplit_once(':') {
            if !candidate.is_empty()
                && !port.is_empty()
                && port.chars().all(|c| c.is_ascii_digit())
            {
                host = candidate;
            }
        }
    }

    let normalized = host.trim_end_matches('.').trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("network_rule host 不能为空".to_string());
    }
    if normalized.contains('*') {
        return Err(
            "network_rule host 必须是特定主机；不允许通配符".to_string(),
        );
    }
    if normalized.chars().any(char::is_whitespace) {
        return Err("network_rule host 不能包含空白字符".to_string());
    }

    Ok(normalized)
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_parse_valid() {
        assert_eq!(NetworkRuleProtocol::parse("http").unwrap(), NetworkRuleProtocol::Http);
        assert_eq!(NetworkRuleProtocol::parse("https").unwrap(), NetworkRuleProtocol::Https);
        assert_eq!(
            NetworkRuleProtocol::parse("https_connect").unwrap(),
            NetworkRuleProtocol::Https
        );
        assert_eq!(
            NetworkRuleProtocol::parse("socks5_tcp").unwrap(),
            NetworkRuleProtocol::Socks5Tcp
        );
        assert_eq!(
            NetworkRuleProtocol::parse("socks5_udp").unwrap(),
            NetworkRuleProtocol::Socks5Udp
        );
    }

    #[test]
    fn test_protocol_parse_invalid() {
        assert!(NetworkRuleProtocol::parse("ftp").is_err());
        assert!(NetworkRuleProtocol::parse("").is_err());
    }

    #[test]
    fn test_protocol_as_policy_string() {
        assert_eq!(NetworkRuleProtocol::Http.as_policy_string(), "http");
        assert_eq!(NetworkRuleProtocol::Https.as_policy_string(), "https");
        assert_eq!(NetworkRuleProtocol::Socks5Tcp.as_policy_string(), "socks5_tcp");
        assert_eq!(NetworkRuleProtocol::Socks5Udp.as_policy_string(), "socks5_udp");
    }

    #[test]
    fn test_normalize_simple_host() {
        assert_eq!(normalize_network_rule_host("example.com").unwrap(), "example.com");
        assert_eq!(normalize_network_rule_host("EXAMPLE.COM").unwrap(), "example.com");
        assert_eq!(normalize_network_rule_host("  example.com  ").unwrap(), "example.com");
    }

    #[test]
    fn test_normalize_host_with_port() {
        assert_eq!(normalize_network_rule_host("example.com:443").unwrap(), "example.com");
        assert_eq!(normalize_network_rule_host("localhost:8080").unwrap(), "localhost");
    }

    #[test]
    fn test_normalize_host_trailing_dot() {
        assert_eq!(normalize_network_rule_host("example.com.").unwrap(), "example.com");
    }

    #[test]
    fn test_normalize_ipv6_literal() {
        assert_eq!(normalize_network_rule_host("[::1]").unwrap(), "::1");
        assert_eq!(normalize_network_rule_host("[::1]:443").unwrap(), "::1");
        assert_eq!(
            normalize_network_rule_host("[2001:db8::1]").unwrap(),
            "2001:db8::1"
        );
    }

    #[test]
    fn test_normalize_reject_empty() {
        assert!(normalize_network_rule_host("").is_err());
        assert!(normalize_network_rule_host("   ").is_err());
    }

    #[test]
    fn test_normalize_reject_scheme() {
        assert!(normalize_network_rule_host("https://example.com").is_err());
        assert!(normalize_network_rule_host("http://localhost").is_err());
    }

    #[test]
    fn test_normalize_reject_path() {
        assert!(normalize_network_rule_host("example.com/path").is_err());
        assert!(normalize_network_rule_host("example.com?q=1").is_err());
        assert!(normalize_network_rule_host("example.com#frag").is_err());
    }

    #[test]
    fn test_normalize_reject_wildcard() {
        assert!(normalize_network_rule_host("*.example.com").is_err());
        assert!(normalize_network_rule_host("*").is_err());
    }

    #[test]
    fn test_normalize_reject_whitespace_in_host() {
        assert!(normalize_network_rule_host("example .com").is_err());
    }

    #[test]
    fn test_normalize_reject_invalid_ipv6_brackets() {
        assert!(normalize_network_rule_host("[::1").is_err());
        assert!(normalize_network_rule_host("[::1]:abc").is_err());
    }

    #[test]
    fn test_network_rule_construction() {
        let rule = NetworkRule {
            host: "api.openai.com".to_string(),
            protocol: NetworkRuleProtocol::Https,
            decision: PolicyDecision::Allow,
            justification: Some("允许 OpenAI API".to_string()),
        };
        assert_eq!(rule.host, "api.openai.com");
        assert_eq!(rule.protocol, NetworkRuleProtocol::Https);
        assert_eq!(rule.decision, PolicyDecision::Allow);
    }
}