//! ═══════════════════════════════════════════════════════════════════════════
//! 工具抽象 - 自研 Tool Trait
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 项目内统一的工具抽象，不依赖外部 Agent 框架。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::types::ToolSpec;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: serde_json::Value,
}

/// 工具错误类型
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// IO 错误
    #[error("IO error: {0}")]
    Io(String),
    /// 路径穿越
    #[error("Path traversal not allowed")]
    PathTraversal,
    /// 无效参数
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    /// 执行错误
    #[error("Execution error: {0}")]
    Execution(String),
    /// 其他错误
    #[error("Other: {0}")]
    Other(String),
}

/// 工具 Trait
///
/// 每个 Tool 实现需提供：
/// - name(): 返回工具名称
/// - definition(): 返回 JSON Schema 描述
/// - call(): 执行工具调用
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;

    /// 返回工具定义
    async fn definition(&self) -> ToolDefinition;

    /// 执行工具调用
    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

/// 将 Tool 转换为 ToolSpec
pub async fn tool_to_spec(tool: &dyn Tool) -> ToolSpec {
    let def = tool.definition().await;
    ToolSpec {
        name: def.name,
        description: def.description,
        parameters: def.parameters,
    }
}

/// 批量转换工具列表为 ToolSpec 列表
pub async fn tools_to_specs(tools: &[std::sync::Arc<dyn Tool>]) -> Vec<ToolSpec> {
    let mut specs = Vec::with_capacity(tools.len());
    for tool in tools {
        specs.push(tool_to_spec(tool.as_ref()).await);
    }
    specs
}