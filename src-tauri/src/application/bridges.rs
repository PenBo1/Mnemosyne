//! ═══════════════════════════════════════════════════════════════════════════
//! Bridges - 应用层桥接模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供 core/agent 工具 trait 的真实实现，将 domain 调用抽象为 trait 方法。
//! 本文件位于 application 层（允许依赖 domain），提供 trait 的具体实现。
//!
//! 桥接职责：
//! - BookEditOpsImpl：包装 edit_controller（plan + execute）
//! - PipelineDelegateOpsImpl：包装 PipelineRunner + consolidator
//! - ResearchOpsImpl：包装 researcher::report + materials::{ingest, retrieve}

use std::sync::Arc;

use async_trait::async_trait;

use crate::core::agent::engine::AgentEngine;
use crate::core::agent::tools::book_ops::{BookEditOps, PipelineDelegateOps, ResearchOps};
use crate::domain::interaction::edit_controller::{self, EditRequest};
use crate::domain::interaction::pipeline_ops::{BookListing, InteractionPipelineOps};
use crate::domain::materials::ingest::ingest_material;
use crate::domain::materials::retrieve::retrieve_materials;
use crate::domain::materials::types::{IngestMaterialInput, RetrieveMaterialsInput};
use crate::domain::pipeline::agents::consolidator;
use crate::domain::pipeline::agents::reviser::ReviseMode;
use crate::domain::pipeline::radar_ops::{RadarScanOps, RadarScanOutcome};
use crate::domain::pipeline::runner::{PipelineConfig, PipelineRunner};
use crate::domain::researcher::report::run_research_report;
use crate::domain::researcher::types::ResearchInput;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

// ── BookEditOpsImpl 实现 ────────────────────────────────────────────────────────

/// BookEditOps 的真实实现：包装 edit_controller 的 plan + execute 闭环。
pub struct BookEditOpsImpl {
    data_dir: DataDir,
}

impl BookEditOpsImpl {
    pub fn new(data_dir: DataDir) -> Self {
        Self { data_dir }
    }

    /// 通用流程：plan → execute → 序列化返回。
    async fn execute(&self, request: EditRequest) -> Result<serde_json::Value, String> {
        let planned = edit_controller::plan_edit_transaction(request)
            .map_err(|e| format!("plan_edit_transaction 失败: {}", e))?;
        let executed = edit_controller::execute_edit_transaction(planned, &self.data_dir)
            .map_err(|e| format!("execute_edit_transaction 失败: {}", e))?;
        serde_json::to_value(&executed).map_err(|e| format!("序列化失败: {}", e))
    }
}

#[async_trait]
impl BookEditOps for BookEditOpsImpl {
    async fn truth_file_edit(
        &self,
        book_id: String,
        file_name: String,
        new_content: String,
    ) -> Result<serde_json::Value, String> {
        self.execute(EditRequest::TruthFileEdit {
            book_id,
            file_name,
            new_content,
        })
        .await
    }

    async fn entity_rename(
        &self,
        book_id: String,
        old_name: String,
        new_name: String,
    ) -> Result<serde_json::Value, String> {
        self.execute(EditRequest::EntityRename {
            book_id,
            old_name,
            new_name,
        })
        .await
    }

    async fn chapter_local_edit(
        &self,
        book_id: String,
        chapter_number: u32,
        find: String,
        replace: String,
    ) -> Result<serde_json::Value, String> {
        self.execute(EditRequest::ChapterLocalEdit {
            book_id,
            chapter_number,
            find,
            replace,
        })
        .await
    }

    async fn chapter_replace(
        &self,
        book_id: String,
        chapter_number: u32,
        new_content: String,
    ) -> Result<serde_json::Value, String> {
        self.execute(EditRequest::ChapterReplace {
            book_id,
            chapter_number,
            new_content,
        })
        .await
    }
}

// ── PipelineDelegateOpsImpl 实现 ────────────────────────────────────────────────────────

/// PipelineDelegateOps 的真实实现：包装 PipelineRunner + consolidator。
pub struct PipelineDelegateOpsImpl {
    data_dir: DataDir,
}

impl PipelineDelegateOpsImpl {
    pub fn new(data_dir: DataDir) -> Self {
        Self { data_dir }
    }

    /// 构造 PipelineRunner（books_dir 锚定 DataDir）。
    fn runner(&self) -> PipelineRunner {
        let config = PipelineConfig {
            books_dir: self.data_dir.books_dir(),
            ..Default::default()
        };
        PipelineRunner::new(config)
    }
}

#[async_trait]
impl PipelineDelegateOps for PipelineDelegateOpsImpl {
    async fn plan_chapter(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
    ) -> Result<serde_json::Value, String> {
        let runner = self.runner();
        let r = runner
            .plan_chapter(engine, book_id)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn write_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let runner = self.runner();
        let r = runner
            .write_draft(engine, book_id, word_count_override)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn audit_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        chapter_number: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let runner = self.runner();
        let r = runner
            .audit_draft(engine, book_id, chapter_number)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn revise_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        chapter_number: Option<u32>,
        mode: &str,
    ) -> Result<serde_json::Value, String> {
        let mode = parse_revise_mode(mode)?;
        let runner = self.runner();
        let r = runner
            .revise_draft(engine, book_id, chapter_number, mode)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn write_next_chapter(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let runner = self.runner();
        let r = runner
            .write_next_chapter(engine, book_id, word_count_override)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn consolidate(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
    ) -> Result<serde_json::Value, String> {
        let book_dir = self.data_dir.books_dir().join(book_id);
        if !book_dir.exists() {
            return Err(format!("书籍目录不存在: {}", book_id));
        }
        let r = consolidator::consolidate(engine, &book_dir)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&r).map_err(|e| format!("序列化失败: {}", e))
    }
}

/// 将 mode 字符串映射到 ReviseMode 枚举。
fn parse_revise_mode(s: &str) -> Result<ReviseMode, String> {
    match s.to_lowercase().as_str() {
        "auto" => Ok(ReviseMode::Auto),
        "polish" => Ok(ReviseMode::Polish),
        "rewrite" => Ok(ReviseMode::Rewrite),
        "rework" => Ok(ReviseMode::Rework),
        "antidetect" => Ok(ReviseMode::AntiDetect),
        "spotfix" => Ok(ReviseMode::SpotFix),
        other => Err(format!("未知 revise mode: {}", other)),
    }
}

// ── ResearchOpsImpl 实现 ────────────────────────────────────────────────────────

/// ResearchOps 的真实实现：包装 researcher::report + materials::{ingest, retrieve}。
pub struct ResearchOpsImpl {
    data_dir: DataDir,
}

impl ResearchOpsImpl {
    pub fn new(data_dir: DataDir) -> Self {
        Self { data_dir }
    }
}

#[async_trait]
impl ResearchOps for ResearchOpsImpl {
    async fn run_research_report(
        &self,
        engine: &Arc<AgentEngine>,
        query: String,
        depth: Option<String>,
    ) -> Result<serde_json::Value, String> {
        let input = ResearchInput { query, depth };
        let report = run_research_report(engine, &input)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&report).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn ingest_material(
        &self,
        source_kind: String,
        url: Option<String>,
        file_path: Option<String>,
        filename: Option<String>,
        mime_type: Option<String>,
        title: Option<String>,
        purpose: Option<String>,
    ) -> Result<serde_json::Value, String> {
        let input = IngestMaterialInput {
            source_kind,
            url,
            file_path,
            filename,
            mime_type,
            title,
            purpose,
        };
        let asset = ingest_material(&self.data_dir, &input)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::to_value(&asset).map_err(|e| format!("序列化失败: {}", e))
    }

    async fn retrieve_materials(
        &self,
        query: String,
        purpose: Option<String>,
        limit: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        let input = RetrieveMaterialsInput {
            query,
            purpose,
            limit,
        };
        let results = retrieve_materials(&self.data_dir, &input).map_err(|e| e.to_string())?;
        serde_json::to_value(&results).map_err(|e| format!("序列化失败: {}", e))
    }
}

// ── InteractionPipelineOpsImpl 实现 ────────────────────────────────────────────────────────

/// InteractionPipelineOps 的真实实现：包装 PipelineRunner + books_dir 扫描。
///
/// 桥接 domain::interaction::runtime 对 domain::pipeline 的 3 处依赖：
/// - write_next_chapter → PipelineRunner::write_next_chapter（返回 chapter_number）
/// - revise_draft → PipelineRunner::revise_draft（rewrite bool → ReviseMode）
/// - list_books → 扫描 books_dir 读取 book.json（返回 BookListing）
pub struct InteractionPipelineOpsImpl {
    data_dir: DataDir,
}

impl InteractionPipelineOpsImpl {
    pub fn new(data_dir: DataDir) -> Self {
        Self { data_dir }
    }

    /// 构造 PipelineRunner（books_dir 锚定 DataDir）。
    fn runner(&self) -> PipelineRunner {
        let config = PipelineConfig {
            books_dir: self.data_dir.books_dir(),
            ..Default::default()
        };
        PipelineRunner::new(config)
    }
}

#[async_trait]
impl InteractionPipelineOps for InteractionPipelineOpsImpl {
    async fn write_next_chapter(
        &self,
        engine: &AgentEngine,
        book_id: &str,
    ) -> Result<u32, AppError> {
        let runner = self.runner();
        let result = runner.write_next_chapter(engine, book_id, None).await?;
        Ok(result.chapter_number)
    }

    async fn revise_draft(
        &self,
        engine: &AgentEngine,
        book_id: &str,
        chapter: u32,
        rewrite: bool,
    ) -> Result<(), AppError> {
        let mode = if rewrite {
            ReviseMode::Rewrite
        } else {
            ReviseMode::Auto
        };
        let runner = self.runner();
        runner.revise_draft(engine, book_id, Some(chapter), mode).await?;
        Ok(())
    }

    fn list_books(&self) -> Result<Vec<BookListing>, AppError> {
        let books_dir = self.data_dir.books_dir();
        if !books_dir.exists() {
            return Ok(Vec::new());
        }

        let mut listings = Vec::new();
        for entry in std::fs::read_dir(&books_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let book_id = entry.file_name().to_string_lossy().to_string();
            if book_id.starts_with('.') || book_id.starts_with(".tmp") {
                continue;
            }

            let config_path = entry.path().join("book.json");
            if !config_path.exists() {
                continue;
            }

            let config_content = match std::fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read book.json, skipping book");
                    continue;
                }
            };
            let book: crate::domain::pipeline::types::BookConfig =
                match serde_json::from_str(&config_content) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to parse book.json, skipping book");
                        continue;
                    }
                };

            listings.push(BookListing {
                id: book.id,
                title: book.title,
            });
        }

        listings.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(listings)
    }
}

// ── RadarScanOpsImpl 实现 ────────────────────────────────────────────────────────

/// RadarScanOps 的真实实现：包装 domain::radar::agent::scan。
///
/// 桥接 domain::pipeline::scheduler 对 domain::radar::agent 的横向依赖：
/// scheduler 通过 RadarScanOps trait 调用，本实现在 application 层注入真实调用。
pub struct RadarScanOpsImpl;

impl Default for RadarScanOpsImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl RadarScanOpsImpl {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RadarScanOps for RadarScanOpsImpl {
    async fn scan(&self, engine: &AgentEngine) -> Result<RadarScanOutcome, AppError> {
        let outcome = crate::domain::radar::agent::scan(engine, None).await?;
        Ok(RadarScanOutcome {
            recommendations_count: outcome.result.recommendations.len(),
        })
    }
}
