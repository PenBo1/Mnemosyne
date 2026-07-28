//! ═══════════════════════════════════════════════════════════════════════════
//! BookOps - Agent 工具操作 trait
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;

use async_trait::async_trait;

use super::super::engine::AgentEngine;

// ── BookEditOps：编辑事务操作 ───────────────────────────────

/// 编辑事务操作 trait。对应 domain::interaction::edit_controller 的 4 种 EditRequest
/// （TruthFileEdit / EntityRename / ChapterLocalEdit / ChapterReplace）。
///
/// 实现方负责 plan_edit_transaction + execute_edit_transaction 闭环，
/// 返回 ExecutedEditTransaction 的序列化 JSON。
#[async_trait]
pub trait BookEditOps: Send + Sync {
    /// 真相文件编辑（白名单内 file_name，整体覆盖为 new_content）
    async fn truth_file_edit(
        &self,
        book_id: String,
        file_name: String,
        new_content: String,
    ) -> Result<serde_json::Value, String>;

    /// 全书实体改名（old_name → new_name，含文件改名）
    async fn entity_rename(
        &self,
        book_id: String,
        old_name: String,
        new_name: String,
    ) -> Result<serde_json::Value, String>;

    /// 章节局部编辑（三级匹配：精确 → 弹性空格 → 段落近似）
    async fn chapter_local_edit(
        &self,
        book_id: String,
        chapter_number: u32,
        find: String,
        replace: String,
    ) -> Result<serde_json::Value, String>;

    /// 章节整章替换（new_content 覆盖原内容）
    async fn chapter_replace(
        &self,
        book_id: String,
        chapter_number: u32,
        new_content: String,
    ) -> Result<serde_json::Value, String>;
}

// ── PipelineDelegateOps：Pipeline 阶段委托 ─────────────────

/// Pipeline 委托操作 trait。对应 PipelineRunner 的 5 个阶段方法 + consolidator::consolidate。
///
/// 实现方负责构造 PipelineRunner / 调用 consolidator，返回各阶段结果的序列化 JSON。
/// mode 参数为 ReviseMode 的字符串形式（auto/polish/rewrite/rework/anti-detect/spot-fix）。
#[async_trait]
pub trait PipelineDelegateOps: Send + Sync {
    /// 规划章节（planner agent）
    async fn plan_chapter(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
    ) -> Result<serde_json::Value, String>;

    /// 写作草稿（writer agent）
    async fn write_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String>;

    /// 审计草稿（continuity agent）
    async fn audit_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        chapter_number: Option<u32>,
    ) -> Result<serde_json::Value, String>;

    /// 修订草稿（reviser agent）
    async fn revise_draft(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        chapter_number: Option<u32>,
        mode: &str,
    ) -> Result<serde_json::Value, String>;

    /// 写下一章（plan + write + audit + consolidate 闭环）
    async fn write_next_chapter(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
        word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String>;

    /// 章节合并（consolidator agent：facts 提取 + truth file 更新）
    async fn consolidate(
        &self,
        engine: &Arc<AgentEngine>,
        book_id: &str,
    ) -> Result<serde_json::Value, String>;
}

// ── ResearchOps：研究 + 材料操作 ───────────────────────────

/// 研究 + 材料操作 trait。对应 domain::researcher::report::run_research_report +
/// domain::materials::ingest::ingest_material + domain::materials::retrieve::retrieve_materials。
///
/// 实现方负责调用 domain 函数并返回序列化 JSON。
#[async_trait]
pub trait ResearchOps: Send + Sync {
    /// 研究报告（LLM 驱动，需 engine）
    async fn run_research_report(
        &self,
        engine: &Arc<AgentEngine>,
        query: String,
        depth: Option<String>,
    ) -> Result<serde_json::Value, String>;

    /// 材料导入（URL 或本地文件 → 落盘 + 资产清单）
    async fn ingest_material(
        &self,
        source_kind: String,
        url: Option<String>,
        file_path: Option<String>,
        filename: Option<String>,
        mime_type: Option<String>,
        title: Option<String>,
        purpose: Option<String>,
    ) -> Result<serde_json::Value, String>;

    /// 材料检索（按关键词匹配已导入材料）
    async fn retrieve_materials(
        &self,
        query: String,
        purpose: Option<String>,
        limit: Option<u32>,
    ) -> Result<serde_json::Value, String>;
}

// ── Stub 实现（默认占位，生产环境由 application/bridges.rs 注入真实实现） ──
//
// 设计动机：AgentEngine::new 默认装填 stub，避免 Option<Arc<dyn>> 散落各处。
// 生产路径在 lib.rs 构造时通过 with_tool_ops 替换为真实实现；若忘记注入，
// 工具调用会显式报错（而非静默失败），符合 "No silent fallbacks" 原则。

const STUB_ERR: &str = "tool ops 未注入：请通过 AgentEngine::with_tool_ops 装填真实实现";

pub struct StubBookEditOps;

#[async_trait]
impl BookEditOps for StubBookEditOps {
    async fn truth_file_edit(
        &self,
        _book_id: String,
        _file_name: String,
        _new_content: String,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn entity_rename(
        &self,
        _book_id: String,
        _old_name: String,
        _new_name: String,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn chapter_local_edit(
        &self,
        _book_id: String,
        _chapter_number: u32,
        _find: String,
        _replace: String,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn chapter_replace(
        &self,
        _book_id: String,
        _chapter_number: u32,
        _new_content: String,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }
}

pub struct StubPipelineDelegateOps;

#[async_trait]
impl PipelineDelegateOps for StubPipelineDelegateOps {
    async fn plan_chapter(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn write_draft(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
        _word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn audit_draft(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
        _chapter_number: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn revise_draft(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
        _chapter_number: Option<u32>,
        _mode: &str,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn write_next_chapter(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
        _word_count_override: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn consolidate(
        &self,
        _engine: &Arc<AgentEngine>,
        _book_id: &str,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }
}

pub struct StubResearchOps;

#[async_trait]
impl ResearchOps for StubResearchOps {
    async fn run_research_report(
        &self,
        _engine: &Arc<AgentEngine>,
        _query: String,
        _depth: Option<String>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn ingest_material(
        &self,
        _source_kind: String,
        _url: Option<String>,
        _file_path: Option<String>,
        _filename: Option<String>,
        _mime_type: Option<String>,
        _title: Option<String>,
        _purpose: Option<String>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }

    async fn retrieve_materials(
        &self,
        _query: String,
        _purpose: Option<String>,
        _limit: Option<u32>,
    ) -> Result<serde_json::Value, String> {
        Err(STUB_ERR.to_string())
    }
}
