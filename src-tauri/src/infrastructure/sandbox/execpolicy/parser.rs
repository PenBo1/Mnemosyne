// ExecPolicy DSL 解析器 —— 将文本配置解析为 ExecPolicy 结构。
//
// DSL 语法（简洁实用，不过度工程化）：
//
//   # 注释行
//   default allow|deny|ask
//
//   # 命令规则（token 前缀匹配）
//   allow|deny|ask command "git status"
//   deny command "rm -rf" @priority 100
//
//   # 路径规则（字符串前缀匹配）
//   deny path "/etc/"
//   deny path "**/.env"     # ** 匹配任意路径前缀
//
//   # 网络规则（host + protocol）
//   allow network "api.openai.com" https
//   deny network "169.254.169.254" http @priority 50
//
// 规则：
// - pattern 用双引号包裹（支持含空格的模式）
// - @priority N 为可选优先级（默认 0）
// - 路径模式中的 ** 前缀在评估时被去除（等价于"任意路径下匹配文件名"）
// - 空行和 # 开头的行被忽略

use crate::shared::error::AppError;

use super::types::{
    ExecPolicy, NetworkProtocol, NetworkRule, PolicyDecision, PrefixRule, RuleKind,
};

/// 解析 DSL 文本为 ExecPolicy。
///
/// 解析失败时返回 AppError，包含行号与原因（不静默跳过）。
pub fn parse(input: &str) -> Result<ExecPolicy, AppError> {
    let mut policy = ExecPolicy {
        default_decision: PolicyDecision::AskUser,
        command_rules: Vec::new(),
        path_rules: Vec::new(),
        network_rules: Vec::new(),
    };

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        // 空行 / 注释行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let tokens = tokenize(line);
        if tokens.is_empty() {
            continue;
        }

        match tokens[0].as_str() {
            "default" => parse_default(&tokens, &mut policy, line_no)?,
            "allow" | "deny" | "ask" => parse_rule(&tokens, &mut policy, line_no)?,
            _ => {
                return Err(AppError::invalid_input(format!(
                    "exec_policy.conf line {}: unknown keyword '{}' (expected 'default'/'allow'/'deny'/'ask')",
                    line_no + 1,
                    tokens[0]
                )));
            }
        }
    }

    // 解析完成后排序（按 priority 降序），使 evaluator 可直接顺序遍历，
    // 避免每次评估重新 sort（L18）
    policy.normalize();
    Ok(policy)
}

/// 将 ExecPolicy 序列化为 DSL 文本（用于持久化）。
pub fn serialize(policy: &ExecPolicy) -> String {
    let mut out = String::new();
    out.push_str("# ExecPolicy DSL —— 自动生成，请勿手动编辑格式\n");
    out.push_str(&format!("default {}\n\n", policy.default_decision.as_str()));

    if !policy.command_rules.is_empty() {
        out.push_str("# Command rules\n");
        for rule in &policy.command_rules {
            out.push_str(&serialize_prefix_rule(rule));
        }
        out.push('\n');
    }

    if !policy.path_rules.is_empty() {
        out.push_str("# Path rules\n");
        for rule in &policy.path_rules {
            out.push_str(&serialize_prefix_rule(rule));
        }
        out.push('\n');
    }

    if !policy.network_rules.is_empty() {
        out.push_str("# Network rules\n");
        for rule in &policy.network_rules {
            out.push_str(&serialize_network_rule(rule));
        }
    }

    out
}

// ── 内部解析函数 ──

fn parse_default(
    tokens: &[String],
    policy: &mut ExecPolicy,
    line_no: usize,
) -> Result<(), AppError> {
    if tokens.len() < 2 {
        return Err(AppError::invalid_input(format!(
            "exec_policy.conf line {}: 'default' requires a decision (allow/deny/ask)",
            line_no + 1
        )));
    }
    let decision = PolicyDecision::from_keyword(&tokens[1]).ok_or_else(|| {
        AppError::invalid_input(format!(
            "exec_policy.conf line {}: invalid default decision '{}' (expected allow/deny/ask)",
            line_no + 1,
            tokens[1]
        ))
    })?;
    policy.default_decision = decision;
    Ok(())
}

fn parse_rule(
    tokens: &[String],
    policy: &mut ExecPolicy,
    line_no: usize,
) -> Result<(), AppError> {
    let decision = PolicyDecision::from_keyword(&tokens[0]).unwrap();
    if tokens.len() < 3 {
        return Err(AppError::invalid_input(format!(
            "exec_policy.conf line {}: rule requires kind and pattern (e.g. `allow command \"git status\"`)",
            line_no + 1
        )));
    }

    let kind_str = &tokens[1];
    let (priority, pattern_start) = extract_priority(tokens, 2, line_no)?;

    match kind_str.as_str() {
        "command" => {
            let pattern = extract_quoted(&tokens[pattern_start..], line_no)?;
            policy.command_rules.push(PrefixRule {
                kind: RuleKind::Command,
                pattern,
                decision,
                priority,
                justification: None,
            });
        }
        "path" => {
            let pattern = extract_quoted(&tokens[pattern_start..], line_no)?;
            let cleaned = strip_glob_prefix(&pattern);
            policy.path_rules.push(PrefixRule {
                kind: RuleKind::Path,
                pattern: cleaned,
                decision,
                priority,
                justification: None,
            });
        }
        "network" => {
            // network "host" protocol
            let host = extract_quoted(&tokens[pattern_start..], line_no)?;
            let protocol_idx = pattern_start + 1;
            if protocol_idx >= tokens.len() {
                return Err(AppError::invalid_input(format!(
                    "exec_policy.conf line {}: network rule requires protocol (http/https/socks5_tcp/socks5_udp)",
                    line_no + 1
                )));
            }
            let protocol = NetworkProtocol::from_keyword(&tokens[protocol_idx]).ok_or_else(|| {
                AppError::invalid_input(format!(
                    "exec_policy.conf line {}: invalid protocol '{}' (expected http/https/socks5_tcp/socks5_udp)",
                    line_no + 1,
                    tokens[protocol_idx]
                ))
            })?;
            policy.network_rules.push(NetworkRule {
                host,
                protocol,
                decision,
                priority,
                justification: None,
            });
        }
        _ => {
            return Err(AppError::invalid_input(format!(
                "exec_policy.conf line {}: unknown rule kind '{}' (expected command/path/network)",
                line_no + 1,
                kind_str
            )));
        }
    }
    Ok(())
}

/// 从 token 切片中提取 @priority N（若存在），返回 (priority, pattern 起始索引)。
///
/// @priority 可出现在 pattern 之后或之前（实际仅支持 pattern 之后）。
/// 此函数扫描 tokens[start..] 查找 "@priority" 标记。
fn extract_priority(
    tokens: &[String],
    start: usize,
    line_no: usize,
) -> Result<(i32, usize), AppError> {
    // 查找 @priority 关键字
    for (i, tok) in tokens.iter().enumerate().skip(start) {
        if tok == "@priority" {
            if i + 1 >= tokens.len() {
                return Err(AppError::invalid_input(format!(
                    "exec_policy.conf line {}: '@priority' requires a numeric value",
                    line_no + 1
                )));
            }
            let val: i32 = tokens[i + 1].parse().map_err(|_| {
                AppError::invalid_input(format!(
                    "exec_policy.conf line {}: '@priority' value '{}' is not a valid integer",
                    line_no + 1,
                    tokens[i + 1]
                ))
            })?;
            return Ok((val, start));
        }
    }
    Ok((0, start))
}

/// 从 token 切片中提取引号包裹的 pattern。
///
/// tokens[start] 应为 "..." 形式的引号字符串（tokenize 已保留引号）。
fn extract_quoted(tokens: &[String], line_no: usize) -> Result<String, AppError> {
    if tokens.is_empty() {
        return Err(AppError::invalid_input(format!(
            "exec_policy.conf line {}: missing quoted pattern",
            line_no + 1
        )));
    }
    let raw = &tokens[0];
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        Ok(raw[1..raw.len() - 1].to_string())
    } else {
        Err(AppError::invalid_input(format!(
            "exec_policy.conf line {}: pattern must be double-quoted, got '{}'",
            line_no + 1,
            raw
        )))
    }
}

/// 去除路径模式的 `**/` 通配前缀（评估时作为"任意路径下匹配"处理）。
///
/// `**/.env` → `.env`（evaluate_path 中 starts_with(".env") 可匹配任意以 .env 结尾的路径）
/// `/etc/**` → `/etc/`
fn strip_glob_prefix(pattern: &str) -> String {
    if let Some(stripped) = pattern.strip_prefix("**/") {
        stripped.to_string()
    } else if let Some(stripped) = pattern.strip_prefix("**") {
        stripped.to_string()
    } else {
        pattern.to_string()
    }
}

/// 简单分词器 —— 按空白分割，但保留双引号内的内容为单个 token（含引号）。
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn serialize_prefix_rule(rule: &PrefixRule) -> String {
    let mut line = format!("{} {} \"{}\"", rule.decision.as_str(), kind_str(rule.kind), rule.pattern);
    if rule.priority != 0 {
        line.push_str(&format!(" @priority {}", rule.priority));
    }
    if let Some(j) = &rule.justification {
        line.push_str(&format!("  # {}", j));
    }
    line.push('\n');
    line
}

fn serialize_network_rule(rule: &NetworkRule) -> String {
    let mut line = format!(
        "{} network \"{}\" {}",
        rule.decision.as_str(),
        rule.host,
        rule.protocol.as_str()
    );
    if rule.priority != 0 {
        line.push_str(&format!(" @priority {}", rule.priority));
    }
    if let Some(j) = &rule.justification {
        line.push_str(&format!("  # {}", j));
    }
    line.push('\n');
    line
}

fn kind_str(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Command => "command",
        RuleKind::Path => "path",
        RuleKind::Network => "network",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_policy() {
        let input = r#"
# Test policy
default deny

allow command "git status"
deny command "rm -rf" @priority 100
deny path "/etc/"
allow network "api.openai.com" https
"#;
        let policy = parse(input).unwrap();
        assert_eq!(policy.default_decision, PolicyDecision::Deny);
        assert_eq!(policy.command_rules.len(), 2);
        assert_eq!(policy.path_rules.len(), 1);
        assert_eq!(policy.network_rules.len(), 1);
        // parse() 末尾调用 normalize() 按 priority 降序排序，
        // rm -rf（priority 100）应排在 git status（默认 0）之前。
        assert_eq!(policy.command_rules[0].priority, 100);
        assert_eq!(policy.command_rules[0].pattern, "rm -rf");
        assert_eq!(policy.command_rules[1].priority, 0);
        assert_eq!(policy.command_rules[1].pattern, "git status");
    }

    #[test]
    fn parse_invalid_keyword() {
        let input = "foobar command \"git\"";
        assert!(parse(input).is_err());
    }

    #[test]
    fn parse_unquoted_pattern() {
        let input = "allow command git";
        assert!(parse(input).is_err());
    }

    #[test]
    fn serialize_roundtrip() {
        let policy = default_policy_from_builtin();
        let text = serialize(&policy);
        let reparsed = parse(&text).unwrap();
        assert_eq!(reparsed.default_decision, policy.default_decision);
        assert_eq!(reparsed.command_rules.len(), policy.command_rules.len());
        assert_eq!(reparsed.network_rules.len(), policy.network_rules.len());
    }

    fn default_policy_from_builtin() -> ExecPolicy {
        super::super::builtin::default_policy()
    }

    #[test]
    fn strip_glob_prefix_works() {
        assert_eq!(strip_glob_prefix("**/.env"), ".env");
        assert_eq!(strip_glob_prefix("/etc/"), "/etc/");
        assert_eq!(strip_glob_prefix("**"), "");
    }
}
