//! ═══════════════════════════════════════════════════════════════════════════
//! ResearchTools - 研究与材料工具
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Deserialize;

use crate::core::agent::engine::AgentEngine;
use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};

use super::book_ops::ResearchOps;

// ── ResearchWebTool ───────────────────────────────────────

/// 研究工具：基于 LLM 知识库生成结构化研究报告。
pub struct ResearchWebTool {
    pub engine: Arc<AgentEngine>,
    pub research_ops: Arc<dyn ResearchOps>,
}

#[derive(Deserialize)]
pub struct ResearchWebArgs {
    /// 研究主题
    pub query: String,
    /// 研究深度：quick / standard / deep
    #[serde(default)]
    pub depth: Option<String>,
}

#[async_trait]
impl Tool for ResearchWebTool {
    fn name(&self) -> &str {
        "research_web"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "research_web".to_string(),
            description: "对一个主题进行研究并返回结构化研究报告（claims/conflicts/unknowns/creativeImplications + markdown）。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "研究主题（≤500 字符）"
                    },
                    "depth": {
                        "type": "string",
                        "enum": ["quick", "standard", "deep"],
                        "description": "研究深度，缺省为 standard"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: ResearchWebArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let start = Instant::now();
        let query_preview = if args.query.len() > 100 {
            format!("{}...", &args.query[..100])
        } else {
            args.query.clone()
        };
        tracing::info!(tool = "research_web", query = %query_preview, depth = ?args.depth, "[tool] call");

        let result = self
            .research_ops
            .run_research_report(&self.engine, args.query, args.depth)
            .await
            .map_err(|e| {
                tracing::error!(tool = "research_web", error = %e, "[tool] run_research_report failed");
                ToolError::Execution(e)
            })?;

        tracing::info!(tool = "research_web", query = %query_preview, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(result)
    }
}

// ── IngestMaterialTool ────────────────────────────────────

/// 材料导入工具：从 URL 或本地文件抓取并落盘，返回资产清单。
pub struct IngestMaterialTool {
    pub research_ops: Arc<dyn ResearchOps>,
}

#[derive(Deserialize)]
pub struct IngestMaterialArgs {
    /// 来源类型：url / file
    pub source_kind: String,
    /// 当 source_kind=url 时的 URL
    #[serde(default)]
    pub url: Option<String>,
    /// 当 source_kind=file 时的本地文件路径
    #[serde(default)]
    pub file_path: Option<String>,
    /// 可选文件名覆盖
    #[serde(default)]
    pub filename: Option<String>,
    /// 可选 mime 覆盖
    #[serde(default)]
    pub mime_type: Option<String>,
    /// 可选标题
    #[serde(default)]
    pub title: Option<String>,
    /// 用途标签（reference/worldbuilding/script/...），缺省 reference
    #[serde(default)]
    pub purpose: Option<String>,
}

#[async_trait]
impl Tool for IngestMaterialTool {
    fn name(&self) -> &str {
        "ingest_material"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ingest_material".to_string(),
            description: "导入一份材料（URL 或本地文件），提取正文并落盘，返回材料资产清单。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_kind": {
                        "type": "string",
                        "enum": ["url", "file"],
                        "description": "来源类型"
                    },
                    "url": { "type": "string", "description": "source_kind=url 时的 URL（http/https）" },
                    "file_path": { "type": "string", "description": "source_kind=file 时的本地文件绝对路径" },
                    "filename": { "type": "string", "description": "可选文件名覆盖" },
                    "mime_type": { "type": "string", "description": "可选 mime 覆盖" },
                    "title": { "type": "string", "description": "可选标题" },
                    "purpose": { "type": "string", "description": "用途标签，缺省 reference" }
                },
                "required": ["source_kind"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: IngestMaterialArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let start = Instant::now();
        tracing::info!(tool = "ingest_material", source_kind = %args.source_kind, "[tool] call");

        let result = self
            .research_ops
            .ingest_material(
                args.source_kind.clone(),
                args.url,
                args.file_path,
                args.filename,
                args.mime_type,
                args.title,
                args.purpose,
            )
            .await
            .map_err(|e| {
                tracing::error!(tool = "ingest_material", error = %e, "[tool] ingest_material failed");
                ToolError::Execution(e)
            })?;

        tracing::info!(tool = "ingest_material", source_kind = %args.source_kind, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(result)
    }
}

// ── RetrieveMaterialTool ──────────────────────────────────

/// 材料检索工具：按关键词匹配已导入材料，返回相关片段。
pub struct RetrieveMaterialTool {
    pub research_ops: Arc<dyn ResearchOps>,
}

#[derive(Deserialize)]
pub struct RetrieveMaterialArgs {
    /// 检索关键词
    pub query: String,
    /// 可选用途过滤
    #[serde(default)]
    pub purpose: Option<String>,
    /// 可选返回上限（1-12，缺省 5）
    #[serde(default)]
    pub limit: Option<u32>,
}

#[async_trait]
impl Tool for RetrieveMaterialTool {
    fn name(&self) -> &str {
        "retrieve_material"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "retrieve_material".to_string(),
            description: "检索已导入的辅助材料，按相关度倒序返回命中的片段（含 score/excerpt）。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "检索关键词" },
                    "purpose": { "type": "string", "description": "可选用途过滤" },
                    "limit": { "type": "integer", "description": "可选返回上限（1-12，缺省 5）" }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let args: RetrieveMaterialArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let start = Instant::now();
        let query_preview = if args.query.len() > 50 {
            format!("{}...", &args.query[..50])
        } else {
            args.query.clone()
        };
        tracing::info!(tool = "retrieve_material", query = %query_preview, purpose = ?args.purpose, limit = ?args.limit, "[tool] call");

        let result = self
            .research_ops
            .retrieve_materials(args.query, args.purpose, args.limit)
            .await
            .map_err(|e| {
                tracing::error!(tool = "retrieve_material", error = %e, "[tool] retrieve_materials failed");
                ToolError::Execution(e)
            })?;

        tracing::info!(tool = "retrieve_material", query = %query_preview, duration_ms = start.elapsed().as_millis() as u64, "[tool] completed");
        Ok(result)
    }
}
