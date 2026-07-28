//! ═══════════════════════════════════════════════════════════════════════════
//! 管道操作 - Pipeline 操作抽象 trait
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use async_trait::async_trait;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

/// 书籍列表条目（interaction 层最小 DTO，不依赖 domain::pipeline::commands::BookSummary）。
#[derive(Debug, Clone)]
pub struct BookListing {
    pub id: String,
    pub title: String,
}

/// Pipeline 操作 trait。抽象 runtime.rs 对 domain::pipeline 的 3 处依赖：
/// - PipelineRunner::write_next_chapter
/// - PipelineRunner::revise_draft
/// - list_books_inner（扫描 books_dir 构造 BookSummary）
#[async_trait]
pub trait InteractionPipelineOps: Send + Sync {
    /// 写下一章，返回新章节号。
    async fn write_next_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
    ) -> Result<u32, AppError>;

    /// 修订/重写章节。rewrite=true 表示整章重写，false 表示自动修订。
    async fn revise_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        chapter: u32,
        rewrite: bool,
    ) -> Result<(), AppError>;

    /// 列出 books_dir 下所有书籍（仅 id + title）。
    fn list_books(&self) -> Result<Vec<BookListing>, AppError>;
}

/// Tauri State 包装：持有 InteractionPipelineOps 的真实实现（由 application/bridges.rs 注入）。
pub struct InteractionPipelineOpsState {
    pub ops: Arc<dyn InteractionPipelineOps>,
}
