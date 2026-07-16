// MCP IPC 命令 —— mcp_* 系列命令的 Tauri 入口。
//
// 命令清单：
// - mcp_list_servers → 列出所有配置的 server
// - mcp_add_server(config) → 添加 server 配置
// - mcp_update_server(id, config) → 更新配置
// - mcp_remove_server(id) → 删除配置
// - mcp_test_server(id) → 测试连接（initialize + list_tools）
// - mcp_list_tools(serverId?) → 列出工具（指定 server 或全部）
// - mcp_call_tool(serverId, toolName, arguments) → 调用工具
//
// IPC 命名约定：Rust 参数用 snake_case，前端调用时用 camelCase（Tauri 自动转换）。

use tauri::State;

use crate::infrastructure::mcp::state::McpState;
use crate::infrastructure::mcp::types::{
    McpServerConfig, McpServerTestResult, McpTool, McpToolCallResult,
};
use crate::shared::error::{AppError, IpcResponse};

/// 列出所有已配置的 MCP server。
#[tauri::command]
pub async fn mcp_list_servers(
    state: State<'_, McpState>,
) -> Result<IpcResponse<Vec<McpServerConfig>>, AppError> {
    Ok(IpcResponse::ok(state.list_servers()?))
}

/// 添加 MCP server 配置。
#[tauri::command]
pub async fn mcp_add_server(
    config: McpServerConfig,
    state: State<'_, McpState>,
) -> Result<IpcResponse<()>, AppError> {
    state.add_server(config)?;
    Ok(IpcResponse::ok(()))
}

/// 更新 MCP server 配置（id 为旧 id，config 可包含新 id）。
#[tauri::command]
pub async fn mcp_update_server(
    id: String,
    config: McpServerConfig,
    state: State<'_, McpState>,
) -> Result<IpcResponse<()>, AppError> {
    state.update_server(&id, config).await?;
    Ok(IpcResponse::ok(()))
}

/// 移除 MCP server 配置。
#[tauri::command]
pub async fn mcp_remove_server(
    id: String,
    state: State<'_, McpState>,
) -> Result<IpcResponse<()>, AppError> {
    state.remove_server(&id).await?;
    Ok(IpcResponse::ok(()))
}

/// 测试 MCP server 连接（initialize + list_tools）。
#[tauri::command]
pub async fn mcp_test_server(
    id: String,
    state: State<'_, McpState>,
) -> Result<IpcResponse<McpServerTestResult>, AppError> {
    let result = state.test_server(&id).await?;
    Ok(IpcResponse::ok(result))
}

/// 列出 MCP 工具：server_id 为 None 时聚合所有 enabled server。
#[tauri::command]
pub async fn mcp_list_tools(
    server_id: Option<String>,
    state: State<'_, McpState>,
) -> Result<IpcResponse<Vec<McpTool>>, AppError> {
    let tools = state.list_tools(server_id.as_deref()).await?;
    Ok(IpcResponse::ok(tools))
}

/// 调用 MCP 工具。
#[tauri::command]
pub async fn mcp_call_tool(
    server_id: String,
    tool_name: String,
    arguments: serde_json::Value,
    state: State<'_, McpState>,
) -> Result<IpcResponse<McpToolCallResult>, AppError> {
    let result = state.call_tool(&server_id, &tool_name, arguments).await?;
    Ok(IpcResponse::ok(result))
}
