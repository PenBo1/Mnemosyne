//! ═══════════════════════════════════════════════════════════════════════════
//! BookTools - 小说创作工具集
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::engine::AgentEngine;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::fs::fs_utils::MAX_READ_SIZE;
use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};
use crate::infrastructure::validation::validate_id;
use crate::shared::error::AppError;

use super::book_ops::{BookEditOps, PipelineDelegateOps};

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

// ── BookToolError → ToolError 转换 ─────────────────────────

impl From<BookToolError> for ToolError {
    fn from(e: BookToolError) -> Self {
        match e {
            BookToolError::Io(s) => ToolError::Io(s),
            BookToolError::PathTraversal => ToolError::PathTraversal,
            BookToolError::Denied => ToolError::Execution("Permission denied".to_string()),
            BookToolError::InvalidInput(s) => ToolError::InvalidArgs(s),
            BookToolError::Failed(s) => ToolError::Execution(s),
        }
    }
}

// AppError 直接转 ToolError（保留供未来 pipeline 操作的错误传播使用）
impl From<AppError> for ToolError {
    fn from(e: AppError) -> Self {
        ToolError::Execution(e.to_string())
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

#[async_trait]
impl Tool for ProposeActionTool {
    fn name(&self) -> &str {
        "propose_action"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "propose_action".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ProposeActionArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "propose_action", action = %args.action, "[tool] call");

        let result = serde_json::json!({
            "action": args.action,
            "payload": args.payload,
            "reason": args.reason,
            "requiresConfirmation": true,
        });

        tracing::info!(tool = "propose_action", action = %args.action, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(result)
    }
}

// ── PipelineDelegateTool（pipeline 委托）────────────────────

/// 委托 pipeline agent 执行重操作。按 agent_type 分派到 PipelineDelegateOps trait 方法。
///
/// 注意：与 core/agent/subagent/tools.rs 的 SubAgentTool（NAME="subagent"）不同，
/// 本工具面向书籍 pipeline（plan/write/audit/revise/consolidate），命名为
/// pipeline_delegate 以避免工具名冲突。
///
/// 实际 pipeline 调用由 `pipeline_ops` trait 提供方实现（application/bridges.rs），
/// 本工具仅负责参数校验、审批、分派，不直接依赖 domain 层。
pub struct PipelineDelegateTool {
    pub engine: Arc<AgentEngine>,
    pub data_dir: DataDir,
    pub approval: Arc<ApprovalManager>,
    pub pipeline_ops: Arc<dyn PipelineDelegateOps>,
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

#[async_trait]
impl Tool for PipelineDelegateTool {
    fn name(&self) -> &str {
        "pipeline_delegate"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "pipeline_delegate".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: PipelineDelegateArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "pipeline_delegate", agent_type = %args.agent_type, book_id = %args.book_id, "[tool] call");

        validate_id(&args.book_id, "book_id")
            .map_err(|e| {
                tracing::error!(tool = "pipeline_delegate", book_id = %args.book_id, error = %e, "[tool] invalid book_id");
                BookToolError::InvalidInput(e)
            })?;

        // 统一审批：pipeline 阶段均为重操作
        let approved = self.approval.request_approval(
            "pipeline_delegate",
            &serde_json::json!({ "agent_type": args.agent_type, "book_id": args.book_id, "params": args.params }),
        ).await;
        if !approved {
            tracing::error!(tool = "pipeline_delegate", agent_type = %args.agent_type, book_id = %args.book_id, "[tool] approval denied");
            return Err(BookToolError::Denied.into());
        }

        let engine = self.engine.clone();
        let pipeline_ops = self.pipeline_ops.clone();

        let result: Result<serde_json::Value, BookToolError> = match args.agent_type.as_str() {
            "plan_chapter" => pipeline_ops
                .plan_chapter(&engine, &args.book_id)
                .await
                .map_err(BookToolError::Failed),
            "write_draft" => {
                let wc = parse_word_count_override(&args.params)?;
                pipeline_ops
                    .write_draft(&engine, &args.book_id, wc)
                    .await
                    .map_err(BookToolError::Failed)
            }
            "audit_draft" => {
                let cn = parse_chapter_number(&args.params)?;
                pipeline_ops
                    .audit_draft(&engine, &args.book_id, cn)
                    .await
                    .map_err(BookToolError::Failed)
            }
            "revise_draft" => {
                let cn = parse_chapter_number(&args.params)?;
                let mode = parse_revise_mode(&args.params)?;
                pipeline_ops
                    .revise_draft(&engine, &args.book_id, cn, &mode)
                    .await
                    .map_err(BookToolError::Failed)
            }
            "write_next_chapter" => {
                let wc = parse_word_count_override(&args.params)?;
                pipeline_ops
                    .write_next_chapter(&engine, &args.book_id, wc)
                    .await
                    .map_err(BookToolError::Failed)
            }
            "consolidate" => {
                let book_dir = self.data_dir.books_dir().join(&args.book_id);
                if !book_dir.exists() {
                    tracing::error!(tool = "pipeline_delegate", agent_type = %args.agent_type, book_id = %args.book_id, "[tool] book directory not found");
                    return Err(BookToolError::InvalidInput(format!(
                        "书籍目录不存在: {}", args.book_id
                    )).into());
                }
                pipeline_ops
                    .consolidate(&engine, &args.book_id)
                    .await
                    .map_err(BookToolError::Failed)
            }
            other => {
                tracing::error!(tool = "pipeline_delegate", agent_type = %other, "[tool] unknown agent_type");
                Err(BookToolError::InvalidInput(format!(
                    "未知 agent_type: {}", other
                )))
            }
        };

        if result.is_ok() {
            tracing::info!(tool = "pipeline_delegate", agent_type = %args.agent_type, book_id = %args.book_id, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        }
        result.map_err(ToolError::from)
    }
}

// ── WriteTruthFileTool ─────────────────────────────────────

/// 编辑真相文件（白名单内）。委托 BookEditOps::truth_file_edit。
pub struct WriteTruthFileTool {
    pub approval: Arc<ApprovalManager>,
    pub edit_ops: Arc<dyn BookEditOps>,
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

#[async_trait]
impl Tool for WriteTruthFileTool {
    fn name(&self) -> &str {
        "write_truth_file"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_truth_file".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: WriteTruthFileArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let edit_ops = self.edit_ops.clone();
        execute_with_approval(
            &self.approval,
            "write_truth_file",
            &serde_json::json!({ "book_id": args.book_id, "file_name": args.file_name }),
            move || async move {
                edit_ops
                    .truth_file_edit(args.book_id, args.file_name, args.new_content)
                    .await
            },
        )
        .await
        .map_err(ToolError::from)
    }
}

// ── RenameEntityTool ───────────────────────────────────────

/// 全书实体改名。委托 BookEditOps::entity_rename。
pub struct RenameEntityTool {
    pub approval: Arc<ApprovalManager>,
    pub edit_ops: Arc<dyn BookEditOps>,
}

#[derive(Deserialize)]
pub struct RenameEntityArgs {
    pub book_id: String,
    pub old_name: String,
    pub new_name: String,
}

#[async_trait]
impl Tool for RenameEntityTool {
    fn name(&self) -> &str {
        "rename_entity"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rename_entity".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: RenameEntityArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let edit_ops = self.edit_ops.clone();
        execute_with_approval(
            &self.approval,
            "rename_entity",
            &serde_json::json!({ "book_id": args.book_id, "old_name": args.old_name, "new_name": args.new_name }),
            move || async move {
                edit_ops
                    .entity_rename(args.book_id, args.old_name, args.new_name)
                    .await
            },
        )
        .await
        .map_err(ToolError::from)
    }
}

// ── PatchChapterTextTool ───────────────────────────────────

/// 章节局部编辑（三级文本匹配：精确 → 弹性空格 → 段落近似）。
/// 委托 BookEditOps::chapter_local_edit。
pub struct PatchChapterTextTool {
    pub approval: Arc<ApprovalManager>,
    pub edit_ops: Arc<dyn BookEditOps>,
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

#[async_trait]
impl Tool for PatchChapterTextTool {
    fn name(&self) -> &str {
        "patch_chapter_text"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "patch_chapter_text".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: PatchChapterTextArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let edit_ops = self.edit_ops.clone();
        execute_with_approval(
            &self.approval,
            "patch_chapter_text",
            &serde_json::json!({ "book_id": args.book_id, "chapter_number": args.chapter_number }),
            move || async move {
                edit_ops
                    .chapter_local_edit(args.book_id, args.chapter_number, args.find, args.replace)
                    .await
            },
        )
        .await
        .map_err(ToolError::from)
    }
}

// ── ReplaceChapterTextTool ─────────────────────────────────

/// 章节整章替换。委托 BookEditOps::chapter_replace。
pub struct ReplaceChapterTextTool {
    pub approval: Arc<ApprovalManager>,
    pub edit_ops: Arc<dyn BookEditOps>,
}

#[derive(Deserialize)]
pub struct ReplaceChapterTextArgs {
    pub book_id: String,
    pub chapter_number: u32,
    pub new_content: String,
}

#[async_trait]
impl Tool for ReplaceChapterTextTool {
    fn name(&self) -> &str {
        "replace_chapter_text"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "replace_chapter_text".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ReplaceChapterTextArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let edit_ops = self.edit_ops.clone();
        execute_with_approval(
            &self.approval,
            "replace_chapter_text",
            &serde_json::json!({ "book_id": args.book_id, "chapter_number": args.chapter_number }),
            move || async move {
                edit_ops
                    .chapter_replace(args.book_id, args.chapter_number, args.new_content)
                    .await
            },
        )
        .await
        .map_err(ToolError::from)
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

#[async_trait]
impl Tool for ImportChaptersTool {
    fn name(&self) -> &str {
        "import_chapters"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "import_chapters".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ImportChaptersArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "import_chapters", book_id = %args.book_id, source = %args.source_path, "[tool] call");

        validate_id(&args.book_id, "book_id")
            .map_err(|e| {
                tracing::error!(tool = "import_chapters", book_id = %args.book_id, error = %e, "[tool] invalid book_id");
                BookToolError::InvalidInput(e)
            })?;

        let book_dir = self.data_dir.books_dir().join(&args.book_id);
        if !book_dir.exists() {
            tracing::error!(tool = "import_chapters", book_id = %args.book_id, "[tool] book directory not found");
            return Err(BookToolError::InvalidInput(format!(
                "书籍目录不存在: {}", args.book_id
            )).into());
        }

        // 源文件路径须锁定在 book_dir 之下
        let source = resolve_under_book(&book_dir, &args.source_path)?;
        if !source.is_file() {
            tracing::error!(tool = "import_chapters", source = %args.source_path, "[tool] source file not found");
            return Err(BookToolError::InvalidInput(format!(
                "源文件不存在: {}", args.source_path
            )).into());
        }

        let approved = self.approval.request_approval(
            "import_chapters",
            &serde_json::json!({ "book_id": args.book_id, "source_path": args.source_path }),
        ).await;
        if !approved {
            tracing::error!(tool = "import_chapters", book_id = %args.book_id, source = %args.source_path, "[tool] approval denied");
            return Err(BookToolError::Denied.into());
        }

        let split_pattern = args.split_pattern.clone();
        // 所有文件 I/O 卸载到阻塞线程池
        let written = tokio::task::spawn_blocking(move || -> Result<Vec<serde_json::Value>, BookToolError> {
            // 大小检查：防止一次性读入超大文件
            let metadata = std::fs::metadata(&source)
                .map_err(|e| {
                    tracing::error!(tool = "import_chapters", error = %e, "[tool] metadata failed");
                    BookToolError::Io(e.to_string())
                })?;
            if metadata.len() > MAX_READ_SIZE as u64 {
                tracing::error!(tool = "import_chapters", size = metadata.len(), max = MAX_READ_SIZE, "[tool] source file too large");
                return Err(BookToolError::InvalidInput(format!(
                    "Source file too large ({} bytes > {} max)", metadata.len(), MAX_READ_SIZE
                )));
            }
            let content = std::fs::read_to_string(&source)?;
            let chunks = split_into_chapters(&content, split_pattern.as_deref())?;
            if chunks.is_empty() {
                tracing::error!(tool = "import_chapters", "[tool] no chapters extracted");
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
            Ok(written)
        })
        .await
        .map_err(|e| {
            tracing::error!(tool = "import_chapters", error = %e, "[tool] spawn_blocking join failed");
            BookToolError::Failed(format!("spawn_blocking join failed: {}", e))
        })??;

        tracing::info!(tool = "import_chapters", book_id = %args.book_id, imported_count = written.len(), duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
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

#[async_trait]
impl Tool for GenerateCoverTool {
    fn name(&self) -> &str {
        "generate_cover"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "generate_cover".to_string(),
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

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: GenerateCoverArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let start = Instant::now();
        tracing::info!(tool = "generate_cover", book_id = %args.book_id, "[tool] call");

        validate_id(&args.book_id, "book_id")
            .map_err(|e| {
                tracing::error!(tool = "generate_cover", book_id = %args.book_id, error = %e, "[tool] invalid book_id");
                BookToolError::InvalidInput(e)
            })?;

        let approved = self.approval.request_approval(
            "generate_cover",
            &serde_json::json!({ "book_id": args.book_id }),
        ).await;
        if !approved {
            tracing::error!(tool = "generate_cover", book_id = %args.book_id, "[tool] approval denied");
            return Err(BookToolError::Denied.into());
        }

        let book_dir = self.data_dir.books_dir().join(&args.book_id);
        if !book_dir.exists() {
            tracing::error!(tool = "generate_cover", book_id = %args.book_id, "[tool] book directory not found");
            return Err(BookToolError::InvalidInput(format!(
                "书籍目录不存在: {}", args.book_id
            )).into());
        }
        let description = args.description.clone();
        // create_dir + write 卸载到阻塞线程池
        tokio::task::spawn_blocking(move || -> Result<(), BookToolError> {
            let story_dir = book_dir.join("story");
            std::fs::create_dir_all(&story_dir)
                .map_err(|e| {
                    tracing::error!(tool = "generate_cover", error = %e, "[tool] create_dir_all failed");
                    BookToolError::Io(e.to_string())
                })?;
            let cover_path = story_dir.join("cover-prompt.md");
            std::fs::write(&cover_path, ensure_trailing_newline(&description))
                .map_err(|e| {
                    tracing::error!(tool = "generate_cover", error = %e, "[tool] write failed");
                    BookToolError::Io(e.to_string())
                })?;
            Ok(())
        })
        .await
        .map_err(|e| {
            tracing::error!(tool = "generate_cover", error = %e, "[tool] spawn_blocking join failed");
            BookToolError::Failed(format!("spawn_blocking join failed: {}", e))
        })??;

        tracing::info!(tool = "generate_cover", book_id = %args.book_id, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(serde_json::json!({
            "saved": true,
            "path": "story/cover-prompt.md",
            "note": "图片生成未实现，仅落盘封面提示词"
        }))
    }
}

// ── 共享：审批 + trait 操作执行 ─────────────────────────────

/// 通用流程：审批 → 调用 trait 操作（返回 Value）→ 日志。
///
/// `op` 是一个返回 `Future<Output = Result<serde_json::Value, String>>` 的闭包，
/// 由各 Tool 提供，封装对 BookEditOps / PipelineDelegateOps / ResearchOps 的调用。
/// 这样各 Tool 只需关注参数解析与 trait 方法选择，审批/日志/错误转换统一在此处理。
async fn execute_with_approval<F, Fut>(
    approval: &Arc<ApprovalManager>,
    tool_name: &str,
    payload: &serde_json::Value,
    op: F,
) -> Result<serde_json::Value, BookToolError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>>,
{
    let start = Instant::now();
    tracing::info!(tool = %tool_name, "[tool] call");

    let approved = approval.request_approval(tool_name, payload).await;
    if !approved {
        tracing::error!(tool = %tool_name, "[tool] approval denied");
        return Err(BookToolError::Denied);
    }

    let result = op().await.map_err(BookToolError::Failed)?;

    tracing::info!(tool = %tool_name, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
    Ok(result)
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

/// 从 params 中解析 revise mode（缺省 "auto"）。
/// 返回字符串形式（auto/polish/rewrite/rework/antidetect/spotfix），
/// 由 PipelineDelegateOps 实现方映射到 domain::pipeline::agents::reviser::ReviseMode。
fn parse_revise_mode(params: &serde_json::Value) -> Result<String, BookToolError> {
    let s = params.get("mode").and_then(|v| v.as_str()).unwrap_or("auto");
    match s.to_lowercase().as_str() {
        "auto" | "polish" | "rewrite" | "rework" | "antidetect" | "spotfix" => {
            Ok(s.to_lowercase())
        }
        other => Err(BookToolError::InvalidInput(format!("未知 revise mode: {}", other))),
    }
}

// ── 辅助：路径与章节切分 ───────────────────────────────────

/// 将相对路径解析到 book_dir 之下，拒绝 `..` 与绝对路径。
fn resolve_under_book(book_dir: &Path, rel: &str) -> Result<PathBuf, BookToolError> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Err(BookToolError::InvalidInput("路径不能为空".to_string()));
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(BookToolError::PathTraversal);
    }
    // 用 security_kernel 的 validate_path_for_creation 做精确的 component-based 校验：
    // - Component::ParentDir 精确检测（避免 contains("..") 误判 `my..file.txt`）
    // - 无需文件已存在（creation 场景）
    // - 解析后 canonical_base starts_with 检查，杜绝符号化穿越
    let canonical = crate::security_kernel::validation::path::validate_path_for_creation(
        std::path::Path::new(trimmed),
        book_dir,
    )
    .map_err(|_| BookToolError::PathTraversal)?;
    Ok(canonical.into_inner())
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
        // 取 file_stem（去掉 .md 扩展名），再提取前导数字。
        // 不用 name.get(..4) 硬取 4 字节：对非 4 位零填充文件名（如 "12.md"）
        // 会取到 "12.m" 解析失败，漏掉有效章节号。
        let path = entry.path();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let leading_digits: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = leading_digits.parse::<u32>() {
            if n > max {
                max = n;
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
