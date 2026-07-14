// LLM JSON 输出提取工具 —— 从可能含前后缀文字的 LLM 响应中提取首个完整 JSON 对象。
//
// 用于 radar / researcher 等解析 LLM 结构化输出的场景,避免重复实现。

/// 从文本中提取首个 `{...}` JSON 块(括号匹配,跳过字符串内部花括号)。
/// 优先尝试从 ```json ... ``` 代码块中提取,回退到全文搜索。
pub fn extract_json_block(content: &str) -> Option<&str> {
    // 先尝试从 ```json ... ``` 代码块中提取
    if let Some(start_marker) = content.find("```json") {
        let after_marker = &content[start_marker + 7..];
        if let Some(json_start) = after_marker.find('{') {
            let abs_start = start_marker + 7 + json_start;
            if let Some(block) = match_braces(&content[abs_start..]) {
                return Some(block);
            }
        }
    }
    // 回退: 全文搜索首个 {...} 块
    match_braces(content)
}

/// 从以 `{` 开头的文本中,用括号匹配找到完整的 JSON 块。
pub fn match_braces(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, ch) in content[start..].char_indices() {
        // 字符串内部不计数,避免 JSON 值中的花括号干扰
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&content[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_block_simple() {
        let input = r#"some text {"a":1} more text"#;
        assert_eq!(extract_json_block(input), Some(r#"{"a":1}"#));
    }

    #[test]
    fn extract_json_block_nested() {
        let input = r#"prefix {"a":{"b":2},"c":[1,2]} suffix"#;
        assert_eq!(extract_json_block(input), Some(r#"{"a":{"b":2},"c":[1,2]}"#));
    }

    #[test]
    fn extract_json_block_none() {
        let input = "no json here";
        assert_eq!(extract_json_block(input), None);
    }

    #[test]
    fn extract_json_block_markdown() {
        let input = "```json\n{\"a\":1}\n```";
        assert_eq!(extract_json_block(input), Some(r#"{"a":1}"#));
    }

    #[test]
    fn extract_json_block_braces_in_string() {
        let input = r#"{"text":"hello {world}","a":1}"#;
        assert_eq!(extract_json_block(input), Some(r#"{"text":"hello {world}","a":1}"#));
    }
}
