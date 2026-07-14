// MCP (Model Context Protocol) 基础模块。
//
// 最小可行实现：
// - 配置管理（McpConfig 持久化到 <data_dir>/mcp_config.json）
// - stdio 传输的 MCP 客户端（JSON-RPC 2.0 over newline-delimited stdio）
// - 工具注册表（McpRegistry 聚合多 server 工具，路由调用）
// - IPC 命令（mcp_* 系列）
//
// 不实现完整的 MCP 协议（resources/prompts/sampling 等），仅支持
// initialize / list_tools / call_tool 三个核心方法。
// HTTP/SSE 传输为 stub，返回 NOT_IMPLEMENTED。

pub mod types;
pub mod config;
pub mod client;
pub mod registry;
pub mod state;
pub mod commands;

pub use state::McpState;
