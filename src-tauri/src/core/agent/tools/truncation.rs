//! ═══════════════════════════════════════════════════════════════════════════
//! Truncation - 工具输出截断策略
//! ═══════════════════════════════════════════════════════════════════════════

/// Tool 输出硬上限（字节）。
pub const EXEC_OUTPUT_MAX_BYTES: usize = 512 * 1024;

/// 单次 tool 调用允许发射的 output delta 事件上限。
///
/// 对照 codex `MAX_EXEC_OUTPUT_DELTAS_PER_CALL`：聚合仍收集完整输出，
/// 仅 live 事件流被 cap（避免流式场景下事件爆炸）。
pub const MAX_EXEC_OUTPUT_DELTAS_PER_CALL: usize = 10_000;

/// Token 估算的字节比例（4 bytes/token，对照 codex `approx_token_count`）。
const BYTES_PER_TOKEN: usize = 4;

/// 截断策略，对照 codex `TruncationPolicy`。
///
/// - `Bytes(n)`：按字节预算截断（精确，但可能切断多字节字符的中间——本实现按 char 边界安全截断）。
/// - `Tokens(n)`：按 token 预算截断（4 bytes/token 启发式估算）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationPolicy {
    /// 按 token 数截断（使用 4 bytes/token 启发式估算）。
    Tokens(usize),
    /// 按字节数截断。
    Bytes(usize),
}

impl TruncationPolicy {
    /// 字节预算（Tokens 策略按 4 bytes/token 转换）。
    pub fn byte_budget(self) -> usize {
        match self {
            TruncationPolicy::Bytes(n) => n,
            TruncationPolicy::Tokens(n) => n.saturating_mul(BYTES_PER_TOKEN),
        }
    }

    /// token 预算（Bytes 策略按 4 bytes/token 反向转换，向上取整）。
    pub fn token_budget(self) -> usize {
        match self {
            TruncationPolicy::Tokens(n) => n,
            TruncationPolicy::Bytes(n) => n.div_ceil(BYTES_PER_TOKEN),
        }
    }

    /// 默认 exec 输出策略（按 [`EXEC_OUTPUT_MAX_BYTES`] 字节截断）。
    pub fn default_exec() -> Self {
        TruncationPolicy::Bytes(EXEC_OUTPUT_MAX_BYTES)
    }
}

/// 估算字符串的 token 数（4 bytes/token 启发式，对照 codex `approx_token_count`）。
///
/// 这是粗略下界，非 tokenizer 精确计数；用于截断决策与日志统计。
pub fn approx_token_count(s: &str) -> usize {
    s.len().div_ceil(BYTES_PER_TOKEN)
}

/// 中段截断：保留首尾各一半预算，丢弃中间。
///
/// 对照 codex `truncate_middle_chars`：模型通常关心输出的开头（错误信息、命令回显）
/// 与结尾（最终结果、退出状态），中段往往是重复性日志，截断中段信息损失最小。
///
/// 截断会在 char 边界安全进行（不会切断多字节字符）。若 `max_bytes >= s.len()`，原样返回。
pub fn truncate_middle_chars(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    // 各保留一半预算给首尾。
    let half = max_bytes / 2;
    // 在 char 边界找 <= half 字节的切分点。
    let head_end = char_boundary_left(s, half);
    let tail_start = char_boundary_right(s, s.len().saturating_sub(half));
    let omitted = tail_start.saturating_sub(head_end);
    let mut result = String::with_capacity(head_end + (s.len() - tail_start) + 64);
    result.push_str(&s[..head_end]);
    result.push_str("\n[truncated, ");
    result.push_str(&omitted.to_string());
    result.push_str(" bytes omitted]\n");
    result.push_str(&s[tail_start..]);
    result
}

/// 按策略截断文本（中段截断）。
pub fn truncate_text(s: &str, policy: TruncationPolicy) -> String {
    truncate_middle_chars(s, policy.byte_budget())
}

/// 统一格式化 tool exec 输出，对照 codex `exec.rs` 的格式化逻辑。
///
/// 输出格式（每节独占一行，空行分隔元信息与正文）：
/// ```text
/// Exit code: N
/// Wall time: Mms
/// Total output lines: L
///
/// <output, possibly truncated>
/// ```
///
/// - `exit_code = None`：省略 Exit code 行（适用于非进程类 tool）。
/// - `wall_time_ms = None`：省略 Wall time 行。
/// - `output` 超过 policy 预算时，通过 [`truncate_text`] 中段截断并附加 `[truncated, ...]` 标记。
pub fn format_exec_output_for_model(
    output: &str,
    exit_code: Option<i32>,
    wall_time_ms: Option<u64>,
    policy: &TruncationPolicy,
) -> String {
    let total_lines = output.lines().count();
    let original_token_count = approx_token_count(output);

    // 先按 policy 截断正文。
    let truncated_body = if output.len() > policy.byte_budget() {
        // 截断后附加 token 计数提示，便于模型感知信息缺失。
        let body = truncate_text(output, *policy);
        format!(
            "Warning: truncated output (original token count: {original_token_count})\n{body}"
        )
    } else {
        output.to_string()
    };

    let mut lines = Vec::new();
    if let Some(code) = exit_code {
        lines.push(format!("Exit code: {code}"));
    }
    if let Some(ms) = wall_time_ms {
        lines.push(format!("Wall time: {ms}ms"));
    }
    lines.push(format!("Total output lines: {total_lines}"));

    let header = lines.join("\n");
    format!("{header}\n\n{truncated_body}")
}

/// 在 `s` 中找到 <= `max_bytes` 字节的最大 char 边界（用于安全切分多字节字符）。
///
/// 从 `max_bytes` 向前回退直到落在 char 边界。
fn char_boundary_left(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut i = max_bytes;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 在 `s` 中找到 >= `min_bytes` 字节的最小 char 边界。
///
/// 从 `min_bytes` 向后推进直到落在 char 边界。
fn char_boundary_right(s: &str, min_bytes: usize) -> usize {
    if min_bytes >= s.len() {
        return s.len();
    }
    let mut i = min_bytes;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

// ============== 测试 ==============

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- 常量 ----------

    #[test]
    fn exec_output_max_bytes_is_512kb() {
        assert_eq!(EXEC_OUTPUT_MAX_BYTES, 512 * 1024);
    }

    #[test]
    fn max_exec_output_deltas_per_call_is_10000() {
        assert_eq!(MAX_EXEC_OUTPUT_DELTAS_PER_CALL, 10_000);
    }

    // ---------- TruncationPolicy ----------

    #[test]
    fn policy_byte_budget_for_bytes_variant() {
        assert_eq!(TruncationPolicy::Bytes(1024).byte_budget(), 1024);
    }

    #[test]
    fn policy_byte_budget_for_tokens_variant_uses_4_bytes_per_token() {
        // 100 tokens * 4 bytes/token = 400 bytes。
        assert_eq!(TruncationPolicy::Tokens(100).byte_budget(), 400);
    }

    #[test]
    fn policy_token_budget_for_tokens_variant() {
        assert_eq!(TruncationPolicy::Tokens(100).token_budget(), 100);
    }

    #[test]
    fn policy_token_budget_for_bytes_variant_ceil_div() {
        // 1024 bytes / 4 = 256 tokens。
        assert_eq!(TruncationPolicy::Bytes(1024).token_budget(), 256);
        // 1025 bytes / 4 = 257 tokens（向上取整）。
        assert_eq!(TruncationPolicy::Bytes(1025).token_budget(), 257);
    }

    #[test]
    fn default_exec_policy_uses_exec_output_max_bytes() {
        assert_eq!(
            TruncationPolicy::default_exec(),
            TruncationPolicy::Bytes(EXEC_OUTPUT_MAX_BYTES)
        );
    }

    // ---------- approx_token_count ----------

    #[test]
    fn approx_token_count_4_bytes_per_token() {
        assert_eq!(approx_token_count(""), 0);
        assert_eq!(approx_token_count("abcd"), 1);
        assert_eq!(approx_token_count("abcde"), 2, "5 bytes -> ceil(5/4) = 2");
        assert_eq!(approx_token_count("abcdefgh"), 2);
    }

    // ---------- truncate_middle_chars ----------

    #[test]
    fn truncate_middle_chars_noop_when_under_budget() {
        let s = "hello world";
        assert_eq!(truncate_middle_chars(s, 100), s);
    }

    #[test]
    fn truncate_middle_chars_truncates_middle_preserves_head_tail() {
        // 30 bytes 输入，预算 10 bytes → head 5 + tail 5。
        let s = "012345678901234567890123456789"; // 30 chars
        let result = truncate_middle_chars(s, 10);
        assert!(result.contains("01234"), "应保留头部 5 bytes");
        assert!(result.contains("56789"), "应保留尾部 5 bytes");
        assert!(result.contains("[truncated,"), "应有截断标记");
        assert!(result.contains("bytes omitted]"));
        // 中段 "56789012345678901234" 应被丢弃。
        assert!(!result.contains("5678901234"), "中段应被丢弃");
    }

    #[test]
    fn truncate_middle_chars_safe_on_multibyte_chars() {
        // 中文每个字符 3 bytes (UTF-8)，确保不在字符中间截断。
        let s = "你好世界测试截断"; // 8 chars * 3 bytes = 24 bytes
        let result = truncate_middle_chars(s, 6); // 预算 6 bytes → head 3 + tail 3
        // 头部应完整保留 "你"（3 bytes），尾部应完整保留 "断"（3 bytes）。
        assert!(result.starts_with("你"), "头部应保留完整字符");
        assert!(result.ends_with("断\n") || result.ends_with("断"), "尾部应保留完整字符");
        assert!(result.contains("[truncated,"));
    }

    #[test]
    fn truncate_middle_chars_empty_input() {
        assert_eq!(truncate_middle_chars("", 100), "");
    }

    #[test]
    fn truncate_middle_chars_budget_larger_than_input() {
        let s = "short";
        assert_eq!(truncate_middle_chars(s, 100), "short");
    }

    // ---------- truncate_text ----------

    #[test]
    fn truncate_text_bytes_policy_uses_byte_budget() {
        let s = "abcdefghij"; // 10 bytes
        let result = truncate_text(s, TruncationPolicy::Bytes(4));
        assert!(result.contains("[truncated,"));
        assert!(result.starts_with("ab"));
        assert!(result.ends_with("ij\n") || result.contains("ij"));
    }

    #[test]
    fn truncate_text_tokens_policy_uses_4x_byte_budget() {
        // 10 tokens * 4 bytes/token = 40 bytes 预算。
        let s = "x".repeat(100); // 100 bytes
        let result = truncate_text(&s, TruncationPolicy::Tokens(10));
        // 40 bytes 预算 < 100 bytes 输入，应触发截断。
        assert!(result.contains("[truncated,"));
    }

    // ---------- format_exec_output_for_model ----------

    #[test]
    fn format_exec_output_includes_all_metadata_sections() {
        let output = "line1\nline2\nline3";
        let result = format_exec_output_for_model(
            output,
            Some(0),
            Some(150),
            &TruncationPolicy::Bytes(1024),
        );
        assert!(result.contains("Exit code: 0"));
        assert!(result.contains("Wall time: 150ms"));
        assert!(result.contains("Total output lines: 3"));
        assert!(result.contains("line1\nline2\nline3"));
    }

    #[test]
    fn format_exec_output_omits_none_metadata() {
        let output = "only output";
        let result = format_exec_output_for_model(output, None, None, &TruncationPolicy::Bytes(1024));
        assert!(!result.contains("Exit code"));
        assert!(!result.contains("Wall time"));
        assert!(result.contains("Total output lines: 1"));
        assert!(result.contains("only output"));
    }

    #[test]
    fn format_exec_output_truncates_when_over_budget() {
        let output = "x".repeat(200); // 200 bytes
        let result = format_exec_output_for_model(
            &output,
            Some(1),
            None,
            &TruncationPolicy::Bytes(100),
        );
        // 应包含截断标记与 token 计数。
        assert!(result.contains("Warning: truncated output"));
        assert!(result.contains("original token count:"));
        assert!(result.contains("[truncated,"));
        assert!(result.contains("Exit code: 1"));
    }

    #[test]
    fn format_exec_output_no_truncation_when_under_budget() {
        let output = "small output";
        let result = format_exec_output_for_model(
            output,
            Some(0),
            Some(10),
            &TruncationPolicy::Bytes(1024),
        );
        // 不应有截断标记。
        assert!(!result.contains("truncated"));
        assert!(result.contains("small output"));
    }

    #[test]
    fn format_exec_output_counts_lines_correctly() {
        // 含空行与多行。
        let output = "a\n\nb\nc";
        let result = format_exec_output_for_model(output, None, None, &TruncationPolicy::Bytes(1024));
        assert!(result.contains("Total output lines: 4"));
    }

    #[test]
    fn format_exec_output_empty_output() {
        let result = format_exec_output_for_model("", Some(0), Some(5), &TruncationPolicy::Bytes(1024));
        assert!(result.contains("Exit code: 0"));
        assert!(result.contains("Wall time: 5ms"));
        assert!(result.contains("Total output lines: 0"));
    }

    #[test]
    fn format_exec_output_with_default_exec_policy_passes_512kb() {
        // 验证 default_exec_policy 与 format_exec_output_for_model 的集成。
        let output = "x".repeat(1024);
        let policy = TruncationPolicy::default_exec();
        let result = format_exec_output_for_model(&output, Some(0), None, &policy);
        // 1KB << 512KB，不应截断。
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn format_exec_output_truncates_large_input_with_default_policy() {
        // 构造超过 512KB 的输出，验证 default policy 触发截断。
        let output = "y".repeat(EXEC_OUTPUT_MAX_BYTES + 1024);
        let policy = TruncationPolicy::default_exec();
        let result = format_exec_output_for_model(&output, Some(0), None, &policy);
        assert!(result.contains("Warning: truncated output"));
        assert!(result.contains("[truncated,"));
    }
}
