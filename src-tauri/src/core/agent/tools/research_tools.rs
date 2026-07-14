// 研究与材料工具：ResearchWebTool / IngestMaterialTool / RetrieveMaterialTool。
//
// 这三个工具均为只读或外部输入型，无破坏性副作用，可无条件注入 agent。
// - ResearchWebTool 调用 researcher::report::run_research_report（依赖 AgentEngine）
// - IngestMaterialTool 调用 materials::ingest::ingest_material（落盘 markdown + JSON 清单）
// - RetrieveMaterialTool 调用 materials::retrieve::retrieve_materials（纯读）
//
// 注：本文件位于 core/agent/tools，但需调用 domain 层函数，属于任务要求的
// 显式指令（见任务描述）。与 AGENTS.md "core/agent 不依赖 domain" 规则存在张力，
// 此处遵循任务指令实现，后续可考虑通过 application 层注入 trait 解耦。

use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;

use crate::core::agent::engine::AgentEngine;
use crate::domain::materials::ingest::ingest_material;
use crate::domain::materials::retrieve::retrieve_materials;
use crate::domain::materials::types::{IngestMaterialInput, RetrieveMaterialsInput};
use crate::domain::researcher::report::run_research_report;
use crate::domain::researcher::types::ResearchInput;
use crate::infrastructure::fs::data_dir::DataDir;

// ── ResearchWebTool ───────────────────────────────────────

/// 研究工具：基于 LLM 知识库生成结构化研究报告。
pub struct ResearchWebTool {
    pub engine: Arc<AgentEngine>,
}

#[derive(Deserialize)]
pub struct ResearchWebArgs {
    /// 研究主题
    pub query: String,
    /// 研究深度：quick / standard / deep
    #[serde(default)]
    pub depth: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ResearchWebError {
    #[error("研究失败: {0}")]
    Failed(String),
}

impl Tool for ResearchWebTool {
    const NAME: &'static str = "research_web";

    type Error = ResearchWebError;
    type Args = ResearchWebArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let input = ResearchInput {
            query: args.query,
            depth: args.depth,
        };
        let report = run_research_report(&self.engine, &input)
            .await
            .map_err(|e| ResearchWebError::Failed(e.to_string()))?;
        // ResearchReport 已实现 Serialize(camelCase)，直接转 Value
        serde_json::to_value(&report)
            .map_err(|e| ResearchWebError::Failed(e.to_string()))
    }
}

// ── IngestMaterialTool ────────────────────────────────────

/// 材料导入工具：从 URL 或本地文件抓取并落盘，返回资产清单。
pub struct IngestMaterialTool {
    pub data_dir: DataDir,
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

#[derive(Debug, thiserror::Error)]
pub enum IngestMaterialError {
    #[error("导入失败: {0}")]
    Failed(String),
}

impl Tool for IngestMaterialTool {
    const NAME: &'static str = "ingest_material";

    type Error = IngestMaterialError;
    type Args = IngestMaterialArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let input = IngestMaterialInput {
            source_kind: args.source_kind,
            url: args.url,
            file_path: args.file_path,
            filename: args.filename,
            mime_type: args.mime_type,
            title: args.title,
            purpose: args.purpose,
        };
        let asset = ingest_material(&self.data_dir, &input)
            .await
            .map_err(|e| IngestMaterialError::Failed(e.to_string()))?;
        serde_json::to_value(&asset)
            .map_err(|e| IngestMaterialError::Failed(e.to_string()))
    }
}

// ── RetrieveMaterialTool ──────────────────────────────────

/// 材料检索工具：按关键词匹配已导入材料，返回相关片段。
pub struct RetrieveMaterialTool {
    pub data_dir: DataDir,
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

#[derive(Debug, thiserror::Error)]
pub enum RetrieveMaterialError {
    #[error("检索失败: {0}")]
    Failed(String),
}

impl Tool for RetrieveMaterialTool {
    const NAME: &'static str = "retrieve_material";

    type Error = RetrieveMaterialError;
    type Args = RetrieveMaterialArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let input = RetrieveMaterialsInput {
            query: args.query,
            purpose: args.purpose,
            limit: args.limit,
        };
        let results = retrieve_materials(&self.data_dir, &input)
            .map_err(|e| RetrieveMaterialError::Failed(e.to_string()))?;
        serde_json::to_value(&results)
            .map_err(|e| RetrieveMaterialError::Failed(e.to_string()))
    }
}
