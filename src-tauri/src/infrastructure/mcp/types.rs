//! ═══════════════════════════════════════════════════════════════════════════
//! MCP 类型 - 类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! serde 结构统一使用 `#[serde(rename_all = "camelCase")]` 以对齐前端契约。
//! 传输类型使用 `#[serde(tag = "type")]` 实现判别式联合。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// MCP server 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    /// 唯一标识（前端生成，建议 UUID v4）
    pub id: String,
    /// 人类可读名称
    pub name: String,
    /// 传输配置
    pub transport: McpTransport,
    /// 是否启用（禁用的 server 不会被 registry 连接）
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否在应用启动时自动连接
    #[serde(default)]
    pub auto_start: bool,
}

/// MCP 传输方式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransport {
    /// stdio 传输：启动子进程，通过 stdin/stdout 通信
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// HTTP 传输（尚未实现）
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// SSE 传输（尚未实现）
    Sse { url: String },
}

/// MCP 工具描述（聚合视图，附带 server_id 用于路由）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// 来源 server id（由 registry 注入，非 MCP 协议字段）
    pub server_id: String,
}

/// MCP 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallResult {
    pub content: Vec<McpContent>,
    #[serde(default)]
    pub is_error: bool,
}

/// MCP 内容块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
}

/// server 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerTestResult {
    pub server_id: String,
    pub connected: bool,
    #[serde(default)]
    pub server_info: serde_json::Value,
    #[serde(default)]
    pub tools: Vec<McpTool>,
    #[serde(default)]
    pub error: Option<String>,
}

fn default_true() -> bool {
    true
}
