// State Bootstrap。
//
// 职责：从 markdown 真相文件（current_state.md / pending_hooks.md / chapter_summaries.md）
// 引导出 JSON 加速索引。首次访问书籍状态时调用。
//
// 简化说明：完整版本包含完整的 markdown 解析（parseCurrentStateFacts / parsePendingHooksMarkdown /
// parseChapterSummariesMarkdown），这些解析器依赖 story-markdown.ts 工具模块。
// Rust 版先实现最小骨架：若 JSON 不存在，创建空索引 + 记录 warning。
// 完整的 markdown → JSON 解析在 agents 阶段补充（settler agent 输出 delta 后由 reducer 更新索引，
// bootstrap 仅用于首次初始化或从纯 markdown 恢复的场景）。

use std::path::Path;

use super::super::types::Language;
use super::types::*;
use super::store::{load_runtime_state_snapshot, save_runtime_state_snapshot};
use crate::shared::error::AppError;

/// bootstrap 结果
pub struct BootstrapResult {
    pub created_files: Vec<String>,
    pub warnings: Vec<String>,
    pub manifest: StateManifest,
}

/// 从 markdown 真相文件引导出 JSON 索引。
/// 若 JSON 索引已存在，直接加载返回；否则创建空索引。
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

    // 首次引导：创建空索引（language 从 book.json 推断，fallback 到参数）
    let resolved_language = resolve_language(book_dir, fallback_language);
    let new_snapshot = RuntimeStateSnapshot::empty(resolved_language);
    save_runtime_state_snapshot(book_dir, &new_snapshot)?;

    let mut warnings = Vec::new();
    // 检查 markdown 真相文件是否存在，若存在则记录 warning（提示需要手动解析或后续 agent 会填充）
    let story_dir = book_dir.join("story");
    let markdown_files = ["current_state.md", "pending_hooks.md", "chapter_summaries.md"];
    for md_file in &markdown_files {
        if story_dir.join(md_file).exists() {
            warnings.push(format!(
                "markdown truth file '{}' exists but JSON index was empty; \
                 structured state will be populated by settler agent on next chapter write",
                md_file
            ));
        }
    }

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
