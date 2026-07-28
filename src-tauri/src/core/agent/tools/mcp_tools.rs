//! ═══════════════════════════════════════════════════════════════════════════
//! MCPTools - MCP 工具路由器
//! ═══════════════════════════════════════════════════════════════════════════

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::infrastructure::llm::tool::{Tool, ToolDefinition, ToolError};

/// MCP 工具调用参数
#[derive(Deserialize)]
pub struct McpCallArgs {
    pub server_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// MCP 工具调用结果
#[derive(Serialize)]
pub struct McpCallOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MCP 工具调用器
///
/// 允许 Agent 调用配置的 MCP server 提供的工具。
/// 通过 IPC 与 McpState 通信，无需 AgentEngine 直接持有 McpState。
pub struct McpCallTool;

#[async_trait]
impl Tool for McpCallTool {
    fn name(&self) -> &str {
        "mcp_call"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: "Call a tool from a configured MCP server. Use mcp_list_tools first to discover available tools.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "server_id": {
                        "type": "string",
                        "description": "The MCP server ID to call (e.g., 'github', 'slack')"
                    },
                    "tool_name": {
                        "type": "string",
                        "description": "The name of the tool to call on the MCP server"
                    },
                    "arguments": {
                        "type": "object",
                        "description": "The arguments to pass to the MCP tool"
                    }
                },
                "required": ["server_id", "tool_name"]
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let _args: McpCallArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        tracing::info!(tool = "mcp_call", "[tool] call (not implemented)");

        tracing::warn!(tool = "mcp_call", "[tool] McpCallTool requires McpState integration");
        let output = McpCallOutput {
            success: false,
            result: None,
            error: Some("McpCallTool requires McpState integration. Use mcp_* IPC commands instead.".to_string()),
        };
        Ok(serde_json::to_value(&output).map_err(|e| ToolError::Other(e.to_string()))?)
    }
}

/// MCP 工具列表查询参数
#[derive(Deserialize)]
pub struct McpListToolsArgs {
    #[serde(default)]
    pub server_id: Option<String>,
}

/// MCP 工具信息
#[derive(Serialize)]
pub struct McpToolInfo {
    pub name: String,
    pub server_id: String,
    pub description: Option<String>,
}

/// MCP 工具列表查询结果
#[derive(Serialize)]
pub struct McpListToolsOutput {
    pub tools: Vec<McpToolInfo>,
}

/// MCP 工具列表查询器
pub struct McpListToolsTool;

#[async_trait]
impl Tool for McpListToolsTool {
    fn name(&self) -> &str {
        "mcp_list_tools"
    }

    async fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: "List available tools from MCP servers. If server_id is provided, list tools from that server only.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "server_id": {
                        "type": "string",
                        "description": "Optional server ID to filter tools. If not provided, lists all tools from all enabled servers."
                    }
                }
            }),
        }
    }

    async fn call(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let _args: McpListToolsArgs =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        tracing::info!(tool = "mcp_list_tools", "[tool] call (not implemented)");

        tracing::warn!(tool = "mcp_list_tools", "[tool] McpListToolsTool requires McpState integration");
        let output = McpListToolsOutput {
            tools: vec![McpToolInfo {
                name: "mcp_integration_pending".to_string(),
                server_id: "system".to_string(),
                description: Some("MCP tool integration requires McpState. Use mcp_list_tools IPC command.".to_string()),
            }],
        };
        Ok(serde_json::to_value(&output).map_err(|e| ToolError::Other(e.to_string()))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_call_tool_name() {
        assert_eq!(McpCallTool.name(), "mcp_call");
    }

    #[test]
    fn mcp_list_tools_tool_name() {
        assert_eq!(McpListToolsTool.name(), "mcp_list_tools");
    }
}
