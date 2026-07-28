//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner Helpers - 内部辅助方法
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：日志 / 章节索引 / 快照 / 真相文件落盘 / 目录拷贝
//!
//! 注意：这些方法使用 `pub(super)` 可见性，以便 pipeline_runner 的其它子模块
//! （foundation / plan_write / audit_revise / write_next）能够调用它们。
//! 它们对外部模块不可见，未改变 PipelineRunner 的公开 API。

use std::path::Path;

use crate::domain::pipeline::agents::writer;
use crate::domain::pipeline::state::store;
use crate::domain::pipeline::types::{ChapterMeta, Language};
use crate::domain::pipeline::utils::text_parse::extract_section;
use crate::shared::error::AppError;

use super::PipelineRunner;

impl PipelineRunner {
    #[allow(dead_code)]
    pub(super) fn log_stage(&self, language: Language, message: &str) {
        let prefix = match language { Language::Zh => "阶段：", Language::En => "Stage: " };
        tracing::info!("[pipeline] {}{}", prefix, message);
    }

    #[allow(dead_code)]
    pub(super) fn log_stage_with_context(&self, language: Language, message: &str, book_id: &str, chapter: Option<u32>) {
        let prefix = match language { Language::Zh => "阶段：", Language::En => "Stage: " };
        match chapter {
            Some(ch) => {
                tracing::info!(
                    book_id = %book_id,
                    chapter = ch,
                    "[pipeline] {}{}", prefix, message
                );
            }
            None => {
                tracing::info!(
                    book_id = %book_id,
                    "[pipeline] {}{}", prefix, message
                );
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn log_stage_start(&self, book_id: &str, stage: &str) {
        tracing::info!(
            book_id = %book_id,
            stage = stage,
            "[pipeline] stage started"
        );
    }

    #[allow(dead_code)]
    pub(super) fn log_stage_complete(&self, book_id: &str, stage: &str, duration_ms: u64) {
        tracing::info!(
            book_id = %book_id,
            stage = stage,
            duration_ms = duration_ms,
            "[pipeline] stage completed"
        );
    }

    #[allow(dead_code)]
    pub(super) fn log_error(&self, book_id: &str, stage: &str, error: &AppError, retry_attempt: Option<u32>) {
        match retry_attempt {
            Some(attempt) => {
                tracing::error!(
                    book_id = %book_id,
                    stage = stage,
                    retry_attempt = attempt,
                    error = %error,
                    "[pipeline] stage failed (will retry)"
                );
            }
            None => {
                tracing::error!(
                    book_id = %book_id,
                    stage = stage,
                    error = %error,
                    "[pipeline] stage failed"
                );
            }
        }
    }

    pub(super) fn get_next_chapter_number(&self, book_dir: &Path) -> Result<u32, AppError> {
        let index = self.load_chapter_index(book_dir)?;
        Ok(index.iter().map(|c| c.number).max().unwrap_or(0) + 1)
    }

    pub(super) fn load_chapter_index(&self, book_dir: &Path) -> Result<Vec<ChapterMeta>, AppError> {
        let path = book_dir.join("chapters.json");
        if !path.exists() { return Ok(vec![]); }
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| AppError::invalid_format(format!("chapters.json parse: {}", e)))
    }

    pub(super) fn save_chapter_index(&self, book_dir: &Path, index: &[ChapterMeta]) -> Result<(), AppError> {
        let path = book_dir.join("chapters.json");
        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub(super) fn snapshot_state(&self, book_dir: &Path, chapter_number: u32) -> Result<(), AppError> {
        let mut snapshot = store::load_runtime_state_snapshot(book_dir)?;
        // 更新 manifest.last_applied_chapter
        snapshot.manifest.last_applied_chapter = chapter_number;
        store::save_runtime_state_snapshot(book_dir, &snapshot)?;
        Ok(())
    }

    pub(super) fn read_chapter_content(&self, book_dir: &Path, chapter_number: u32) -> Result<Option<String>, AppError> {
        let chapters_dir = book_dir.join("chapters");
        let padded = format!("{:04}", chapter_number);
        if !chapters_dir.exists() { return Ok(None); }
        for entry in std::fs::read_dir(&chapters_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&padded) && name.ends_with(".md") {
                let content = std::fs::read_to_string(entry.path())?;
                // 去除首行标题（仅当首行以 "# " 开头时），否则保留全部正文
                let without_heading: String = if content.lines().next().is_some_and(|l| l.starts_with("# ")) {
                    content.lines().skip(1).collect::<Vec<_>>().join("\n")
                } else {
                    content
                };
                return Ok(Some(without_heading.trim().to_string()));
            }
        }
        Ok(None)
    }

    pub(super) fn read_recent_chapters(&self, book_dir: &Path, current_chapter: u32, count: u32) -> String {
        let mut chapters = Vec::new();
        for i in (1..current_chapter).rev().take(count as usize) {
            if let Ok(Some(content)) = self.read_chapter_content(book_dir, i) {
                chapters.push(format!("## 第{}章\n\n{}", i, content));
            }
        }
        chapters.join("\n\n---\n\n")
    }

    pub(super) fn get_chapter_title(&self, book_dir: &Path, chapter_number: u32) -> String {
        let index = self.load_chapter_index(book_dir).unwrap_or_default();
        index.iter()
            .find(|c| c.number == chapter_number)
            .map(|c| c.title.clone())
            .unwrap_or_default()
    }

    pub(super) fn trim_recent_summaries(&self, content: &str, n: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut header_lines: Vec<&str> = Vec::new();
        let mut data_lines: Vec<&str> = Vec::new();
        for line in &lines {
            if line.starts_with('|') {
                if header_lines.is_empty() || line.contains("---") {
                    header_lines.push(line);
                } else {
                    data_lines.push(line);
                }
            }
        }
        if data_lines.is_empty() { return String::new(); }
        let start = data_lines.len().saturating_sub(n);
        let recent = &data_lines[start..];
        let mut result = header_lines.join("\n");
        if !result.is_empty() { result.push('\n'); }
        result.push_str(&recent.join("\n"));
        result
    }

    pub(super) fn persist_truth_files(&self, story_dir: &Path, output: &writer::WriterOutput) -> Result<(), AppError> {
        // post_settlement 包含 settler 的 UPDATED_STATE + UPDATED_HOOKS 等
        // 简化版：直接写入 post_settlement 到 current_state.md
        if !output.post_settlement.is_empty() {
            // 解析 post_settlement 中的区块
            let state = extract_section(&output.post_settlement, "UPDATED_STATE");
            let hooks = extract_section(&output.post_settlement, "UPDATED_HOOKS");
            if let Some(s) = state { std::fs::write(story_dir.join("current_state.md"), s)?; }
            if let Some(h) = hooks { std::fs::write(story_dir.join("pending_hooks.md"), h)?; }
        }
        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)] // src 用于 exists()/read_dir()，非仅递归用途
    pub(super) fn copy_dir_recursive(&self, src: &Path, dest: &Path) -> Result<(), AppError> {
        if !src.exists() { return Ok(()); }
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                self.copy_dir_recursive(&src_path, &dest_path)?;
            } else if file_type.is_file() {
                if let Ok(content) = std::fs::read_to_string(&src_path) {
                    std::fs::write(&dest_path, content)?;
                }
            }
        }
        Ok(())
    }
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::{PipelineConfig, PipelineRunner};
    use crate::domain::pipeline::utils::text_parse::extract_section;

    #[test]
    fn extract_section_finds_tag() {
        let content = "前文\n=== UPDATED_STATE ===\n这是状态内容\n=== UPDATED_HOOKS ===\n这是伏笔内容";
        let state = extract_section(content, "UPDATED_STATE").expect("应找到 UPDATED_STATE");
        assert_eq!(state, "这是状态内容");

        let hooks = extract_section(content, "UPDATED_HOOKS").expect("应找到 UPDATED_HOOKS");
        assert_eq!(hooks, "这是伏笔内容");
    }

    #[test]
    fn extract_section_returns_none_when_missing() {
        let content = "无标签内容";
        assert!(extract_section(content, "UPDATED_STATE").is_none());
    }

    #[test]
    fn extract_section_returns_content_until_end() {
        // 最后一个区块：提取到文本结尾
        let content = "=== UPDATED_STATE ===\n最后一行内容\n没有后续标签";
        let state = extract_section(content, "UPDATED_STATE").expect("应找到");
        assert_eq!(state, "最后一行内容\n没有后续标签");
    }

    #[test]
    fn trim_recent_summaries_keeps_last_n() {
        // 构造 15 行数据 + 2 行表头（表头行 + 分隔行）
        let runner = PipelineRunner::new(PipelineConfig::default());
        let mut lines: Vec<String> = Vec::new();
        lines.push("| 章节 | 标题 | 摘要 |".to_string());
        lines.push("| --- | --- | --- |".to_string());
        for i in 1..=15 {
            lines.push(format!("| {} | 标题{} | 摘要{} |", i, i, i));
        }
        let content = lines.join("\n");
        let trimmed = runner.trim_recent_summaries(&content, 10);
        let trimmed_lines: Vec<&str> = trimmed.lines().collect();
        // 2 行表头 + 10 行数据
        assert_eq!(trimmed_lines.len(), 12);
        // 数据行应是第 6..=15 行（最近 10 条）
        assert!(trimmed_lines[2].contains("标题6"));
        assert!(trimmed_lines[11].contains("标题15"));
    }

    #[test]
    fn trim_recent_summaries_returns_empty_when_no_data() {
        let runner = PipelineRunner::new(PipelineConfig::default());
        let content = "| 章节 | 标题 |\n| --- | --- |";
        let trimmed = runner.trim_recent_summaries(content, 10);
        assert!(trimmed.is_empty());
    }
}
