// RuntimeState Store。
//
// 职责：load/save RuntimeStateSnapshot 的 4 个 JSON 文件（state/ 目录下）。
// 文件布局：
//   <book_dir>/story/state/
//     ├── manifest.json
//     ├── current_state.json
//     ├── hooks.json
//     └── chapter_summaries.json
//
// 简化说明：完整版本在 load 时会调用 bootstrapStructuredStateFromMarkdown 确保从 markdown 引导。
// Rust 版的 bootstrap 作为独立模块，store 只负责 JSON 读写；若 JSON 不存在，返回 empty snapshot。
// 调用方（pipeline runner）在 init_book 阶段会先调用 bootstrap 建立索引。

use std::path::{Path, PathBuf};

use super::super::types::Language;
use super::types::*;
use crate::shared::error::AppError;

/// state 目录下的 4 个 JSON 文件路径
struct StatePaths {
    manifest: PathBuf,
    current_state: PathBuf,
    hooks: PathBuf,
    chapter_summaries: PathBuf,
}

impl StatePaths {
    fn new(book_dir: &Path) -> Self {
        let state_dir = book_dir.join("story").join("state");
        Self {
            manifest: state_dir.join("manifest.json"),
            current_state: state_dir.join("current_state.json"),
            hooks: state_dir.join("hooks.json"),
            chapter_summaries: state_dir.join("chapter_summaries.json"),
        }
    }

    fn ensure_dir(&self) -> Result<(), AppError> {
        if let Some(parent) = self.manifest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

/// 加载 snapshot。若 JSON 不存在返回 empty snapshot（language 从 book.json 推断）。
pub fn load_runtime_state_snapshot(book_dir: &Path) -> Result<RuntimeStateSnapshot, AppError> {
    let paths = StatePaths::new(book_dir);

    // 若 manifest.json 不存在，返回 empty snapshot
    if !paths.manifest.exists() {
        let language = resolve_book_language(book_dir);
        return Ok(RuntimeStateSnapshot::empty(language));
    }

    let manifest: StateManifest = read_json(&paths.manifest)?;
    let current_state: CurrentStateState = read_json_or_default(&paths.current_state)?;
    let hooks: HooksState = read_json_or_default(&paths.hooks)?;
    let chapter_summaries: ChapterSummariesState = read_json_or_default(&paths.chapter_summaries)?;

    let snapshot = RuntimeStateSnapshot {
        manifest,
        current_state,
        hooks,
        chapter_summaries,
    };

    Ok(snapshot)
}

/// 保存 snapshot 到 4 个 JSON 文件
pub fn save_runtime_state_snapshot(
    book_dir: &Path,
    snapshot: &RuntimeStateSnapshot,
) -> Result<(), AppError> {
    let paths = StatePaths::new(book_dir);
    paths.ensure_dir()?;

    write_json(&paths.manifest, &snapshot.manifest)?;
    write_json(&paths.current_state, &snapshot.current_state)?;
    write_json(&paths.hooks, &snapshot.hooks)?;
    write_json(&paths.chapter_summaries, &snapshot.chapter_summaries)?;

    Ok(())
}

/// 从 book.json 推断语言
fn resolve_book_language(book_dir: &Path) -> Language {
    let book_json = book_dir.join("book.json");
    match std::fs::read_to_string(&book_json) {
        Ok(content) => {
            let value: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => return Language::Zh,
            };
            match value.get("language").and_then(|v| v.as_str()) {
                Some("en") => Language::En,
                _ => Language::Zh,
            }
        }
        Err(_) => Language::Zh,
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| AppError::file_read_error(path.display().to_string()))?;
    serde_json::from_str(&content).map_err(|e| {
        AppError::invalid_format(format!(
            "Failed to parse {}: {}",
            path.display(),
            e
        ))
    })
}

fn read_json_or_default<T: serde::de::DeserializeOwned + Default>(
    path: &Path,
) -> Result<T, AppError> {
    if !path.exists() {
        return Ok(T::default());
    }
    read_json(path)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)
        .map_err(|_| AppError::file_write_error(path.display().to_string()))?;
    Ok(())
}
