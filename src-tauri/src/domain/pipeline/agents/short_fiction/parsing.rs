//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Parsing - 输出解析
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：解析 create_outline / write_draft / generate_package 三个 agent 的原始输出，
//! 提取标题、章节、销售包装字段。包含标签区块提取（`=== TAG ===`）、Markdown 回退解析、
//! 标题/章节归一化等纯函数辅助。

use crate::domain::pipeline::types::Language;

use super::helpers::{count_chapter_length, fallback_chapter_title};
use super::types::{ShortFictionBatchDraft, ShortFictionChapter, ShortFictionOutline,
    ShortFictionSalesPackage};

// ═══════════════════════════════════════════════════════════════
//  解析函数
// ═══════════════════════════════════════════════════════════════

/// 解析短篇大纲输出。
pub fn parse_outline(raw_content: &str, language: Language) -> ShortFictionOutline {
    let fallback_title = untitled_short_title(language);
    let story_title = extract_tagged_block(raw_content, "SHORT_FICTION_PLAN_TITLE")
        .or_else(|| extract_tagged_block(raw_content, "SHORT_FICTION_TITLE"))
        .or_else(|| extract_first_heading(raw_content))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    ShortFictionOutline {
        story_title,
        raw_content: raw_content.trim().to_string(),
    }
}

/// 解析批量草稿输出。
pub fn parse_batch_draft(
    raw_content: &str,
    expected_chapters: u32,
    language: Language,
) -> ShortFictionBatchDraft {
    let fallback_title = untitled_short_title(language);
    let story_title = extract_tagged_block(raw_content, "SHORT_FICTION_TITLE")
        .or_else(|| extract_first_heading(raw_content))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    let opening_hook = extract_tagged_block(raw_content, "SHORT_FICTION_OPENING_HOOK")
        .or_else(|| extract_tagged_block(raw_content, "OPENING_HOOK"))
        .filter(|s| !s.is_empty());

    let mut chapters = Vec::with_capacity(expected_chapters as usize);
    for number in 1..=expected_chapters {
        let title = extract_tagged_block(raw_content, &format!("CHAPTER {} TITLE", number))
            .or_else(|| extract_markdown_chapter_title(raw_content, number))
            .map(|t| normalize_chapter_title(&t, number, language))
            .unwrap_or_else(|| fallback_chapter_title(number, language));

        let content = extract_last_nonempty_tagged_block(raw_content, &format!("CHAPTER {} CONTENT", number))
            .or_else(|| extract_duplicate_title_tagged_chapter_content(raw_content, number))
            .or_else(|| extract_markdown_chapter_content(raw_content, number))
            .map(|c| sanitize_chapter_content(&c))
            .unwrap_or_default();

        let char_count = count_chapter_length(&content, language);
        chapters.push(ShortFictionChapter {
            number,
            title,
            content,
            char_count,
        });
    }

    ShortFictionBatchDraft {
        story_title,
        opening_hook,
        chapters,
        raw_content: raw_content.to_string(),
    }
}

/// 解析销售包装输出。
pub fn parse_sales_package(raw_content: &str, fallback_title: &str) -> ShortFictionSalesPackage {
    let title = extract_tagged_block(raw_content, "SHORT_FICTION_PACKAGE_TITLE")
        .or_else(|| extract_tagged_block(raw_content, "SHORT_FICTION_TITLE"))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    let intro = extract_tagged_block(raw_content, "SHORT_FICTION_INTRO")
        .or_else(|| extract_tagged_block(raw_content, "INTRO"))
        .unwrap_or_default();

    let selling_raw = extract_tagged_block(raw_content, "SHORT_FICTION_SELLING_POINTS")
        .or_else(|| extract_tagged_block(raw_content, "SELLING_POINTS"))
        .unwrap_or_default();

    let selling_points: Vec<String> = selling_raw
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            // Strip leading "- " or "* " bullet
            let stripped = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("-"))
                .or_else(|| trimmed.strip_prefix("*"))
                .unwrap_or(trimmed);
            stripped.trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    let cover_prompt = extract_tagged_block(raw_content, "SHORT_FICTION_COVER_PROMPT")
        .or_else(|| extract_tagged_block(raw_content, "COVER_PROMPT"))
        .unwrap_or_default();

    ShortFictionSalesPackage {
        title,
        intro: intro.trim().to_string(),
        selling_points,
        cover_prompt: cover_prompt.trim().to_string(),
        raw_content: raw_content.trim().to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════
//  标签区块提取
// ═══════════════════════════════════════════════════════════════

/// 提取 `=== TAG ===` 到下一个 `=== ... ===` 之间的内容（第一个非空块）。
/// 返回首个块，空字符串视为未找到。
pub fn extract_tagged_block(content: &str, tag: &str) -> Option<String> {
    extract_tagged_blocks(content, tag)
        .into_iter()
        .next()
        .filter(|b| !b.is_empty())
}

/// 提取最后一个非空标签块。
fn extract_last_nonempty_tagged_block(content: &str, tag: &str) -> Option<String> {
    extract_tagged_blocks(content, tag)
        .into_iter()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .next_back()
}

/// 提取所有 `=== TAG ===` 标签块。
fn extract_tagged_blocks(content: &str, tag: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if is_tag_line(lines[i].trim(), tag) {
            i += 1;
            // 跳过标签后紧跟的空行
            if i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            let mut block: Vec<&str> = Vec::new();
            while i < lines.len() && !is_any_tag_line(lines[i].trim()) {
                block.push(lines[i]);
                i += 1;
            }
            blocks.push(block.join("\n").trim().to_string());
        } else {
            i += 1;
        }
    }
    blocks
}

/// 判断行是否为指定标签行（大小写不敏感）。
fn is_tag_line(line: &str, tag: &str) -> bool {
    let line = line.trim();
    match line.strip_prefix("===").and_then(|s| s.strip_suffix("===")) {
        Some(inner) => inner.trim().eq_ignore_ascii_case(tag),
        None => false,
    }
}

/// 判断行是否为任意标签行（`=== [A-Z0-9_ ]+ ===`，大小写不敏感）。
fn is_any_tag_line(line: &str) -> bool {
    let line = line.trim();
    let inner = match line
        .strip_prefix("===")
        .and_then(|s| s.strip_suffix("==="))
    {
        Some(s) => s.trim(),
        None => return false,
    };
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ' ')
}

// ═══════════════════════════════════════════════════════════════
//  Markdown 回退解析
// ═══════════════════════════════════════════════════════════════

/// 提取首个 `# ` 一级标题。
fn extract_first_heading(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// 从 Markdown `## ` 标题提取章节标题（回退方案）。
fn extract_markdown_chapter_title(content: &str, number: u32) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            let rest = rest.trim();
            let title = strip_chapter_prefix(rest, number);
            if !title.is_empty() {
                return Some(title.to_string());
            }
            return Some(rest.to_string());
        }
    }
    None
}

/// 从 Markdown `## ` 标题提取章节正文（回退方案）。
fn extract_markdown_chapter_content(content: &str, _number: u32) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    // 找到首个 `## ` 标题
    while i < lines.len() {
        if lines[i].trim_start().starts_with("## ") {
            i += 1;
            break;
        }
        i += 1;
    }
    if i >= lines.len() {
        return None;
    }
    let mut block: Vec<&str> = Vec::new();
    while i < lines.len() {
        if lines[i].trim_start().starts_with("## ") {
            break;
        }
        block.push(lines[i]);
        i += 1;
    }
    let content = block.join("\n").trim().to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

/// 提取重复 `=== CHAPTER N TITLE ===` 标签后的内容。
/// 当模型意外把内容放在第二个 TITLE 标签下而非 CONTENT 标签时使用。
fn extract_duplicate_title_tagged_chapter_content(content: &str, number: u32) -> Option<String> {
    let tag = format!("CHAPTER {} TITLE", number);
    let blocks = extract_tagged_blocks(content, &tag);
    // 第二个块即重复标题后的内容
    blocks.into_iter().nth(1).filter(|b| !b.is_empty())
}

/// 剥离章节标题前缀（第N章 / Chapter N）。
fn strip_chapter_prefix(s: &str, number: u32) -> String {
    // zh: "第N章" 前缀
    let zh_prefix = format!("第{}章", number);
    if let Some(rest) = s.strip_prefix(&zh_prefix as &str) {
        return rest.trim().to_string();
    }
    // zh: "第 N 章" 带空格
    let zh_spaced = format!("第 {} 章", number);
    if let Some(rest) = s.strip_prefix(&zh_spaced as &str) {
        return rest.trim().to_string();
    }
    // en: "Chapter N" 前缀（大小写不敏感）
    let lower = s.to_lowercase();
    let en_prefix = format!("chapter {}", number);
    if let Some(rest) = lower.strip_prefix(&en_prefix) {
        // 剥离分隔符：: ： . - – — 及空格
        let stripped: &str = rest.trim_start_matches(|c: char| {
            matches!(c, ':' | '：' | '.' | '-' | '–' | '—' | ' ')
        });
        // 返回原始字符串对应位置
        let prefix_len = s.len() - rest.len();
        let sep_len = rest.len() - stripped.len();
        return s[prefix_len + sep_len..].trim().to_string();
    }
    s.trim().to_string()
}

/// 清理章节内容：去除代码围栏和残留标签行。
fn sanitize_chapter_content(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.lines().collect();
    // 去除开头代码围栏
    if let Some(first) = lines.first() {
        let f = first.trim().to_lowercase();
        if f == "```" || f == "```md" || f == "```markdown" {
            lines.remove(0);
        }
    }
    // 去除结尾代码围栏
    if let Some(last) = lines.last() {
        if last.trim() == "```" {
            lines.pop();
        }
    }
    // 去除残留的 === TAG === 行
    lines
        .into_iter()
        .filter(|line| !is_any_tag_line(line.trim()))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// 标题归一化：剥离 `#` 前缀和 `《》` 书名号。
fn normalize_title(raw: &str) -> String {
    for line in raw.lines() {
        let stripped = line.trim_start_matches('#').trim();
        if !stripped.is_empty() {
            let result = stripped
                .strip_prefix("《")
                .and_then(|s| s.strip_suffix("》"))
                .unwrap_or(stripped);
            return result.trim().to_string();
        }
    }
    String::new()
}

/// 章节标题归一化：剥离前缀，空则回退。
fn normalize_chapter_title(raw: &str, number: u32, language: Language) -> String {
    let normalized = normalize_title(raw);
    let stripped = strip_chapter_prefix(&normalized, number);
    let title = stripped.trim().to_string();
    if title.is_empty() {
        fallback_chapter_title(number, language)
    } else {
        title
    }
}

/// 未命名短篇回退标题。
fn untitled_short_title(language: Language) -> &'static str {
    match language {
        Language::En => "Untitled Short Story",
        Language::Zh => "未命名短篇",
    }
}

// ═══════════════════════════════════════════════════════════════
//  测试
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outline_with_tagged_title() {
        let raw = "=== SHORT_FICTION_PLAN_TITLE ===\n我的短篇\n=== SHORT_FICTION_PLAN ===\n故事方案内容";
        let outline = parse_outline(raw, Language::Zh);
        assert_eq!(outline.story_title, "我的短篇");
        assert!(outline.raw_content.contains("故事方案内容"));
    }

    #[test]
    fn parses_outline_fallback_to_heading() {
        let raw = "# 回家的路\n\n故事方案";
        let outline = parse_outline(raw, Language::Zh);
        assert_eq!(outline.story_title, "回家的路");
    }

    #[test]
    fn parses_batch_draft_with_tagged_chapters() {
        let raw = r#"=== SHORT_FICTION_TITLE ===
测试短篇
=== SHORT_FICTION_OPENING_HOOK ===
开篇钩子内容
=== CHAPTER 1 TITLE ===
第一章标题
=== CHAPTER 1 CONTENT ===
第一章正文内容
=== CHAPTER 2 TITLE ===
第二章标题
=== CHAPTER 2 CONTENT ===
第二章正文内容"#;
        let draft = parse_batch_draft(raw, 2, Language::Zh);
        assert_eq!(draft.story_title, "测试短篇");
        assert_eq!(draft.opening_hook.as_deref(), Some("开篇钩子内容"));
        assert_eq!(draft.chapters.len(), 2);
        assert_eq!(draft.chapters[0].number, 1);
        assert_eq!(draft.chapters[0].title, "第一章标题");
        assert_eq!(draft.chapters[0].content, "第一章正文内容");
        assert_eq!(draft.chapters[1].number, 2);
        assert_eq!(draft.chapters[1].content, "第二章正文内容");
    }

    #[test]
    fn parses_sales_package() {
        let raw = r#"=== SHORT_FICTION_PACKAGE_TITLE ===
包装标题
=== SHORT_FICTION_INTRO ===
简介内容
=== SHORT_FICTION_SELLING_POINTS ===
- 卖点1
- 卖点2
- 卖点3
=== SHORT_FICTION_COVER_PROMPT ===
封面提示词内容"#;
        let pkg = parse_sales_package(raw, "回退标题");
        assert_eq!(pkg.title, "包装标题");
        assert_eq!(pkg.intro, "简介内容");
        assert_eq!(pkg.selling_points.len(), 3);
        assert_eq!(pkg.selling_points[0], "卖点1");
        assert_eq!(pkg.cover_prompt, "封面提示词内容");
    }
}
