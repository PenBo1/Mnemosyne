// MCP State —— Tauri managed state，协调 McpConfig 与 McpRegistry。
//
// 设计：
// - config: std::sync::RwLock（读写快速，不跨 await 持有）
// - registry: tokio::sync::Mutex（操作跨 await，必须用 tokio Mutex）
// - data_dir: 配置持久化路径
//
// 配置变更时主动断开受影响连接（update / remove），强制下次调用重连。

use std::sync::{Arc, RwLock};

use tokio::sync::Mutex;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::mcp::config::McpConfig;
use crate::infrastructure::mcp::registry::McpRegistry;
use crate::infrastructure::mcp::types::{
    McpServerConfig, McpServerTestResult, McpTool, McpToolCallResult,
};
use crate::shared::error::AppError;

/// MCP 全局状态
#[derive(Clone)]
pub struct McpState {
    config: Arc<RwLock<McpConfig>>,
    registry: Arc<Mutex<McpRegistry>>,
    data_dir: DataDir,
}

impl McpState {
    pub fn new(data_dir: DataDir) -> Self {
        let config = McpConfig::load(&data_dir).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load mcp_config.json, using default");
            McpConfig::default()
        });
        Self {
            config: Arc::new(RwLock::new(config)),
            registry: Arc::new(Mutex::new(McpRegistry::new())),
            data_dir,
        }
    }

    /// 列出所有 server 配置。
    pub fn list_servers(&self) -> Vec<McpServerConfig> {
        self.config
            .read()
            .map(|c| c.list())
            .unwrap_or_default()
    }

    /// 添加 server 配置并持久化。
    pub fn add_server(&self, server: McpServerConfig) -> Result<(), AppError> {
        let mut guard = self
            .config
            .write()
            .map_err(|e| AppError::internal(format!("Config lock poisoned: {}", e)))?;
        guard.add(server)?;
        guard.save(&self.data_dir)?;
        Ok(())
    }

    /// 更新 server 配置并持久化，断开旧连接。
    pub async fn update_server(&self, id: &str, server: McpServerConfig) -> Result<(), AppError> {
        {
            let mut guard = self
                .config
                .write()
                .map_err(|e| AppError::internal(format!("Config lock poisoned: {}", e)))?;
            guard.update(id, server)?;
            guard.save(&self.data_dir)?;
        }
        // 配置变更后断开旧连接，下次调用时按新配置重连
        self.disconnect_server(id).await;
        Ok(())
    }

    /// 移除 server 配置并持久化，断开连接。
    pub async fn remove_server(&self, id: &str) -> Result<McpServerConfig, AppError> {
        let removed = {
            let mut guard = self
                .config
                .write()
                .map_err(|e| AppError::internal(format!("Config lock poisoned: {}", e)))?;
            let r = guard
                .remove(id)
                .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", id)))?;
            guard.save(&self.data_dir)?;
            r
        };
        self.disconnect_server(id).await;
        Ok(removed)
    }

    /// 查找单个 server 配置（克隆）。
    pub fn find_server(&self, id: &str) -> Option<McpServerConfig> {
        self.config
            .read()
            .ok()
            .and_then(|c| c.find(id).cloned())
    }

    /// 测试 server 连接（ensure_connected + list_tools）。
    ///
    /// 连接成功后保留在 registry 中供后续调用。
    pub async fn test_server(&self, id: &str) -> Result<McpServerTestResult, AppError> {
        let server = self
            .find_server(id)
            .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", id)))?;

        let mut registry = self.registry.lock().await;
        match registry.list_tools(&server).await {
            Ok(tools) => Ok(McpServerTestResult {
                server_id: server.id.clone(),
                connected: true,
                server_info: serde_json::Value::Null,
                tools,
                error: None,
            }),
            Err(e) => {
                // 测试失败时断开可能存在的半连接
                registry.disconnect(&server.id);
                Ok(McpServerTestResult {
                    server_id: server.id.clone(),
                    connected: false,
                    server_info: serde_json::Value::Null,
                    tools: vec![],
                    error: Some(e.message),
                })
            }
        }
    }

    /// 列出工具：指定 server 则列单个，None 则聚合所有 enabled server。
    pub async fn list_tools(&self, server_id: Option<&str>) -> Result<Vec<McpTool>, AppError> {
        let mut registry = self.registry.lock().await;
        match server_id {
            Some(id) => {
                let server = self
                    .find_server(id)
                    .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", id)))?;
                registry.list_tools(&server).await
            }
            None => {
                let servers = self.list_servers();
                let (tools, errors) = registry.list_all_tools(&servers).await;
                if !errors.is_empty() {
                    tracing::warn!(
                        count = errors.len(),
                        "Some MCP servers failed to list tools"
                    );
                }
                Ok(tools)
            }
        }
    }

    /// 调用工具。
    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolCallResult, AppError> {
        let server = self
            .find_server(server_id)
            .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", server_id)))?;
        let mut registry = self.registry.lock().await;
        registry.call_tool(&server, tool_name, arguments).await
    }

    /// 断开指定 server 连接。
    pub async fn disconnect_server(&self, server_id: &str) {
        let mut registry = self.registry.lock().await;
        registry.disconnect(server_id);
    }
}

impl Drop for McpState {
    fn drop(&mut self) {
        // 尝试断开所有连接（best-effort，不阻塞）
        if let Ok(mut registry) = self.registry.try_lock() {
            registry.disconnect_all();
        }
    }
}
