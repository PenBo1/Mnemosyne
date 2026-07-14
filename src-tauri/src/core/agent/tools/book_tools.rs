// 小说创作工具集：8 个工具，覆盖确认闸门、pipeline 委托、真相文件编辑、
// 实体重命名、章节局部/整章编辑、章节导入、封面生成。
//
// 工具清单：
// - ProposeActionTool:       提议动作（确认闸门，纯 JSON，无副作用）
// - PipelineDelegateTool:    委托 pipeline agent 执行（plan/write/audit/revise/consolidate）
// - WriteTruthFileTool:      编辑真相文件（白名单内）
// - RenameEntityTool:        全书实体改名
// - PatchChapterTextTool:    章节局部编辑（三级文本匹配）
// - ReplaceChapterTextTool:  章节整章替换
// - ImportChaptersTool:      从源文件批量导入章节
// - GenerateCoverTool:       落盘封面提示词（图片生成未实现）
//
// 注：本文件位于 core/agent/tools，但需调用 domain 层函数（edit_controller /
// pipeline runner / consolidator），属于任务要求的显式指令。与 AGENTS.md
// "core/agent 不依赖 domain" 规则存在张力，此处遵循任务指令实现。
//
// 安全约束：
// - 所有路径以 DataDir.books_dir() 为根锚定，拒绝 `..` 与绝对路径
// - book_id 走 validate_id 校验
// - truth 文件名走 assert_safe_truth_file_name 校验（由 edit_controller 执行）
// - 写操作均需 ApprovalManager 审批

use std::path::PathBuf;
use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::engine::AgentEngine;
use crate::domain::interaction::edit_controller::{self, EditRequest};
use crate::domain::pipeline::agents::consolidator;
use crate::domain::pipeline::agents::reviser::ReviseMode;
use crate::domain::pipeline::runner::{PipelineConfig, PipelineRunner};
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::validation::validate_id;
use crate::shared::error::AppError;

// ── 统一错误类型 ────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum BookToolError {
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("路径穿越被禁止")]
    PathTraversal,
    #[error("权限被拒绝")]
    Denied,
    #[error("非法输入: {0}")]
    InvalidInput(String),
    #[error("执行失败: {0}")]
    Failed(String),
}

impl From<AppError> for BookToolError {
    fn from(e: AppError) -> Self {
        BookToolError::Failed(e.to_string())
    }
}

impl From<std::io::Error> for BookToolError {
    fn from(e: std::io::Error) -> Self {
        BookToolError::Io(e.to_string())
    }
}

// ── ProposeActionTool（确认闸门）────────────────────────────

/// 提议动作工具：让 agent 显式声明意图，供前端/用户确认后再执行破坏性操作。
/// 纯 JSON 返回，无副作用。
pub struct ProposeActionTool;

#[derive(Deserialize)]
pub struct ProposeActionArgs {
    /// 动作类型（如 rename_entity / replace_chapter / write_truth_file）
    pub action: String,
    /// 动作载荷（任意 JSON）
    pub payload: serde_json::Value,
    /// 提议理由（为何要执行此操作）
    #[serde(default)]
    pub reason: String,
}

impl Tool for ProposeActionTool {
    const NAME: &'static str = "propose_action";

    type Error = BookToolError;
    type Args = ProposeActionArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "提议一个需要用户确认的操作。本身不执行任何副作用，仅声明意图与载荷，等待审批。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "动作类型，如 rename_entity / replace_chapter / write_truth_file"
                    },
                    "payload": {
                        "type": "object",
                        "description": "动作载荷（任意 JSON，与对应工具的参数一致）"
                    },
                    "reason": {
                        "type": "string",
                        "description": "提议理由"
                    }
                },
                "required": ["action", "payload"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(serde_json::json!({
            "action": args.action,
            "payload": args.payload,
            "reason": args.reason,
            "requiresConfirmation": true,
        }))
    }
}

// ── PipelineDelegateTool（pipeline 委托）────────────────────

/// 委托 pipeline agent 执行重操作。按 agent_type 分派到 PipelineRunner 方法。
///
/// 注意：与 core/agent/subagent/tools.rs 的 SubAgentTool（NAME="subagent"）不同，
/// 本工具面向书籍 pipeline（plan/write/audit/revise/consolidate），命名为
/// pipeline_delegate 以避免工具名冲突。
pub struct PipelineDelegateTool {
    pub engine: Arc<AgentEngine>,
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct PipelineDelegateArgs {
    /// 委托的 pipeline 阶段：
    /// plan_chapter / write_draft / audit_draft / revise_draft /
    /// write_next_chapter / consolidate
    pub agent_type: String,
    /// 书籍 ID
    pub book_id: String,
    /// 可选参数（wordCountOverride / chapterNumber / mode）
    #[serde(default)]
    pub params: serde_json::Value,
}

impl Tool for PipelineDelegateTool {
    const NAME: &'static str = "pipeline_delegate";

    type Error = BookToolError;
    type Args = PipelineDelegateArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "委托书籍 pipeline 执行一个阶段（plan_chapter/write_draft/audit_draft/revise_draft/write_next_chapter/consolidate）。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_type": {
                        "type": "string",
                        "enum": [
                            "plan_chapter",
                            "write_draft",
                            "audit_draft",
                            "revise_draft",
                            "write_next_chapter",
                            "consolidate"
                        ],
                        "description": "pipeline 阶段"
                    },
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "params": {
                        "type": "object",
                        "description": "可选参数：wordCountOverride(int) / chapterNumber(int) / mode(revise 模式: auto/polish/rewrite/rework/antidetect/spotfix)"
                    }
                },
                "required": ["agent_type", "book_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_id(&args.book_id, "book_id")
            .map_err(|e| BookToolError::InvalidInput(e))?;

        // 统一审批：pipeline 阶段均为重操作
        let approved = self.approval.request_approval(
            "pipeline_delegate",
            &serde_json::json!({ "agent_type": args.agent_type, "book_id": args.book_id, "params": args.params }),
        ).await;
        if !approved {
            return Err(BookToolError::Denied);
        }

        let config = PipelineConfig {
            books_dir: self.data_dir.books_dir(),
            ..Default::default()
        };
        let runner = PipelineRunner::new(config);
        let engine = self.engine.clone();

        match args.agent_type.as_str() {
            "plan_chapter" => {
                let r = runner.plan_chapter(&engine, &args.book_id).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            "write_draft" => {
                let wc = parse_word_count_override(&args.params)?;
                let r = runner.write_draft(&engine, &args.book_id, wc).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            "audit_draft" => {
                let cn = parse_chapter_number(&args.params)?;
                let r = runner.audit_draft(&engine, &args.book_id, cn).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            "revise_draft" => {
                let cn = parse_chapter_number(&args.params)?;
                let mode = parse_revise_mode(&args.params)?;
                let r = runner.revise_draft(&engine, &args.book_id, cn, mode).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            "write_next_chapter" => {
                let wc = parse_word_count_override(&args.params)?;
                let r = runner.write_next_chapter(&engine, &args.book_id, wc).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            "consolidate" => {
                let book_dir = self.data_dir.books_dir().join(&args.book_id);
                if !book_dir.exists() {
                    return Err(BookToolError::InvalidInput(format!(
                        "书籍目录不存在: {}", args.book_id
                    )));
                }
                let r = consolidator::consolidate(&engine, &book_dir).await?;
                serde_json::to_value(&r).map_err(|e| BookToolError::Failed(e.to_string()))
            }
            other => Err(BookToolError::InvalidInput(format!(
                "未知 agent_type: {}", other
            ))),
        }
    }
}

// ── WriteTruthFileTool ─────────────────────────────────────

/// 编辑真相文件（白名单内）。委托 edit_controller::TruthFileEdit。
pub struct WriteTruthFileTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct WriteTruthFileArgs {
    pub book_id: String,
    /// 真相文件名（必须在白名单内：author_intent.md / current_focus.md /
    /// story_bible.md / volume_outline.md / book_rules.md / current_state.md /
    /// pending_hooks.md / chapter_summaries.md）
    pub file_name: String,
    /// 新内容（整体覆盖）
    pub new_content: String,
}

impl Tool for WriteTruthFileTool {
    const NAME: &'static str = "write_truth_file";

    type Error = BookToolError;
    type Args = WriteTruthFileArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "编辑书籍的真相文件（白名单内），整体覆盖。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "file_name": {
                        "type": "string",
                        "description": "真相文件名（白名单内）"
                    },
                    "new_content": { "type": "string", "description": "新内容（整体覆盖）" }
                },
                "required": ["book_id", "file_name", "new_content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        execute_edit_with_approval(
            &self.approval,
            "write_truth_file",
            &serde_json::json!({ "book_id": args.book_id, "file_name": args.file_name }),
            EditRequest::TruthFileEdit {
                book_id: args.book_id,
                file_name: args.file_name,
                new_content: args.new_content,
            },
            &self.data_dir,
        ).await
    }
}

// ── RenameEntityTool ───────────────────────────────────────

/// 全书实体改名。委托 edit_controller::EntityRename。
pub struct RenameEntityTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct RenameEntityArgs {
    pub book_id: String,
    pub old_name: String,
    pub new_name: String,
}

impl Tool for RenameEntityTool {
    const NAME: &'static str = "rename_entity";

    type Error = BookToolError;
    type Args = RenameEntityArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "在全书范围内将旧实体名替换为新名（含文件内容与文件改名）。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "old_name": { "type": "string", "description": "旧实体名" },
                    "new_name": { "type": "string", "description": "新实体名" }
                },
                "required": ["book_id", "old_name", "new_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        execute_edit_with_approval(
            &self.approval,
            "rename_entity",
            &serde_json::json!({ "book_id": args.book_id, "old_name": args.old_name, "new_name": args.new_name }),
            EditRequest::EntityRename {
                book_id: args.book_id,
                old_name: args.old_name,
                new_name: args.new_name,
            },
            &self.data_dir,
        ).await
    }
}

// ── PatchChapterTextTool ───────────────────────────────────

/// 章节局部编辑（三级文本匹配：精确 → 弹性空格 → 段落近似）。
/// 委托 edit_controller::ChapterLocalEdit。
pub struct PatchChapterTextTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct PatchChapterTextArgs {
    pub book_id: String,
    pub chapter_number: u32,
    /// 待查找的目标文本
    pub find: String,
    /// 替换为的新文本
    pub replace: String,
}

impl Tool for PatchChapterTextTool {
    const NAME: &'static str = "patch_chapter_text";

    type Error = BookToolError;
    type Args = PatchChapterTextArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "在指定章节内查找目标文本并替换（三级匹配：精确 → 弹性空格 → 段落近似）。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "chapter_number": { "type": "integer", "description": "章节号" },
                    "find": { "type": "string", "description": "待查找的目标文本" },
                    "replace": { "type": "string", "description": "替换为的新文本" }
                },
                "required": ["book_id", "chapter_number", "find", "replace"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        execute_edit_with_approval(
            &self.approval,
            "patch_chapter_text",
            &serde_json::json!({ "book_id": args.book_id, "chapter_number": args.chapter_number }),
            EditRequest::ChapterLocalEdit {
                book_id: args.book_id,
                chapter_number: args.chapter_number,
                find: args.find,
                replace: args.replace,
            },
            &self.data_dir,
        ).await
    }
}

// ── ReplaceChapterTextTool ─────────────────────────────────

/// 章节整章替换。委托 edit_controller::ChapterReplace。
pub struct ReplaceChapterTextTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct ReplaceChapterTextArgs {
    pub book_id: String,
    pub chapter_number: u32,
    pub new_content: String,
}

impl Tool for ReplaceChapterTextTool {
    const NAME: &'static str = "replace_chapter_text";

    type Error = BookToolError;
    type Args = ReplaceChapterTextArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "用新内容整体覆盖指定章节正文。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "chapter_number": { "type": "integer", "description": "章节号" },
                    "new_content": { "type": "string", "description": "新章节正文（整体覆盖）" }
                },
                "required": ["book_id", "chapter_number", "new_content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        execute_edit_with_approval(
            &self.approval,
            "replace_chapter_text",
            &serde_json::json!({ "book_id": args.book_id, "chapter_number": args.chapter_number }),
            EditRequest::ChapterReplace {
                book_id: args.book_id,
                chapter_number: args.chapter_number,
                new_content: args.new_content,
            },
            &self.data_dir,
        ).await
    }
}

// ── ImportChaptersTool ─────────────────────────────────────

/// 从源文件批量导入章节。源文件须位于书籍目录下，按章节标题切分后
/// 写入 chapters/XXXX_<slug>.md。不更新 chapters.json 索引（agent 可后续
/// 调用 consolidate 重建）。
pub struct ImportChaptersTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct ImportChaptersArgs {
    pub book_id: String,
    /// 相对 book_dir 的源文件路径
    pub source_path: String,
    /// 可选切分正则（缺省按 markdown 标题 `^#{1,6}\\s` 或中文 `^第.+章` 切分）
    #[serde(default)]
    pub split_pattern: Option<String>,
}

impl Tool for ImportChaptersTool {
    const NAME: &'static str = "import_chapters";

    type Error = BookToolError;
    type Args = ImportChaptersArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "从源文件批量导入章节到书籍。源文件须位于书籍目录下，按章节标题切分写入 chapters/。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "source_path": {
                        "type": "string",
                        "description": "相对 book_dir 的源文件路径"
                    },
                    "split_pattern": {
                        "type": "string",
                        "description": "可选切分正则（缺省按 markdown 标题或中文「第X章」切分）"
                    }
                },
                "required": ["book_id", "source_path"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_id(&args.book_id, "book_id")
            .map_err(BookToolError::InvalidInput)?;

        let book_dir = self.data_dir.books_dir().join(&args.book_id);
        if !book_dir.exists() {
            return Err(BookToolError::InvalidInput(format!(
                "书籍目录不存在: {}", args.book_id
            )));
        }

        // 源文件路径须锁定在 book_dir 之下
        let source = resolve_under_book(&book_dir, &args.source_path)?;
        if !source.is_file() {
            return Err(BookToolError::InvalidInput(format!(
                "源文件不存在: {}", args.source_path
            )));
        }

        let approved = self.approval.request_approval(
            "import_chapters",
            &serde_json::json!({ "book_id": args.book_id, "source_path": args.source_path }),
        ).await;
        if !approved {
            return Err(BookToolError::Denied);
        }

        let content = std::fs::read_to_string(&source)?;
        let chunks = split_into_chapters(&content, args.split_pattern.as_deref())?;
        if chunks.is_empty() {
            return Err(BookToolError::InvalidInput("切分后未得到任何章节".to_string()));
        }

        let chapters_dir = book_dir.join("chapters");
        std::fs::create_dir_all(&chapters_dir)?;

        let start_no = next_chapter_number(&chapters_dir)? + 1;
        let mut written: Vec<serde_json::Value> = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let num = start_no + i as u32;
            let title = extract_title(chunk);
            let slug = sanitize_slug(&title);
            let filename = format!("{:04}_{}.md", num, slug);
            let path = chapters_dir.join(&filename);
            std::fs::write(&path, ensure_trailing_newline(chunk))?;
            written.push(serde_json::json!({
                "chapter_number": num,
                "title": title,
                "file": filename,
            }));
        }

        Ok(serde_json::json!({
            "imported_count": written.len(),
            "chapters": written,
        }))
    }
}

// ── GenerateCoverTool ──────────────────────────────────────

/// 封面生成工具。图片生成能力未实现，仅将封面提示词落盘到
/// story/cover-prompt.md（与 short_fiction_runner 一致）。
pub struct GenerateCoverTool {
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
}

#[derive(Deserialize)]
pub struct GenerateCoverArgs {
    pub book_id: String,
    /// 封面概念描述（风格 / 主体 / 构图 / 色调）
    pub description: String,
}

impl Tool for GenerateCoverTool {
    const NAME: &'static str = "generate_cover";

    type Error = BookToolError;
    type Args = GenerateCoverArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "为书籍生成封面提示词并落盘（图片生成未实现，仅写 cover-prompt.md）。需用户确认。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "book_id": { "type": "string", "description": "书籍 ID" },
                    "description": {
                        "type": "string",
                        "description": "封面概念描述（风格/主体/构图/色调）"
                    }
                },
                "required": ["book_id", "description"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_id(&args.book_id, "book_id")
            .map_err(BookToolError::InvalidInput)?;

        let approved = self.approval.request_approval(
            "generate_cover",
            &serde_json::json!({ "book_id": args.book_id }),
        ).await;
        if !approved {
            return Err(BookToolError::Denied);
        }

        let book_dir = self.data_dir.books_dir().join(&args.book_id);
        if !book_dir.exists() {
            return Err(BookToolError::InvalidInput(format!(
                "书籍目录不存在: {}", args.book_id
            )));
        }
        let story_dir = book_dir.join("story");
        std::fs::create_dir_all(&story_dir)?;
        let cover_path = story_dir.join("cover-prompt.md");
        std::fs::write(&cover_path, ensure_trailing_newline(&args.description))?;

        Ok(serde_json::json!({
            "saved": true,
            "path": "story/cover-prompt.md",
            "note": "图片生成未实现，仅落盘封面提示词"
        }))
    }
}

// ── 共享：审批 + edit_controller 执行 ──────────────────────

/// 通用流程：审批 → plan_edit_transaction → execute_edit_transaction → 序列化返回。
async fn execute_edit_with_approval(
    approval: &Arc<ApprovalManager>,
    tool_name: &str,
    payload: &serde_json::Value,
    request: EditRequest,
    data_dir: &DataDir,
) -> Result<serde_json::Value, BookToolError> {
    let approved = approval.request_approval(tool_name, payload).await;
    if !approved {
        return Err(BookToolError::Denied);
    }
    let planned = edit_controller::plan_edit_transaction(request)?;
    let executed = edit_controller::execute_edit_transaction(planned, data_dir)?;
    serde_json::to_value(&executed).map_err(|e| BookToolError::Failed(e.to_string()))
}

// ── 辅助：参数解析 ─────────────────────────────────────────

/// 从 params 中解析 wordCountOverride（正整数）
fn parse_word_count_override(params: &serde_json::Value) -> Result<Option<u32>, BookToolError> {
    match params.get("wordCountOverride").and_then(|v| v.as_u64()) {
        Some(n) if n > 0 => Ok(Some(n as u32)),
        Some(_) => Err(BookToolError::InvalidInput("wordCountOverride 须为正整数".to_string())),
        None => Ok(None),
    }
}

/// 从 params 中解析 chapterNumber（正整数）
fn parse_chapter_number(params: &serde_json::Value) -> Result<Option<u32>, BookToolError> {
    match params.get("chapterNumber").and_then(|v| v.as_u64()) {
        Some(n) if n > 0 => Ok(Some(n as u32)),
        Some(_) => Err(BookToolError::InvalidInput("chapterNumber 须为正整数".to_string())),
        None => Ok(None),
    }
}

/// 从 params 中解析 revise mode（缺省 Auto）
fn parse_revise_mode(params: &serde_json::Value) -> Result<ReviseMode, BookToolError> {
    let s = params.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
    match s.to_lowercase().as_str() {
        "auto" => Ok(ReviseMode::Auto),
        "polish" => Ok(ReviseMode::Polish),
        "rewrite" => Ok(ReviseMode::Rewrite),
        "rework" => Ok(ReviseMode::Rework),
        "antidetect" => Ok(ReviseMode::AntiDetect),
        "spotfix" => Ok(ReviseMode::SpotFix),
        other => Err(BookToolError::InvalidInput(format!("未知 revise mode: {}", other))),
    }
}

// ── 辅助：路径与章节切分 ───────────────────────────────────

/// 将相对路径解析到 book_dir 之下，拒绝 `..` 与绝对路径。
fn resolve_under_book(book_dir: &PathBuf, rel: &str) -> Result<PathBuf, BookToolError> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Err(BookToolError::InvalidInput("路径不能为空".to_string()));
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains("..") {
        return Err(BookToolError::PathTraversal);
    }
    let resolved = book_dir.join(trimmed);
    if !resolved.starts_with(book_dir) {
        return Err(BookToolError::PathTraversal);
    }
    Ok(resolved)
}

/// 将正文按章节标题切分。默认匹配 markdown 标题 `^#{1,6}\s` 或中文 `^第.+章`。
/// 返回的每个块包含标题行本身。
fn split_into_chapters(content: &str, pattern: Option<&str>) -> Result<Vec<String>, BookToolError> {
    let re = if let Some(p) = pattern {
        regex::Regex::new(p).map_err(|e| BookToolError::InvalidInput(format!("非法正则: {}", e)))?
    } else {
        // 默认：markdown 标题或中文「第X章」标题行
        regex::Regex::new(r"^(#{1,6}\s.+|第[^\n]*章[^\n]*)$")
            .map_err(|e| BookToolError::Failed(e.to_string()))?
    };

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        if re.is_match(line) && !current.trim().is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current);
    }
    Ok(chunks)
}

/// 从章节块提取标题（取首行，去掉 markdown 标记与「第X章」前缀）
fn extract_title(chunk: &str) -> String {
    let first = chunk.lines().next().unwrap_or("").trim();
    let stripped = first
        .trim_start_matches('#')
        .trim()
        .trim_start_matches("第")
        .trim();
    if stripped.is_empty() {
        "imported".to_string()
    } else {
        stripped.to_string()
    }
}

/// 将标题净化为文件名安全的 slug（保留中文/字母数字，其余替换为 -，截断 30 字符）
fn sanitize_slug(title: &str) -> String {
    let mut out: String = title.chars().take(30).filter_map(|c| {
        if c.is_alphanumeric() {
            Some(c)
        } else if c.is_whitespace() {
            Some('-')
        } else {
            None
        }
    }).collect();
    if out.is_empty() {
        out = "imported".to_string();
    }
    out
}

/// 扫描 chapters 目录，返回最大章节号
fn next_chapter_number(chapters_dir: &PathBuf) -> Result<u32, BookToolError> {
    let mut max: u32 = 0;
    if !chapters_dir.exists() {
        return Ok(0);
    }
    for entry in std::fs::read_dir(chapters_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(num_str) = name.get(..4) {
            if let Ok(n) = num_str.parse::<u32>() {
                if n > max {
                    max = n;
                }
            }
        }
    }
    Ok(max)
}

/// 确保字符串以单个换行结尾
fn ensure_trailing_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{}\n", s)
    }
}
