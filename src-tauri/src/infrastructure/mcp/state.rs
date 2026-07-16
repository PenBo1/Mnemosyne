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
    McpServerConfig, McpServerTestResult, McpTool, McpToolCallResult, McpTransport,
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
        // 区分"文件不存在（用 default）"与"文件存在但解析失败（启动失败/告警）"。
        // 解析失败时记录 warn 但仍用 default，避免单点错误阻塞整个应用启动；
        // 配置损坏的修复由用户在 UI 中通过 mcp_update_server 完成。
        let config = match McpConfig::load(&data_dir) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to load mcp_config.json (using default). \
                     If the file is corrupt, fix or delete it manually."
                );
                McpConfig::default()
            }
        };
        Self {
            config: Arc::new(RwLock::new(config)),
            registry: Arc::new(Mutex::new(McpRegistry::new())),
            data_dir,
        }
    }

    /// 列出所有 server 配置。
    ///
    /// 锁毒化时返回错误而非静默回退空列表（"no silent fallback"）。
    pub fn list_servers(&self) -> Result<Vec<McpServerConfig>, AppError> {
        self.config
            .read()
            .map_err(|e| AppError::internal(format!("Config lock poisoned: {}", e)))
            .map(|c| c.list())
    }

    /// 添加 server 配置并持久化。
    ///
    /// 安全约束（AGENTS.md Security Kernel 模型）：
    /// MCP stdio 传输会启动任意子进程。由于 `ShellScope` 当前不支持
    /// 任意外部进程变体，无法走 SecurityKernel.execute_async(Shell) 审批。
    /// 因此在 add_server 入口拒绝高危险 shell 命令作为 stdio command。
    /// 调用方仍需通过审批 UI 显式确认 MCP server 配置。
    pub fn add_server(&self, server: McpServerConfig) -> Result<(), AppError> {
        validate_transport_safety(&server.transport)?;
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
        validate_transport_safety(&server.transport)?;
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
    ///
    /// 锁毒化时返回错误而非静默返回 None（"no silent fallback"）。
    pub fn find_server(&self, id: &str) -> Result<Option<McpServerConfig>, AppError> {
        let guard = self
            .config
            .read()
            .map_err(|e| AppError::internal(format!("Config lock poisoned: {}", e)))?;
        Ok(guard.find(id).cloned())
    }

    /// 测试 server 连接（ensure_connected + list_tools）。
    ///
    /// 连接成功后保留在 registry 中供后续调用。
    pub async fn test_server(&self, id: &str) -> Result<McpServerTestResult, AppError> {
        let server = self
            .find_server(id)?
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
                    .find_server(id)?
                    .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", id)))?;
                registry.list_tools(&server).await
            }
            None => {
                let servers = self.list_servers()?;
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
            .find_server(server_id)?
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

// ── 安全约束 ──────────────────────────────────────────────────

/// 禁止作为 MCP stdio command 的可执行文件名黑名单。
///
/// 这些命令自身可执行任意代码（如 `sh -c "..."`），等同于让 MCP server
/// 成为通用 RCE 通道。要求用户配置实际的 MCP server 二进制路径（如
/// `node /path/to/server.js`、`python -m foo`），而非把 shell 当 command。
const FORBIDDEN_COMMAND_BASENAMES: &[&str] = &[
    // Unix shells
    "sh", "bash", "zsh", "fish", "dash", "ksh", "csh", "tcsh",
    // Windows shells
    "cmd.exe", "cmd", "powershell.exe", "powershell", "pwsh.exe", "pwsh",
    // Other interpreters that accept arbitrary code as args
    "python", "python3", "python2",
    "node", "node.exe",
    "ruby", "perl", "php",
];

/// 校验 MCP transport 的安全性。
///
/// 当前规则：禁止 stdio 命令直接以 shell / 任意代码解释器作为 command。
/// 这不是完整 SecurityKernel 审批（参见 add_server 注释），仅作为
/// "防止把 shell 作为 MCP server"的最小防御层。
fn validate_transport_safety(transport: &McpTransport) -> Result<(), AppError> {
    if let McpTransport::Stdio { command, .. } = transport {
        // 取 basename 做比较（去掉路径前缀，统一大小写）
        let basename = std::path::Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command)
            .to_lowercase();
        if FORBIDDEN_COMMAND_BASENAMES.iter().any(|&forbidden| basename == forbidden) {
            return Err(AppError::invalid_input(format!(
                "MCP stdio command '{}' is forbidden: shell or interpreter as command \
                 enables arbitrary code execution. Please specify the MCP server binary \
                 directly (e.g. 'node /path/to/server.js').",
                command
            )));
        }
    }
    Ok(())
}
