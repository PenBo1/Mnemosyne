// LengthNormalizer Agent。
//
// 职责：当章节字数偏离目标区间时，单次修正（compress 或 expand）使其落在软边界内。
// 保留原有事实/关键钩子/角色名，不引入新支线。
//
// prompt 策略：单次修正模式 + 事实保留约束 + 纯文本输出。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

/// LengthNormalizer 输出
#[derive(Debug, Clone)]
pub struct NormalizeOutput {
    pub normalized_content: String,
    pub final_count: u32,
    pub applied: bool,
}

/// 修正模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalizeMode {
    Compress,
    Expand,
    None,
}

/// 修正章节字数。
pub async fn normalize_chapter(
    engine: &AgentEngine,
    chapter_content: &str,
    target_words: u32,
    soft_min: u32,
    soft_max: u32,
) -> Result<NormalizeOutput, AppError> {
    let current_count = count_words(chapter_content);
    let mode = resolve_mode(current_count, soft_min, soft_max);

    // 字数在软边界内，无需修正
    if mode == NormalizeMode::None {
        return Ok(NormalizeOutput {
            normalized_content: chapter_content.to_string(),
            final_count: current_count,
            applied: false,
        });
    }

    let system_prompt = build_system_prompt(mode, target_words, soft_min, soft_max);
    let user_message = build_user_message(chapter_content, current_count, mode);

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let sanitized = sanitize_wrapper(&response);

    // 安全回退：如果输出截断或跨越相反硬边界，保留原文
    let new_count = count_words(&sanitized);
    if !is_safe_output(&sanitized, new_count, mode, soft_min, soft_max) {
        return Ok(NormalizeOutput {
            normalized_content: chapter_content.to_string(),
            final_count: current_count,
            applied: false,
        });
    }

    Ok(NormalizeOutput {
        normalized_content: sanitized,
        final_count: new_count,
        applied: true,
    })
}

/// 判断修正模式：超上限压缩、低于下限扩写、区间内不处理
fn resolve_mode(current_count: u32, soft_min: u32, soft_max: u32) -> NormalizeMode {
    if current_count > soft_max {
        NormalizeMode::Compress
    } else if current_count < soft_min {
        NormalizeMode::Expand
    } else {
        NormalizeMode::None
    }
}

/// 字数统计（非空白字符数）
fn count_words(content: &str) -> u32 {
    content.chars().filter(|c| !c.is_whitespace()).count() as u32
}

/// 判断输出是否安全：非空且未跨越相反硬边界
fn is_safe_output(
    content: &str,
    count: u32,
    mode: NormalizeMode,
    soft_min: u32,
    soft_max: u32,
) -> bool {
    if content.trim().is_empty() {
        return false; // 输出截断
    }
    match mode {
        // 压缩后不能低于下限（跨越相反边界）
        NormalizeMode::Compress => count >= soft_min,
        // 扩写后不能高于上限（跨越相反边界）
        NormalizeMode::Expand => count <= soft_max,
        NormalizeMode::None => true,
    }
}

fn build_system_prompt(mode: NormalizeMode, target_words: u32, soft_min: u32, soft_max: u32) -> String {
    let mode_desc = match mode {
        NormalizeMode::Compress => format!(
            "The current chapter exceeds the {soft_max}-word ceiling and must be compressed into the {soft_min}-{soft_max} word band. While compressing, preserve every fact, every key hook, and every character name; cut redundant description and transitional passages; introduce no new subplots."
        ),
        NormalizeMode::Expand => format!(
            "The current chapter falls short of the {soft_min}-word floor and must be expanded into the {soft_min}-{soft_max} word band. While expanding, add sensory detail, action description, and dialogue layers; introduce no new subplots or new characters."
        ),
        NormalizeMode::None => String::new(),
    };

    format!(
        r###"<identity>
You are a word-count correction editor for web fiction. Your task is to apply a single correction pass that brings the chapter's word count inside the target band.
</identity>

## This Task
{mode_desc}

<correction_principles>
- Target word count: {target_words} words
- Soft boundary: {soft_min}-{soft_max} words
- Preserve every existing fact, key hook, and character name.
- Introduce no new subplots, no new characters, no new events.
- Preserve the original voice and narrative rhythm.
</correction_principles>

## Output Format

Emit the corrected full prose directly — no commentary, no code-fence markers, no prefixes or suffixes."###,
        mode_desc = mode_desc,
        target_words = target_words,
        soft_min = soft_min,
        soft_max = soft_max,
    )
}

fn build_user_message(chapter_content: &str, current_count: u32, _mode: NormalizeMode) -> String {
    format!(
        r###"Current word count: {current_count} words

## Prose to Correct
{chapter_content}"###,
        current_count = current_count,
        chapter_content = chapter_content,
    )
}

/// Strip wrapper lines (code fences, `#` commentary, "下面是..." / "here is..." prefixes) from LLM output.
fn sanitize_wrapper(content: &str) -> String {
    let text = strip_code_fence(content);

    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return text;
    }

    // Find the first non-wrapper line.
    let start = lines
        .iter()
        .position(|l| !is_wrapper_line(l))
        .unwrap_or(lines.len());
    // Find the last non-wrapper line.
    let end = lines
        .iter()
        .rposition(|l| !is_wrapper_line(l))
        .map(|i| i + 1)
        .unwrap_or(0);

    if start >= end {
        return text.trim().to_string();
    }

    lines[start..end].join("\n").trim().to_string()
}

/// Determine whether a line is a wrapper (empty, starts with `#`, starts with "下面是" / "here is", or starts with ```).
fn is_wrapper_line(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#') || t.starts_with("下面是") || t.starts_with("```")
}

/// 去除可能的 ``` 代码块包裹
fn strip_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let after_open = &trimmed[3..];
    let inner_start = after_open.find('\n').map(|p| p + 1).unwrap_or(0);
    let inner = &after_open[inner_start..];
    let inner = inner.trim_end();
    match inner.strip_suffix("```") {
        Some(rest) => rest.trim().to_string(),
        None => inner.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_compress_mode_when_over_max() {
        assert_eq!(resolve_mode(4000, 2550, 3450), NormalizeMode::Compress);
    }

    #[test]
    fn resolves_expand_mode_when_under_min() {
        assert_eq!(resolve_mode(2000, 2550, 3450), NormalizeMode::Expand);
    }

    #[test]
    fn resolves_none_mode_when_in_range() {
        assert_eq!(resolve_mode(3000, 2550, 3450), NormalizeMode::None);
    }

    #[test]
    fn counts_non_whitespace_chars() {
        assert_eq!(count_words("你好 world 123"), 10);
        assert_eq!(count_words("  \n\t  "), 0);
    }

    #[test]
    fn sanitizes_wrapper_lines() {
        let content = "```\n# 说明\n下面是修正后的内容：\n这是正文。\n```";
        let result = sanitize_wrapper(content);
        assert_eq!(result, "这是正文。");
    }

    #[test]
    fn sanitizes_wrapper_without_fence() {
        let content = "# 说明\n下面是修正后的正文：\n这是正文内容。";
        let result = sanitize_wrapper(content);
        assert_eq!(result, "这是正文内容。");
    }

    #[test]
    fn sanitizes_keeps_content_without_wrapper() {
        let content = "这是第一段。\n\n这是第二段。";
        let result = sanitize_wrapper(content);
        assert_eq!(result, "这是第一段。\n\n这是第二段。");
    }

    #[test]
    fn safe_output_rejects_empty() {
        assert!(!is_safe_output("", 0, NormalizeMode::Compress, 2550, 3450));
        assert!(!is_safe_output("   \n  ", 0, NormalizeMode::Expand, 2550, 3450));
    }

    #[test]
    fn safe_output_rejects_crossing_opposite_boundary() {
        // 压缩后低于下限 → 不安全
        assert!(!is_safe_output("内容", 2000, NormalizeMode::Compress, 2550, 3450));
        // 扩写后高于上限 → 不安全
        assert!(!is_safe_output("内容", 4000, NormalizeMode::Expand, 2550, 3450));
    }

    #[test]
    fn safe_output_accepts_in_range() {
        assert!(is_safe_output("内容", 3000, NormalizeMode::Compress, 2550, 3450));
        assert!(is_safe_output("内容", 3000, NormalizeMode::Expand, 2550, 3450));
    }
}
