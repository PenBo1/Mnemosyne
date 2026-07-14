// MCP 注册表 —— 管理活跃的 MCP server 连接，聚合工具，路由调用。
//
// 设计：
// - clients: server_id → McpClient 的映射
// - 懒连接：list_tools / call_tool 时若未连接则按需 connect + initialize
// - 工具聚合：list_all_tools 遍历所有 enabled server，注入 server_id
// - 配置变更由 state 层协调（remove → disconnect、update → reconnect）

use std::collections::HashMap;

use crate::infrastructure::mcp::client::McpClient;
use crate::infrastructure::mcp::types::{McpServerConfig, McpTool, McpToolCallResult};
use crate::shared::error::AppError;

/// MCP 连接注册表
pub struct McpRegistry {
    clients: HashMap<String, McpClient>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// 是否已连接。
    pub fn is_connected(&mut self, server_id: &str) -> bool {
        match self.clients.get_mut(server_id) {
            Some(client) => client.is_alive(),
            None => false,
        }
    }

    /// 确保已连接（未连接则 connect + initialize）。
    ///
    /// 若已存在连接但子进程已退出，则先移除旧连接再重连。
    pub async fn ensure_connected(&mut self, server: &McpServerConfig) -> Result<(), AppError> {
        if !server.enabled {
            return Err(AppError::invalid_state(format!(
                "MCP server '{}' is disabled",
                server.id
            )));
        }

        // 已连接且存活 → 直接返回
        if let Some(client) = self.clients.get_mut(&server.id) {
            if client.is_alive() {
                return Ok(());
            }
            // 子进程已退出，移除旧连接
            self.clients.remove(&server.id);
        }

        let mut client = McpClient::connect(&server.transport).await?;
        client.initialize().await?;
        self.clients.insert(server.id.clone(), client);
        Ok(())
    }

    /// 列出指定 server 的工具（注入 server_id）。
    pub async fn list_tools(&mut self, server: &McpServerConfig) -> Result<Vec<McpTool>, AppError> {
        self.ensure_connected(server).await?;
        let client = self
            .clients
            .get_mut(&server.id)
            .ok_or_else(|| AppError::internal("MCP client missing after ensure_connected"))?;
        let mut tools = client.list_tools().await?;
        for t in &mut tools {
            t.server_id = server.id.clone();
        }
        Ok(tools)
    }

    /// 聚合所有 enabled server 的工具。
    ///
    /// 返回 (成功聚合的工具, 失败的 server 错误列表)。
    /// 单个 server 失败不影响其它 server（错误记录而非整体失败）。
    pub async fn list_all_tools(
        &mut self,
        servers: &[McpServerConfig],
    ) -> (Vec<McpTool>, Vec<(String, String)>) {
        let mut all_tools = Vec::new();
        let mut errors = Vec::new();

        for server in servers.iter().filter(|s| s.enabled) {
            match self.list_tools(server).await {
                Ok(tools) => all_tools.extend(tools),
                Err(e) => errors.push((server.id.clone(), e.message)),
            }
        }

        (all_tools, errors)
    }

    /// 调用工具（路由到指定 server）。
    pub async fn call_tool(
        &mut self,
        server: &McpServerConfig,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResult, AppError> {
        self.ensure_connected(server).await?;
        let client = self
            .clients
            .get_mut(&server.id)
            .ok_or_else(|| AppError::internal("MCP client missing after ensure_connected"))?;
        client.call_tool(tool_name, arguments).await
    }

    /// 断开指定 server 连接。
    pub fn disconnect(&mut self, server_id: &str) {
        if let Some(mut client) = self.clients.remove(server_id) {
            client.shutdown();
        }
    }

    /// 断开所有连接。
    pub fn disconnect_all(&mut self) {
        for (_, mut client) in self.clients.drain() {
            client.shutdown();
        }
    }

    /// 已连接的 server 数量。
    pub fn connected_count(&self) -> usize {
        self.clients.len()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}
