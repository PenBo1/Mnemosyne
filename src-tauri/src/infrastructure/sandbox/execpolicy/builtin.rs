// ExecPolicy 内置默认规则 —— 白名单 + 黑名单 + 路径保护。
//
// 设计原则：
// - 默认决策 AskUser（未知操作需确认，对齐零信任）
// - 常见开发命令白名单（git/cargo/npm/bun 只读操作）
// - 高风险命令黑名单（rm -rf/sudo/format/del 高优先级拒绝）
// - 系统目录与敏感文件路径保护（/etc/、/Windows/、.env、.git/）
// - 云元数据端点拒绝（169.254.169.254）

use super::types::{
    ExecPolicy, NetworkProtocol, NetworkRule, PolicyDecision, PrefixRule, RuleKind,
};

/// 构造内置默认 ExecPolicy。
///
/// 构造后调用 `normalize()` 按 priority 降序排序，使 evaluator 可直接顺序遍历，
/// 避免每次评估重新 sort（L18）。
pub fn default_policy() -> ExecPolicy {
    let mut policy = ExecPolicy {
        default_decision: PolicyDecision::AskUser,
        command_rules: default_command_rules(),
        path_rules: default_path_rules(),
        network_rules: default_network_rules(),
    };
    policy.normalize();
    policy
}

// ── 命令规则 ──
//
// priority 层级：
// - 200：系统破坏（rm -rf / sudo / format）—— 最高优先级拒绝
// - 150：权限提升 / 块设备
// - 100：安全只读命令白名单
// - 0：其他

fn default_command_rules() -> Vec<PrefixRule> {
    let deny_high = |pattern: &str| PrefixRule {
        kind: RuleKind::Command,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Deny,
        priority: 200,
        justification: Some("high-risk system destruction".to_string()),
    };

    let deny_med = |pattern: &str| PrefixRule {
        kind: RuleKind::Command,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Deny,
        priority: 150,
        justification: Some("privilege escalation or block device".to_string()),
    };

    let allow = |pattern: &str| PrefixRule {
        kind: RuleKind::Command,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Allow,
        priority: 100,
        justification: Some("safe read-only command".to_string()),
    };

    vec![
        // ── 高风险（priority 200）──
        deny_high("rm -rf"),
        deny_high("sudo"),
        deny_high("doas"),
        deny_high("format"),
        deny_high("mkfs"),
        deny_high("fdisk"),
        deny_high("dd"),
        deny_high("shutdown"),
        deny_high("reboot"),
        // ── 权限提升 / 块设备（priority 150）──
        deny_med("su"),
        deny_med("chmod"),
        deny_med("chown"),
        deny_med("chattr"),
        // ── 安全只读命令（priority 100）──
        allow("git status"),
        allow("git log"),
        allow("git diff"),
        allow("git show"),
        allow("git branch"),
        allow("git stash list"),
        allow("cargo check"),
        allow("cargo build"),
        allow("cargo test"),
        allow("cargo clippy"),
        allow("cargo fmt"),
        allow("cargo doc"),
        allow("npm run build"),
        allow("npm test"),
        allow("npm ci"),
        allow("bun run build"),
        allow("bun test"),
        allow("ls"),
        allow("cat"),
        allow("grep"),
        allow("rg"),
        allow("find"),
        allow("head"),
        allow("tail"),
        allow("wc"),
        allow("echo"),
        allow("pwd"),
        allow("which"),
        allow("node --version"),
        allow("python --version"),
    ]
}

// ── 路径规则 ──
//
// priority 层级：
// - 200：敏感文件（.env / .git/）—— 最高优先级
// - 150：系统目录
// - 100：临时目录白名单

fn default_path_rules() -> Vec<PrefixRule> {
    let deny_sensitive = |pattern: &str, just: &str| PrefixRule {
        kind: RuleKind::Path,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Deny,
        priority: 200,
        justification: Some(just.to_string()),
    };

    let deny_system = |pattern: &str| PrefixRule {
        kind: RuleKind::Path,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Deny,
        priority: 150,
        justification: Some("system directory".to_string()),
    };

    let allow = |pattern: &str| PrefixRule {
        kind: RuleKind::Path,
        pattern: pattern.to_string(),
        decision: PolicyDecision::Allow,
        priority: 100,
        justification: Some("temp/workspace directory".to_string()),
    };

    vec![
        // ── 敏感文件（priority 200）──
        deny_sensitive(".env", "environment secrets"),
        deny_sensitive(".env.local", "environment secrets"),
        deny_sensitive(".git", "version control metadata"),
        deny_sensitive(".ssh", "SSH keys"),
        deny_sensitive(".aws", "AWS credentials"),
        // ── 系统目录（priority 150）──
        deny_system("/etc/"),
        deny_system("/usr/bin/"),
        deny_system("/bin/"),
        deny_system("/sbin/"),
        deny_system("/boot/"),
        deny_system("/sys/"),
        deny_system("/proc/"),
        // Windows 系统目录
        deny_system("/Windows/System32/"),
        deny_system("/Windows/SysWOW64/"),
        // ── 临时目录白名单（priority 100）──
        allow("/tmp/"),
        allow("/var/folders/"),
        allow("/temp/"),
    ]
}

// ── 网络规则 ──
//
// priority 层级：
// - 200：云元数据端点 —— 最高优先级拒绝
// - 100：已知 LLM API 白名单

fn default_network_rules() -> Vec<NetworkRule> {
    let deny_metadata = |host: &str, proto: NetworkProtocol| NetworkRule {
        host: host.to_string(),
        protocol: proto,
        decision: PolicyDecision::Deny,
        priority: 200,
        justification: Some("cloud metadata endpoint".to_string()),
    };

    let allow_api = |host: &str, proto: NetworkProtocol| NetworkRule {
        host: host.to_string(),
        protocol: proto,
        decision: PolicyDecision::Allow,
        priority: 100,
        justification: Some("approved LLM API".to_string()),
    };

    vec![
        // ── 云元数据端点拒绝（priority 200）──
        deny_metadata("169.254.169.254", NetworkProtocol::Http),
        deny_metadata("169.254.169.254", NetworkProtocol::Https),
        deny_metadata("metadata.google.internal", NetworkProtocol::Http),
        deny_metadata("metadata.google.internal", NetworkProtocol::Https),
        deny_metadata("metadata.azure.com", NetworkProtocol::Http),
        deny_metadata("metadata.azure.com", NetworkProtocol::Https),
        // ── LLM API 白名单（priority 100）──
        allow_api("api.openai.com", NetworkProtocol::Https),
        allow_api("api.anthropic.com", NetworkProtocol::Https),
        allow_api("generativelanguage.googleapis.com", NetworkProtocol::Https),
        allow_api("api.deepseek.com", NetworkProtocol::Https),
        allow_api("openrouter.ai", NetworkProtocol::Https),
    ]
}
