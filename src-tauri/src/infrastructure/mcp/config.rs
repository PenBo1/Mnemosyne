//! ═══════════════════════════════════════════════════════════════════════════
//! MCP 配置 - 配置管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! McpConfig 持久化到 <data_dir>/mcp_config.json。
//!
//! 设计参考：infrastructure/tool_limits/config.rs
//! - 文件不存在 → 返回 Default（空 server 列表）
//! - 文件存在但解析失败 → 返回 Err（不静默回退）
//! - 保存前校验 id 唯一性

use serde::{Deserialize, Serialize};

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::mcp::types::McpServerConfig;
use crate::shared::error::AppError;

const CONFIG_FILENAME: &str = "mcp_config.json";

/// MCP 配置（server 列表）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

impl McpConfig {
    /// 从 <data_dir>/mcp_config.json 加载。
    pub fn load(data_dir: &DataDir) -> Result<Self, AppError> {
        let path = data_dir.root().join(CONFIG_FILENAME);
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            AppError::internal(format!("Failed to read {}: {}", CONFIG_FILENAME, e))
        })?;
        let config: Self = serde_json::from_str(&content).map_err(|e| {
            AppError::internal(format!("Failed to parse {}: {}", CONFIG_FILENAME, e))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// 保存到 <data_dir>/mcp_config.json（pretty-printed）。
    pub fn save(&self, data_dir: &DataDir) -> Result<(), AppError> {
        self.validate()?;
        let path = data_dir.root().join(CONFIG_FILENAME);
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            AppError::internal(format!("Failed to serialize {}: {}", CONFIG_FILENAME, e))
        })?;
        std::fs::write(&path, content).map_err(|e| {
            AppError::internal(format!("Failed to write {}: {}", CONFIG_FILENAME, e))
        })?;
        tracing::info!(path = %path.display(), "mcp_config.json saved");
        Ok(())
    }

    /// 校验：id 唯一性、name 非空。
    pub fn validate(&self) -> Result<(), AppError> {
        let mut seen = std::collections::HashSet::new();
        for s in &self.servers {
            if s.id.is_empty() {
                return Err(AppError::bad_request("MCP server id must not be empty"));
            }
            if s.name.is_empty() {
                return Err(AppError::bad_request("MCP server name must not be empty"));
            }
            if !seen.insert(&s.id) {
                return Err(AppError::duplicate(format!("Duplicate MCP server id: {}", s.id)));
            }
        }
        Ok(())
    }

    /// 按 id 查找。
    pub fn find(&self, id: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|s| s.id == id)
    }

    /// 按 id 查找（可变）。
    pub fn find_mut(&mut self, id: &str) -> Option<&mut McpServerConfig> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    /// 添加 server（id 重复时返回 Err）。
    pub fn add(&mut self, server: McpServerConfig) -> Result<(), AppError> {
        if self.find(&server.id).is_some() {
            return Err(AppError::duplicate(format!("MCP server already exists: {}", server.id)));
        }
        self.servers.push(server);
        Ok(())
    }

    /// 更新 server（不存在时返回 Err）。
    pub fn update(&mut self, id: &str, server: McpServerConfig) -> Result<(), AppError> {
        let idx = self.servers.iter().position(|s| s.id == id)
            .ok_or_else(|| AppError::not_found(format!("MCP server not found: {}", id)))?;
        // 新 id 若与其它 server 冲突则拒绝
        if server.id != id && self.servers.iter().enumerate()
            .any(|(i, s)| i != idx && s.id == server.id)
        {
            return Err(AppError::duplicate(format!("MCP server id already in use: {}", server.id)));
        }
        self.servers[idx] = server;
        Ok(())
    }

    /// 移除 server，返回被移除的配置。
    pub fn remove(&mut self, id: &str) -> Option<McpServerConfig> {
        self.servers.iter().position(|s| s.id == id)
            .map(|idx| self.servers.remove(idx))
    }

    /// 列出所有 server（克隆）。
    pub fn list(&self) -> Vec<McpServerConfig> {
        self.servers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::mcp::types::McpTransport;

    fn make_server(id: &str, name: &str) -> McpServerConfig {
        McpServerConfig {
            id: id.to_string(),
            name: name.to_string(),
            transport: McpTransport::Stdio {
                command: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
            },
            enabled: true,
            auto_start: false,
        }
    }

    #[test]
    fn default_is_empty() {
        let c = McpConfig::default();
        assert!(c.servers.is_empty());
    }

    #[test]
    fn add_and_find() {
        let mut c = McpConfig::default();
        c.add(make_server("s1", "Server 1")).unwrap();
        assert_eq!(c.servers.len(), 1);
        assert!(c.find("s1").is_some());
        assert!(c.find("nope").is_none());
    }

    #[test]
    fn add_duplicate_rejected() {
        let mut c = McpConfig::default();
        c.add(make_server("s1", "A")).unwrap();
        assert!(c.add(make_server("s1", "B")).is_err());
    }

    #[test]
    fn update_existing() {
        let mut c = McpConfig::default();
        c.add(make_server("s1", "A")).unwrap();
        let mut s = make_server("s1", "B");
        s.name = "Updated".to_string();
        c.update("s1", s).unwrap();
        assert_eq!(c.find("s1").unwrap().name, "Updated");
    }

    #[test]
    fn update_missing_rejected() {
        let mut c = McpConfig::default();
        assert!(c.update("nope", make_server("nope", "A")).is_err());
    }

    #[test]
    fn remove_returns_config() {
        let mut c = McpConfig::default();
        c.add(make_server("s1", "A")).unwrap();
        let removed = c.remove("s1");
        assert!(removed.is_some());
        assert!(c.servers.is_empty());
        assert!(c.remove("s1").is_none());
    }

    #[test]
    fn validate_rejects_empty_id() {
        let mut c = McpConfig::default();
        let mut s = make_server("x", "A");
        s.id = "".to_string();
        c.servers.push(s);
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_duplicate_id() {
        let mut c = McpConfig::default();
        c.servers.push(make_server("s1", "A"));
        c.servers.push(make_server("s1", "B"));
        assert!(c.validate().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut c = McpConfig::default();
        c.add(make_server("s1", "Server 1")).unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: McpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].id, "s1");
    }
}
