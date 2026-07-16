// 文件系统安全化工具 —— 纯函数，无 I/O。
//
// 收口 short_fiction_runner / script_storyboard_runner 中重复的 slugify 与
// safe_segment 实现。两者仅在空结果回退前缀上不同，故通过 fallback_prefix
// 参数化。

use std::sync::OnceLock;

/// slugify：将标题转为 URL 友好的 slug。
///
/// 小写化 → 去引号 → 非字母数字序列替换为单个 - → 去首尾 - → 截断 60 字符。
/// 结果为空时回退为 `{fallback_prefix}-{timestamp}`。
pub fn slugify(value: &str, fallback_prefix: &str) -> String {
    let lower: String = value.to_lowercase();
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in lower.chars() {
        if c == '\'' || c == '"' {
            continue;
        }
        if c.is_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    let truncated: String = trimmed.chars().take(60).collect();
    if truncated.is_empty() {
        format!("{}-{}", fallback_prefix, chrono::Utc::now().timestamp_millis())
    } else {
        truncated
    }
}

/// 文件系统安全段。
///
/// 危险字符 → -，空白序列 → 单个 -，去首尾 -，截断 80 字符。
/// 结果为空 / "." / ".." 时回退为 `{fallback_prefix}-{timestamp}`。
pub fn safe_segment(value: &str, fallback_prefix: &str) -> String {
    // 1. 替换危险字符为 -
    let after_dangerous: String = value
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '\0' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();
    // 2. 替换空白字符序列为单个 -（缓存正则避免重复编译）
    static WS_PLUS: OnceLock<regex::Regex> = OnceLock::new();
    let after_ws = WS_PLUS
        .get_or_init(|| regex::Regex::new(r"\s+").expect("valid ws+ regex"))
        .replace_all(&after_dangerous, "-")
        .to_string();
    // 3. 去除首尾 -，截断到 80 字符
    let trimmed = after_ws.trim_matches('-');
    let truncated: String = trimmed.chars().take(80).collect();
    if truncated.is_empty() || truncated == "." || truncated == ".." {
        format!("{}-{}", fallback_prefix, chrono::Utc::now().timestamp_millis())
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_lowercases_and_dashes() {
        assert_eq!(slugify("Hello World!", "short"), "hello-world");
    }

    #[test]
    fn slugify_preserves_cjk() {
        assert_eq!(slugify("暗流 涌动", "short"), "暗流-涌动");
    }

    #[test]
    fn slugify_truncates_to_60() {
        let long: String = "a".repeat(80);
        let result = slugify(&long, "short");
        assert_eq!(result.chars().count(), 60);
    }

    #[test]
    fn slugify_empty_returns_fallback() {
        let result = slugify("!!!", "short");
        assert!(result.starts_with("short-"));
    }

    #[test]
    fn safe_segment_replaces_dangerous_chars() {
        assert_eq!(safe_segment("a/b:c", "short"), "a-b-c");
    }

    #[test]
    fn safe_segment_collapses_whitespace_runs() {
        assert_eq!(safe_segment("hello   world", "short"), "hello-world");
    }

    #[test]
    fn safe_segment_rejects_dot() {
        let result = safe_segment(".", "short");
        assert!(result.starts_with("short-"));
    }

    #[test]
    fn safe_segment_rejects_empty() {
        let result = safe_segment("", "short");
        assert!(result.starts_with("short-"));
    }

    #[test]
    fn safe_segment_truncates_to_80() {
        let long: String = "a".repeat(100);
        let result = safe_segment(&long, "short");
        assert_eq!(result.chars().count(), 80);
    }
}
