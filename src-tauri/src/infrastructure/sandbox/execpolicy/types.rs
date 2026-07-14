// ExecPolicy 类型定义 —— 精细化沙箱策略。
//
// 设计要点：
// - PolicyDecision: Allow / Deny / AskUser（三态，支持交互式审批）
// - 三类规则：Command（命令前缀）、Path（路径前缀）、Network（主机+协议）
// - 每条规则带 priority（降序匹配）和可选 justification

use serde::{Deserialize, Serialize};

/// 策略决策结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyDecision {
    /// 允许执行。
    Allow,
    /// 拒绝执行。
    Deny,
    /// 需要用户确认（前端弹窗审批）。
    AskUser,
}

impl PolicyDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::AskUser => "ask",
        }
    }

    /// 从 DSL 关键字解析。
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            "ask" => Some(Self::AskUser),
            _ => None,
        }
    }

    /// 在同步校验路径下（validate_command 返回 bool）的保守映射：
    /// Allow → true，Deny/AskUser → false（无法同步交互，保守拒绝）。
    pub fn is_allowed_sync(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// 规则类型 —— 区分命令前缀、路径前缀、网络规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleKind {
    Command,
    Path,
    Network,
}

/// 网络协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkProtocol {
    Http,
    Https,
    Socks5Tcp,
    Socks5Udp,
}

impl NetworkProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::Socks5Tcp => "socks5_tcp",
            Self::Socks5Udp => "socks5_udp",
        }
    }

    /// 从 DSL 关键字解析。
    pub fn from_keyword(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            "socks5_tcp" | "socks5tcp" => Some(Self::Socks5Tcp),
            "socks5_udp" | "socks5udp" => Some(Self::Socks5Udp),
            _ => None,
        }
    }
}

/// 前缀匹配规则（命令 / 路径）。
///
/// 命令匹配采用 token 前缀：pattern 按空白分词后，command 的前 N 个 token 逐一匹配。
/// 路径匹配采用字符串前缀：path.starts_with(pattern)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrefixRule {
    pub kind: RuleKind,
    /// 匹配模式（命令 token 序列或路径前缀字符串）。
    pub pattern: String,
    pub decision: PolicyDecision,
    /// 优先级，数字越大越先评估；默认 0。
    #[serde(default)]
    pub priority: i32,
    /// 可选的规则说明。
    pub justification: Option<String>,
}

/// 网络规则 —— 按主机名 + 协议匹配。
///
/// host 支持通配符前缀 `*.`（如 `*.example.com` 匹配 `api.example.com`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRule {
    pub host: String,
    pub protocol: NetworkProtocol,
    pub decision: PolicyDecision,
    #[serde(default)]
    pub priority: i32,
    pub justification: Option<String>,
}

/// ExecPolicy 完整策略 —— 由 default_decision + 三类规则列表组成。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPolicy {
    /// 无规则匹配时的默认决策。
    pub default_decision: PolicyDecision,
    pub command_rules: Vec<PrefixRule>,
    pub path_rules: Vec<PrefixRule>,
    pub network_rules: Vec<NetworkRule>,
}

impl Default for ExecPolicy {
    fn default() -> Self {
        super::builtin::default_policy()
    }
}
