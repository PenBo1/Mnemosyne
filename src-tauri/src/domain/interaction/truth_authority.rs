// 真相权威分类（TruthAuthority classification）。
//
// 真相文件白名单（防路径穿越）：
// - author_intent.md     → Direction
// - current_focus.md     → Direction
// - story_bible.md       → Foundation
// - volume_outline.md    → Foundation
// - book_rules.md        → Rules
// - current_state.md     → RuntimeTruth
// - pending_hooks.md     → RuntimeTruth
// - chapter_summaries.md → Memory
//
// outline/* 和 roles/* 子路径不在白名单内（编辑入口只接受顶层文件名），
// 避免通过 file_name 字段构造穿越路径。

use crate::shared::error::AppError;
use crate::domain::interaction::types::TruthAuthority;

/// 白名单：允许编辑的真相文件名（小写、带 .md 后缀）。
const NORMALIZED_TRUTH_FILES: &[&str] = &[
    "author_intent.md",
    "current_focus.md",
    "story_bible.md",
    "volume_outline.md",
    "book_rules.md",
    "current_state.md",
    "pending_hooks.md",
    "chapter_summaries.md",
];

/// 规范化真相文件名：trim + lowercase + 强制 .md 后缀。
/// 若规范化后的文件名不在白名单内，返回 None。
pub fn normalize_truth_file_name(name: &str) -> Option<String> {
    let trimmed = name.trim().to_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    // 拒绝任何路径分隔符或 .. —— 单文件名不能构造穿越
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return None;
    }
    let normalized = if trimmed.ends_with(".md") {
        trimmed
    } else {
        format!("{}.md", trimmed)
    };
    if NORMALIZED_TRUTH_FILES.contains(&normalized.as_str()) {
        Some(normalized)
    } else {
        None
    }
}

/// 根据真相文件名分类权威级别。
pub fn classify_truth_authority(file_name: &str) -> Option<TruthAuthority> {
    let normalized = normalize_truth_file_name(file_name)?;
    Some(match normalized.as_str() {
        "author_intent.md" | "current_focus.md" => TruthAuthority::Direction,
        "story_bible.md" | "volume_outline.md" => TruthAuthority::Foundation,
        "book_rules.md" => TruthAuthority::Rules,
        "current_state.md" | "pending_hooks.md" => TruthAuthority::RuntimeTruth,
        "chapter_summaries.md" => TruthAuthority::Memory,
        _ => return None,
    })
}

/// 断言真相文件名安全（白名单校验，防路径穿越）。
/// 用于 IPC 层入参校验，失败返回 AppError::path_traversal。
pub fn assert_safe_truth_file_name(name: &str) -> Result<(), AppError> {
    if normalize_truth_file_name(name).is_none() {
        return Err(AppError::invalid_input(format!(
            "非法真相文件名: {}（白名单: {}）",
            name,
            NORMALIZED_TRUTH_FILES.join(" / ")
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basic_files() {
        assert_eq!(
            normalize_truth_file_name("author_intent").as_deref(),
            Some("author_intent.md")
        );
        assert_eq!(
            normalize_truth_file_name("Author_Intent.md").as_deref(),
            Some("author_intent.md")
        );
        assert_eq!(
            normalize_truth_file_name(" current_focus.md ").as_deref(),
            Some("current_focus.md")
        );
    }

    #[test]
    fn normalize_rejects_unknown_file() {
        assert!(normalize_truth_file_name("random_file.md").is_none());
        assert!(normalize_truth_file_name("").is_none());
    }

    #[test]
    fn normalize_rejects_traversal() {
        assert!(normalize_truth_file_name("../etc/passwd").is_none());
        assert!(normalize_truth_file_name("outline/story_frame.md").is_none());
        assert!(normalize_truth_file_name("roles\\hero.md").is_none());
        assert!(normalize_truth_file_name("a..b.md").is_none());
    }

    #[test]
    fn classify_matches_normalize() {
        assert_eq!(
            classify_truth_authority("author_intent"),
            Some(TruthAuthority::Direction)
        );
        assert_eq!(
            classify_truth_authority("current_focus.md"),
            Some(TruthAuthority::Direction)
        );
        assert_eq!(
            classify_truth_authority("story_bible"),
            Some(TruthAuthority::Foundation)
        );
        assert_eq!(
            classify_truth_authority("volume_outline.md"),
            Some(TruthAuthority::Foundation)
        );
        assert_eq!(
            classify_truth_authority("book_rules"),
            Some(TruthAuthority::Rules)
        );
        assert_eq!(
            classify_truth_authority("current_state.md"),
            Some(TruthAuthority::RuntimeTruth)
        );
        assert_eq!(
            classify_truth_authority("pending_hooks"),
            Some(TruthAuthority::RuntimeTruth)
        );
        assert_eq!(
            classify_truth_authority("chapter_summaries.md"),
            Some(TruthAuthority::Memory)
        );
        assert_eq!(classify_truth_authority("unknown"), None);
    }

    #[test]
    fn assert_safe_rejects_traversal() {
        assert!(assert_safe_truth_file_name("../etc/passwd").is_err());
        assert!(assert_safe_truth_file_name("author_intent").is_ok());
        assert!(assert_safe_truth_file_name("CURRENT_FOCUS.md").is_ok());
    }
}
