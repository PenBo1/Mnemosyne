// State Bootstrap。
//
// 职责：从 markdown 真相文件（current_state.md / pending_hooks.md / chapter_summaries.md）
// 引导出 JSON 加速索引。首次访问书籍状态时调用。
//
// 已实现：完整的 markdown → JSON 解析（parse_current_state_facts /
// parse_pending_hooks_markdown / parse_chapter_summaries_markdown）。

use std::path::Path;

use super::super::types::Language;
use super::types::*;
use super::store::{load_runtime_state_snapshot, save_runtime_state_snapshot};
use crate::domain::pipeline::utils::story_markdown;
use crate::shared::error::AppError;

/// bootstrap 结果
pub struct BootstrapResult {
    pub created_files: Vec<String>,
    pub warnings: Vec<String>,
    pub manifest: StateManifest,
}

/// 从 markdown 真相文件引导出 JSON 索引。
///
/// 流程：
/// 1. 若 manifest.json 已存在 → 直接加载返回
/// 2. 否则读取 markdown 真相文件，解析为结构化数据
/// 3. 写入 JSON 索引文件（manifest.json + 3 个 .json 文件）
/// 4. markdown 缺失时创建空索引 + 记录 warning
pub fn bootstrap_structured_state_from_markdown(
    book_dir: &Path,
    fallback_language: Language,
) -> Result<BootstrapResult, AppError> {
    let snapshot = load_runtime_state_snapshot(book_dir)?;

    // 若 manifest 已存在，说明已引导过，直接返回
    let manifest_exists = book_dir
        .join("story")
        .join("state")
        .join("manifest.json")
        .exists();

    if manifest_exists {
        return Ok(BootstrapResult {
            created_files: Vec::new(),
            warnings: Vec::new(),
            manifest: snapshot.manifest,
        });
    }

    let resolved_language = resolve_language(book_dir, fallback_language);
    let story_dir = book_dir.join("story");

    // 读取 markdown 真相文件（缺失视为空字符串）
    let current_state_md = read_md_or_empty(&story_dir, "current_state.md");
    let pending_hooks_md = read_md_or_empty(&story_dir, "pending_hooks.md");
    let chapter_summaries_md = read_md_or_empty(&story_dir, "chapter_summaries.md");

    let mut warnings = Vec::new();
    let last_applied_chapter = infer_last_chapter(&chapter_summaries_md);

    // 解析 markdown → 结构化数据
    let facts = if current_state_md.trim().is_empty() {
        Vec::new()
    } else {
        let parsed = story_markdown::parse_current_state_facts(
            &current_state_md,
            last_applied_chapter,
            resolved_language,
        );
        if parsed.is_empty() {
            warnings.push("current_state.md exists but no facts parsed".to_string());
        }
        parsed
    };

    let hooks = if pending_hooks_md.trim().is_empty() {
        Vec::new()
    } else {
        let parsed = story_markdown::parse_pending_hooks_markdown(&pending_hooks_md, resolved_language);
        if parsed.is_empty() {
            warnings.push("pending_hooks.md exists but no hooks parsed".to_string());
        }
        parsed
    };

    let summary_rows = if chapter_summaries_md.trim().is_empty() {
        Vec::new()
    } else {
        let parsed = story_markdown::parse_chapter_summaries_markdown(&chapter_summaries_md);
        if parsed.is_empty() {
            warnings.push("chapter_summaries.md exists but no rows parsed".to_string());
        }
        parsed
    };

    let new_snapshot = RuntimeStateSnapshot {
        manifest: StateManifest {
            schema_version: 2,
            language: resolved_language,
            last_applied_chapter,
            projection_version: 1,
            migration_warnings: Vec::new(),
        },
        current_state: CurrentStateState {
            chapter: last_applied_chapter,
            facts,
        },
        hooks: HooksState { hooks },
        chapter_summaries: ChapterSummariesState { rows: summary_rows },
    };
    save_runtime_state_snapshot(book_dir, &new_snapshot)?;

    Ok(BootstrapResult {
        created_files: vec![
            "story/state/manifest.json".into(),
            "story/state/current_state.json".into(),
            "story/state/hooks.json".into(),
            "story/state/chapter_summaries.json".into(),
        ],
        warnings,
        manifest: new_snapshot.manifest,
    })
}

/// 检测 markdown 真相文件是否已存在（用于判断是否需要引导）
pub fn has_markdown_truth_files(book_dir: &Path) -> bool {
    let story_dir = book_dir.join("story");
    ["current_state.md", "pending_hooks.md", "chapter_summaries.md"]
        .iter()
        .any(|f| story_dir.join(f).exists())
}

/// fallback language 解析：若 book.json 无 language 字段，用 fallback
pub fn resolve_language(book_dir: &Path, fallback: Language) -> Language {
    let book_json = book_dir.join("book.json");
    match std::fs::read_to_string(&book_json) {
        Ok(content) => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(lang) = value.get("language").and_then(|v| v.as_str()) {
                    return match lang {
                        "en" => Language::En,
                        _ => Language::Zh,
                    };
                }
            }
            fallback
        }
        Err(_) => fallback,
    }
}

// ── 内部辅助 ─────────────────────────────────────────────────

fn read_md_or_empty(story_dir: &Path, file_name: &str) -> String {
    std::fs::read_to_string(story_dir.join(file_name)).unwrap_or_default()
}

/// 从 chapter_summaries.md 表中提取最大章节号作为 last_applied_chapter。
/// 若无法解析返回 0。
fn infer_last_chapter(summaries_md: &str) -> u32 {
    let rows = story_markdown::parse_chapter_summaries_markdown(summaries_md);
    rows.iter().map(|r| r.chapter).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_last_chapter_from_summaries() {
        let md = "| 章节 | 标题 |\n|---|---|\n| 1 | a |\n| 5 | b |\n| 3 | c |";
        assert_eq!(infer_last_chapter(md), 5);
    }

    #[test]
    fn infer_last_chapter_empty() {
        assert_eq!(infer_last_chapter(""), 0);
    }

    #[test]
    fn resolves_language_falls_back_when_book_json_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(resolve_language(tmp.path(), Language::Zh), Language::Zh);
        assert_eq!(resolve_language(tmp.path(), Language::En), Language::En);
    }

    #[test]
    fn resolves_language_from_book_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("book.json"),
            r#"{"language": "en"}"#,
        )
        .unwrap();
        assert_eq!(resolve_language(tmp.path(), Language::Zh), Language::En);
    }
}
