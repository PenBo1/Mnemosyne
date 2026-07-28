//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Helpers - 渲染、校验与计量辅助函数
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：草稿的 Markdown 渲染、空章检测、完整性校验、章节标题格式化、章节长度
//! 统计、max_tokens 估算，以及 CJK 字符判定与章节回退标题。

use crate::domain::pipeline::types::Language;
use crate::shared::error::AppError;

use super::types::ShortFictionBatchDraft;

// ═══════════════════════════════════════════════════════════════
//  渲染与验证辅助
// ═══════════════════════════════════════════════════════════════

/// 渲染草稿为 Markdown（用于审稿/修订上下文）。
pub fn render_draft_markdown(draft: &ShortFictionBatchDraft, language: Language) -> String {
    let hook_heading = match language {
        Language::En => "## Opening Hook",
        Language::Zh => "## Opening Hook",
    };
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("# {}", draft.story_title));
    if let Some(hook) = &draft.opening_hook {
        if !hook.trim().is_empty() {
            parts.push(format!("{}\n\n{}", hook_heading, hook));
        }
    }
    for chapter in &draft.chapters {
        let heading = format_chapter_heading(chapter.number, &chapter.title, language);
        parts.push(format!("## {}\n\n{}", heading, chapter.content));
    }
    parts.join("\n\n")
}

/// 查找内容为空的章节编号。
pub fn find_empty_chapters(draft: &ShortFictionBatchDraft) -> Vec<u32> {
    draft
        .chapters
        .iter()
        .filter(|c| c.content.trim().is_empty())
        .map(|c| c.number)
        .collect()
}

/// 校验草稿完整性。
pub fn validate_draft_for_final(
    draft: &ShortFictionBatchDraft,
    expected_chapters: Option<u32>,
) -> Result<(), AppError> {
    if let Some(expected) = expected_chapters {
        if draft.chapters.len() != expected as usize {
            return Err(AppError::invalid_format(format!(
                "Short-hit draft is incomplete; expected {} chapters, got {}.",
                expected,
                draft.chapters.len()
            )));
        }
    }
    let empty = find_empty_chapters(draft);
    if !empty.is_empty() {
        let list = empty
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::invalid_format(format!(
            "Short-hit draft is incomplete; empty chapters: {}.",
            list
        )));
    }
    Ok(())
}

/// 格式化章节标题（用于落盘 Markdown）。
pub fn format_chapter_heading(number: u32, title: &str, language: Language) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return fallback_chapter_title(number, language);
    }
    match language {
        Language::En => {
            let pattern = format!(r"(?i)^Chapter\s*{}\b", number);
            if regex::Regex::new(&pattern)
                .map(|re| re.is_match(trimmed))
                .unwrap_or(false)
            {
                trimmed.to_string()
            } else {
                format!("Chapter {}: {}", number, trimmed)
            }
        }
        Language::Zh => {
            let pattern = format!(r"^第\s*{}\s*章", number);
            if regex::Regex::new(&pattern)
                .map(|re| re.is_match(trimmed))
                .unwrap_or(false)
            {
                trimmed.to_string()
            } else {
                format!("第{}章 {}", number, trimmed)
            }
        }
    }
}

/// 统计章节长度（zh=中文字符数, en=单词数）。
pub fn count_chapter_length(content: &str, language: Language) -> u32 {
    match language {
        Language::Zh => content.chars().filter(|c| is_cjk_char(*c)).count() as u32,
        Language::En => content.split_whitespace().count() as u32,
    }
}

/// 估算短篇写作 max_tokens。
/// 公式：max(12288, ceil(chapters * chars_per_chapter * 2.2) + 4096)
pub fn estimate_short_fiction_max_tokens(chapter_count: u32, chars_per_chapter: u32) -> u64 {
    let product = (chapter_count as u64) * (chars_per_chapter as u64) * 22;
    let estimated = product.div_ceil(10) + 4096;
    std::cmp::max(12288, estimated)
}

/// 章节回退标题。
pub(super) fn fallback_chapter_title(number: u32, language: Language) -> String {
    match language {
        Language::En => format!("Chapter {}", number),
        Language::Zh => format!("第{}章", number),
    }
}

/// 判断字符是否为 CJK 汉字。
fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    matches!(
        code,
        0x4E00..=0x9FFF       // CJK Unified Ideographs
        | 0x3400..=0x4DBF     // CJK Extension A
        | 0x20000..=0x2A6DF   // CJK Extension B
        | 0x2A700..=0x2B73F   // CJK Extension C
        | 0x2B740..=0x2B81F   // CJK Extension D
        | 0x2B820..=0x2CEAF   // CJK Extension E
        | 0xF900..=0xFAFF     // CJK Compatibility Ideographs
        | 0x2F800..=0x2FA1F   // CJK Compatibility Ideographs Supplement
    )
}

// ═══════════════════════════════════════════════════════════════
//  测试
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{ShortFictionBatchDraft, ShortFictionChapter};
    use crate::domain::pipeline::types::Language;

    #[test]
    fn find_empty_chapters_detects_missing_content() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![
                ShortFictionChapter {
                    number: 1,
                    title: "第一章".to_string(),
                    content: "内容".to_string(),
                    char_count: 2,
                },
                ShortFictionChapter {
                    number: 2,
                    title: "第二章".to_string(),
                    content: "   ".to_string(),
                    char_count: 0,
                },
            ],
            raw_content: String::new(),
        };
        let empty = find_empty_chapters(&draft);
        assert_eq!(empty, vec![2]);
    }

    #[test]
    fn validate_draft_rejects_empty_chapters() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "第一章".to_string(),
                content: String::new(),
                char_count: 0,
            }],
            raw_content: String::new(),
        };
        assert!(validate_draft_for_final(&draft, Some(1)).is_err());
    }

    #[test]
    fn validate_draft_rejects_wrong_chapter_count() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "第一章".to_string(),
                content: "内容".to_string(),
                char_count: 2,
            }],
            raw_content: String::new(),
        };
        assert!(validate_draft_for_final(&draft, Some(2)).is_err());
    }

    #[test]
    fn count_chapter_length_zh_counts_cjk() {
        let content = "你好世界hello 123";
        assert_eq!(count_chapter_length(content, Language::Zh), 4);
    }

    #[test]
    fn count_chapter_length_en_counts_words() {
        let content = "hello world foo bar";
        assert_eq!(count_chapter_length(content, Language::En), 4);
    }

    #[test]
    fn estimate_max_tokens_respects_floor() {
        let tokens = estimate_short_fiction_max_tokens(1, 100);
        assert_eq!(tokens, 12288);
    }

    #[test]
    fn estimate_max_tokens_scales_with_chapters() {
        let tokens = estimate_short_fiction_max_tokens(12, 1000);
        // 12 * 1000 * 2.2 = 26400, ceil = 26400, + 4096 = 30496
        assert_eq!(tokens, 30496);
    }

    #[test]
    fn format_chapter_heading_zh_adds_prefix() {
        let heading = format_chapter_heading(3, "暗流", Language::Zh);
        assert_eq!(heading, "第3章 暗流");
    }

    #[test]
    fn format_chapter_heading_zh_preserves_existing_prefix() {
        let heading = format_chapter_heading(3, "第3章 暗流", Language::Zh);
        assert_eq!(heading, "第3章 暗流");
    }

    #[test]
    fn format_chapter_heading_en_adds_prefix() {
        let heading = format_chapter_heading(5, "The Storm", Language::En);
        assert_eq!(heading, "Chapter 5: The Storm");
    }

    #[test]
    fn render_draft_markdown_includes_all_chapters() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试短篇".to_string(),
            opening_hook: Some("钩子".to_string()),
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "开局".to_string(),
                content: "正文".to_string(),
                char_count: 2,
            }],
            raw_content: String::new(),
        };
        let md = render_draft_markdown(&draft, Language::Zh);
        assert!(md.contains("# 测试短篇"));
        assert!(md.contains("## Opening Hook"));
        assert!(md.contains("## 第1章 开局"));
        assert!(md.contains("正文"));
    }
}
