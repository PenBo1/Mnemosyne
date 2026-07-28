//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Runner - Pipeline 编排器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 8-agent 编排核心：initBook / planChapter / composeChapter / writeDraft / auditDraft /
//! reviseDraft / writeNextChapter / reviseFoundation。
//!
//! Rust 版拆为 PipelineRunner struct + 关联函数。
//! 文件系统操作用 std::fs 同步 API（与 state/agents 模块一致），LLM 调用走 AgentEngine。
//!
//! 子模块：
//! - types: PipelineConfig + 各阶段结果类型
//! - foundation: init_book / generate_and_review_foundation / revise_foundation
//! - plan_write: plan_chapter / compose_chapter / write_draft
//! - audit_revise: audit_draft / revise_draft
//! - write_next: write_next_chapter（完整 8-agent cycle）
//! - helpers: 日志 / 章节索引 / 快照 / 真相文件落盘 等内部辅助方法

// ── 模块声明 ────────────────────────────────────────────────────────────────

mod audit_revise;
mod foundation;
mod helpers;
mod plan_write;
mod types;
mod write_next;

pub use types::{
    ChapterPipelineResult, ComposeChapterResult, DraftResult, PipelineConfig,
    PlanChapterResult, ReviseResult,
};

use std::path::PathBuf;

// ── PipelineRunner ───────────────────────────────────────────

/// Pipeline 编排器
pub struct PipelineRunner {
    config: PipelineConfig,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    fn book_dir(&self, book_id: &str) -> PathBuf {
        self.config.books_dir.join(book_id)
    }
}

// ── 工具函数（模块级）──

/// 当前 ISO 时间戳
fn current_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

/// 文件名安全化：将特殊字符替换为下划线
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_replaces_special_chars() {
        // "a/b:c" → 'a','_','b','_','c' = "a_b_c"
        let result = sanitize_filename("a/b:c");
        assert_eq!(result, "a_b_c");
    }

    #[test]
    fn sanitize_filename_replaces_all_special_chars() {
        // 覆盖所有特殊字符
        let result = sanitize_filename(r#"a/b\c:d*e?f"g<h>i|j"#);
        assert_eq!(result, "a_b_c_d_e_f_g_h_i_j");
    }

    #[test]
    fn sanitize_filename_preserves_normal_chars() {
        let result = sanitize_filename("第1章 暗流");
        assert_eq!(result, "第1章 暗流");
    }

    #[test]
    fn current_iso_returns_valid_iso() {
        let iso = current_iso();
        // RFC3339 格式包含 'T' 分隔符和时区偏移（+08:00 或 Z）
        assert!(iso.contains('T'), "ISO 时间戳应包含 T 分隔符: {}", iso);
        // 时区偏移：以 + 或 Z 结尾特征
        assert!(
            iso.contains('+') || iso.ends_with('Z'),
            "ISO 时间戳应包含时区信息: {}", iso
        );
    }
}
