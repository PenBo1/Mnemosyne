//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Parsing - 解析与规范化辅助
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：Markdown 小节抽取、图像提示词解析、集尾标签规范化、max_tokens 估算。

/// 从分镜 Markdown 中抽取图像提示词（合并抽取与解析两步）。
///
/// 先尝试定位"图像提示词"小节，找不到则用全文。支持：
/// - Markdown 表格中的 Prompt 列
/// - `Prompt: ...` / `提示词: ...` 行
/// - 编号列表行 `1. ...`
pub fn extract_image_prompts(markdown: &str) -> Vec<String> {
    let section = extract_image_prompt_section(markdown);
    let section_trimmed = section.trim();
    let source = if section_trimmed.is_empty() {
        markdown.trim()
    } else {
        section_trimmed
    };
    parse_prompt_lines(source)
}

/// 抽取 Markdown 小节（单标题版本）。
///
/// 匹配规则：标题文本归一化（小写、去 markdown 标记）后，
/// 若 text == heading 或 text 以 heading 开头且剩余部分为空或以分隔符开头，则匹配。
pub fn extract_markdown_section(content: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let normalized_heading = normalize_heading_text(heading);

    let mut start: i64 = -1;
    let mut level: usize = 0;

    let heading_re = regex::Regex::new(r"^(#{1,6})\s*(.+?)\s*$").ok()?;

    for (index, line) in lines.iter().enumerate() {
        if let Some(caps) = heading_re.captures(line) {
            let text = normalize_heading_text(&caps[2]);
            if heading_matches(&text, &normalized_heading) {
                start = index as i64 + 1;
                level = caps[1].len();
                break;
            }
        }
    }

    if start < 0 {
        return None;
    }

    let start_idx = start as usize;
    let mut end = lines.len();
    for (index, line) in lines.iter().enumerate().skip(start_idx) {
        if let Some(caps) = heading_re.captures(line) {
            if caps[1].len() <= level {
                end = index;
                break;
            }
        }
    }

    Some(lines[start_idx..end].join("\n"))
}

/// 规范化剧本集尾标签。
///
/// 跟踪 `## 第N集` 标题，将 "字幕：第X集完" 替换为 "字幕：第{current}集完"。
/// current_episode 作为初始值（在未遇到任何集标题前使用）。
pub fn normalize_episode_end_labels(markdown: &str, current_episode: u32) -> String {
    let heading_re = regex::Regex::new(
        r"^#{1,6}\s*第\s*([一二三四五六七八九十百千万\d]+)\s*集(?:\s|$)",
    )
    .expect("invalid episode heading regex");

    let replace_re = regex::Regex::new(
        r"(字幕\s*[：:]\s*)第\s*[一二三四五六七八九十百千万\d]+\s*集完",
    )
    .expect("invalid episode end label regex");

    let mut current = current_episode.to_string();
    markdown
        .lines()
        .map(|line| {
            if let Some(caps) = heading_re.captures(line.trim()) {
                current = caps[1].to_string();
            }
            replace_re
                .replace_all(line, format!("${{1}}第{}集完", current))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 估算 max_tokens。
pub fn estimate_max_tokens(episodes: u32, per_episode: u32, min: u64, max: u64) -> u64 {
    std::cmp::min(max, std::cmp::max(min, episodes as u64 * per_episode as u64))
}

// ── 内部辅助函数（private） ─────────────────────────────────

/// 归一化标题文本。
fn normalize_heading_text(text: &str) -> String {
    let trimmed = text.trim();
    // 去首尾 ** 包裹（使用 strip_prefix/strip_suffix 避免 byte slicing panic；
    // 仅当去包裹后仍有内容时才剥离，避免把 "***" 之类剥成空串）
    let de_bolded: &str = match trimmed
        .strip_prefix("**")
        .and_then(|s| s.strip_suffix("**"))
    {
        Some(inner) if !inner.is_empty() => inner,
        _ => trimmed,
    };
    // 去 markdown 标记字符 ` * _
    let de_marked: String = de_bolded
        .chars()
        .filter(|&c| c != '`' && c != '*' && c != '_')
        .collect();
    // 合并空白并小写（缓存正则避免重复编译）
    static WS_PLUS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let collapsed = WS_PLUS
        .get_or_init(|| regex::Regex::new(r"\s+").expect("valid ws+ regex"))
        .replace_all(&de_marked, " ")
        .to_string();
    collapsed.trim().to_lowercase()
}

/// 标题匹配。
fn heading_matches(text: &str, heading: &str) -> bool {
    if text == heading {
        return true;
    }
    if !text.starts_with(heading) {
        return false;
    }
    let rest = text[heading.len()..].trim();
    if rest.is_empty() {
        return true;
    }
    // 剩余以分隔符开头
    rest.starts_with('（')
        || rest.starts_with('(')
        || rest.starts_with('【')
        || rest.starts_with('[')
        || rest.starts_with(':')
        || rest.starts_with('：')
        || rest.starts_with('-')
        || rest.starts_with('—')
        || rest.starts_with(' ')
}

/// 尝试定位图像提示词小节（多标题候选）。
fn extract_image_prompt_section(markdown: &str) -> String {
    for heading in &["图像提示词", "分镜图提示词", "Image Prompts", "Shot Image Prompts"] {
        if let Some(section) = extract_markdown_section(markdown, heading) {
            return section;
        }
    }
    String::new()
}

/// 解析提示词行。
fn parse_prompt_lines(markdown: &str) -> Vec<String> {
    let mut prompts: Vec<String> = Vec::new();
    let mut prompt_column_index: i64 = -1;

    let prompt_re = regex::Regex::new(
        r"(?i)(?:^|[|>\-\d.)、\s])(?:\*\*)?\s*(?:Prompt(?:\s+for\s+[^:*：]+)?|提示词(?:\s*[^:*：]+)?|图像提示词|分镜图提示词)\s*(?:\*\*)?\s*[：:]\s*(.+?)\s*$",
    )
    .expect("invalid prompt regex");

    let numbered_re = regex::Regex::new(
        r"^(?:[-*]\s*)?(?:\d+)[.)、：:\s-]+(.+)$",
    )
    .expect("invalid numbered prompt regex");

    for raw_line in markdown.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            prompt_column_index = -1;
            continue;
        }

        // Markdown 表格行
        if let Some(cells) = parse_markdown_table_row(line) {
            if is_markdown_table_separator(&cells) {
                continue;
            }
            if let Some(idx) = cells.iter().position(|c| is_prompt_column_header(c)) {
                prompt_column_index = idx as i64;
                continue;
            }
            if prompt_column_index >= 0 {
                let cell = cells.get(prompt_column_index as usize).map(|s| s.as_str()).unwrap_or("");
                let prompt = clean_prompt_text(cell);
                if !prompt.is_empty() {
                    prompts.push(prompt);
                }
            }
            continue;
        }

        prompt_column_index = -1;

        // Prompt: ... / 提示词: ... 行
        if let Some(caps) = prompt_re.captures(line) {
            let prompt = clean_prompt_text(&caps[1]);
            if !prompt.is_empty() {
                prompts.push(prompt);
            }
            continue;
        }

        // 编号列表行 1. ...
        if let Some(caps) = numbered_re.captures(line) {
            let prompt = caps[1]
                .chars()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !prompt.is_empty() {
                prompts.push(prompt);
            }
        }
    }

    prompts
}

/// 解析 Markdown 表格行。
fn parse_markdown_table_row(line: &str) -> Option<Vec<String>> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let inner = &line[1..line.len() - 1];
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    if cells.len() >= 2 {
        Some(cells)
    } else {
        None
    }
}

/// 判断是否为表格分隔行。
fn is_markdown_table_separator(cells: &[String]) -> bool {
    let re = regex::Regex::new(r"^:?-{3,}:?$").expect("invalid separator regex");
    cells.iter().all(|c| re.is_match(c))
}

/// 判断是否为 Prompt 列表头。
fn is_prompt_column_header(cell: &str) -> bool {
    let cleaned: String = cell.chars().filter(|&c| c != '`' && c != '*' && c != '_').collect();
    let trimmed = cleaned.trim();
    let re = regex::Regex::new(r"(?i)^(?:prompt|image\s*prompt|shot\s*prompt|提示词|图像提示词|分镜图提示词)$")
        .expect("invalid header regex");
    re.is_match(trimmed)
}

/// 清理提示词文本。
fn clean_prompt_text(text: &str) -> String {
    let step1 = regex::Regex::new(r"\s*\|\s*$")
        .map(|re| re.replace_all(text, "").to_string())
        .unwrap_or_else(|_| text.to_string());

    let step2 = regex::Regex::new(r"\*\*$")
        .map(|re| re.replace_all(&step1, "").to_string())
        .unwrap_or(step1);

    let step3 = regex::Regex::new(r"(?i)^(?:Prompt(?:\s+for\s+[^:*：]+)?|提示词(?:\s*[^:*：]+)?|图像提示词|分镜图提示词)\s*[：:]\s*")
        .map(|re| re.replace_all(&step2, "").to_string())
        .unwrap_or(step2);

    let step4 = regex::Regex::new(r"\s+")
        .map(|re| re.replace_all(&step3, " ").to_string())
        .unwrap_or(step3);

    step4.trim().to_string()
}

// ── 单元测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── estimate_max_tokens ──

    #[test]
    fn estimate_max_tokens_clamps_to_min() {
        assert_eq!(estimate_max_tokens(1, 2200, 12000, 32000), 12000);
    }

    #[test]
    fn estimate_max_tokens_clamps_to_max() {
        assert_eq!(estimate_max_tokens(100, 2200, 12000, 32000), 32000);
    }

    #[test]
    fn estimate_max_tokens_computes_midrange() {
        assert_eq!(estimate_max_tokens(10, 2200, 12000, 32000), 22000);
    }

    // ── normalize_episode_end_labels ──

    #[test]
    fn normalize_episode_end_labels_replaces_with_heading() {
        let markdown = "## 第一集\n\nsome content\n\n字幕：第三集完\n";
        let result = normalize_episode_end_labels(markdown, 1);
        assert!(result.contains("字幕：第一集完"));
        assert!(!result.contains("第三集完"));
    }

    #[test]
    fn normalize_episode_end_labels_uses_fallback_before_heading() {
        let markdown = "字幕：第五集完\n";
        let result = normalize_episode_end_labels(markdown, 3);
        // fallback 使用 current_episode 的阿拉伯数字形式
        assert!(result.contains("字幕：第3集完"));
    }

    #[test]
    fn normalize_episode_end_labels_preserves_arabic_numerals() {
        let markdown = "## 第2集\n\n字幕：第9集完\n";
        let result = normalize_episode_end_labels(markdown, 1);
        assert!(result.contains("字幕：第2集完"));
    }

    // ── extract_markdown_section ──

    #[test]
    fn extract_markdown_section_finds_heading() {
        let content = "# Title\n\n## Storyboard\n\nshot 1\nshot 2\n\n## Image Prompts\n\nPrompt: foo\n";
        let section = extract_markdown_section(content, "Image Prompts");
        assert!(section.is_some());
        assert!(section.unwrap().contains("Prompt: foo"));
    }

    #[test]
    fn extract_markdown_section_returns_none_when_missing() {
        let content = "# Title\n\nNo headings here\n";
        let section = extract_markdown_section(content, "Image Prompts");
        assert!(section.is_none());
    }

    #[test]
    fn extract_markdown_section_stops_at_same_level() {
        let content = "## Section A\n\ncontent a\n\n## Section B\n\ncontent b\n";
        let section = extract_markdown_section(content, "Section A");
        assert!(section.is_some());
        let section = section.unwrap();
        assert!(section.contains("content a"));
        assert!(!section.contains("content b"));
    }

    // ── extract_image_prompts ──

    #[test]
    fn extract_image_prompts_parses_prompt_lines() {
        let markdown = "## Image Prompts\n\nPrompt: a beautiful sunset\nPrompt: a city at night\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "a beautiful sunset");
        assert_eq!(prompts[1], "a city at night");
    }

    #[test]
    fn extract_image_prompts_parses_chinese_prompts() {
        let markdown = "## 图像提示词\n\n提示词：日落\n提示词：夜景\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "日落");
        assert_eq!(prompts[1], "夜景");
    }

    #[test]
    fn extract_image_prompts_parses_numbered_list() {
        let markdown = "## Image Prompts\n\n1. first prompt\n2. second prompt\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "first prompt");
        assert_eq!(prompts[1], "second prompt");
    }

    #[test]
    fn extract_image_prompts_parses_table_column() {
        let markdown = "## Image Prompts\n\n| Shot | Prompt |\n| --- | --- |\n| 1 | sunset |\n| 2 | night |\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "sunset");
        assert_eq!(prompts[1], "night");
    }

    #[test]
    fn extract_image_prompts_falls_back_to_full_content() {
        let markdown = "Prompt: standalone prompt\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0], "standalone prompt");
    }

    // ── normalize_heading_text ──

    #[test]
    fn normalize_heading_text_lowercases_and_strips_markdown() {
        assert_eq!(normalize_heading_text("**Image Prompts**"), "image prompts");
        assert_eq!(normalize_heading_text("图像提示词"), "图像提示词");
    }
}
