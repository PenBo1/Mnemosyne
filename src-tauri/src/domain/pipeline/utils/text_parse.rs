// 文本解析共享工具 - 收口 agents / runner 中重复的纯文本解析逻辑。
//
// 提供能力：
// - count_zh_chars: 中文字符计数（CJK 区段）
// - count_non_whitespace_chars: 非空白字符计数（粗略长度度量）
// - strip_code_fence: 去除代码块包裹
// - extract_section: 从 === TAG === 标记中提取区块内容
//
// 注意：
// - JSON 提取请使用 crate::shared::utils::json::extract_json_block（已存在）。
// - governance/input.rs::extract_section 使用 ## heading markdown 格式，语义不同，不可合并。

/// 统计中文字符数（CJK 统一表意文字 + 扩展 A 区 + 兼容表意文字）。
pub fn count_zh_chars(content: &str) -> u32 {
    content
        .chars()
        .filter(|&c| {
            ('\u{4E00}'..='\u{9FFF}').contains(&c)
                || ('\u{3400}'..='\u{4DBF}').contains(&c)
                || ('\u{F900}'..='\u{FAFF}').contains(&c)
        })
        .count() as u32
}

/// 统计非空白字符数（粗略长度度量）。
///
/// 与 count_zh_chars 语义不同：
/// - count_zh_chars 仅计汉字，用于中文长度规格的精确计数
/// - count_non_whitespace_chars 计所有非空白字符，用于中英文混排的粗略长度比较
pub fn count_non_whitespace_chars(content: &str) -> u32 {
    content
        .chars()
        .filter(|c| !c.is_whitespace())
        .count() as u32
}

/// 去除 ```...``` 代码块包裹。
///
/// 支持带语言标识（```markdown）与不带标识（```）两种形式。
/// 若内容未被代码块包裹，原样返回（trim 后）。
pub fn strip_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let after_open = match trimmed.find('\n') {
        Some(idx) => &trimmed[idx + 1..],
        None => return trimmed.trim_start_matches("```").to_string(),
    };

    let without_close = after_open.trim_end_matches("```");
    without_close.trim().to_string()
}

/// 从 === TAG === 标记中提取区块内容。
///
/// 找到 === TAG === 标记后，提取到下一个 === TAG === 或文本结尾的内容。
/// 返回去除首尾空白的区块文本；未找到标记时返回 None。
pub fn extract_section(content: &str, tag: &str) -> Option<String> {
    let marker = format!("=== {} ===", tag);
    let start = content.find(&marker)?;
    let content_start = start + marker.len();

    let remaining = &content[content_start..];
    let end = remaining
        .find("\n=== ")
        .map(|pos| content_start + pos)
        .unwrap_or(content.len());

    Some(content[content_start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_zh_chars_counts_cjk_only() {
        assert_eq!(count_zh_chars("你好 world 123"), 2);
        assert_eq!(count_zh_chars("暗流涌动"), 4);
        assert_eq!(count_zh_chars("no chinese here"), 0);
    }

    #[test]
    fn count_non_whitespace_chars_counts_all_non_ws() {
        assert_eq!(count_non_whitespace_chars("你好 world 123"), 10);
        assert_eq!(count_non_whitespace_chars("  \n\t  "), 0);
    }

    #[test]
    fn strip_code_fence_with_lang() {
        let content = "```markdown\n这是正文内容。\n```";
        assert_eq!(strip_code_fence(content), "这是正文内容。");
    }

    #[test]
    fn strip_code_fence_plain() {
        let content = "```\n正文内容\n```";
        assert_eq!(strip_code_fence(content), "正文内容");
    }

    #[test]
    fn strip_code_fence_keeps_plain_content() {
        let content = "这是普通正文，没有代码块包裹。";
        assert_eq!(strip_code_fence(content), "这是普通正文，没有代码块包裹。");
    }

    #[test]
    fn extract_section_returns_block_content() {
        let content = "=== TITLE ===\n暗流\n\n=== CONTENT ===\n这是正文。";
        assert_eq!(extract_section(content, "TITLE"), Some("暗流".to_string()));
        assert_eq!(extract_section(content, "CONTENT"), Some("这是正文。".to_string()));
    }

    #[test]
    fn extract_section_last_block_to_end() {
        let content = "=== TITLE ===\n标题\n\n=== CONTENT ===\n最后一行";
        assert_eq!(extract_section(content, "CONTENT"), Some("最后一行".to_string()));
    }

    #[test]
    fn extract_section_missing_returns_none() {
        assert_eq!(extract_section("无标记文本", "TITLE"), None);
    }
}
