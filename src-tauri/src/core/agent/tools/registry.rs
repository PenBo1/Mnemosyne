//! ═══════════════════════════════════════════════════════════════════════════
//! Registry - 中央工具注册中心
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::shared::error::AppError;

// ── ToolSchema: OpenAI 兼容的工具定义 ─────────────────────────────

/// 工具 JSON Schema 定义（OpenAI 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

impl ToolSchema {
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: JsonValue) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

// ── ToolMeta: 工具元数据 ─────────────────────────────────────────

/// 工具元数据（注册时提供）
#[derive(Clone)]
pub struct ToolMeta {
    pub name: String,
    pub toolset: String,
    pub schema: ToolSchema,
    pub handler: ToolHandler,
    pub check_fn: Option<AvailabilityCheck>,
    pub requires_env: Vec<String>,
    pub description: String,
    pub emoji: String,
    pub max_result_size_chars: Option<usize>,
}

impl std::fmt::Debug for ToolMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolMeta")
            .field("name", &self.name)
            .field("toolset", &self.toolset)
            .field("schema", &self.schema)
            .field("requires_env", &self.requires_env)
            .field("description", &self.description)
            .field("emoji", &self.emoji)
            .field("max_result_size_chars", &self.max_result_size_chars)
            .finish_non_exhaustive()
    }
}

/// 工具处理器（同步函数）
pub type ToolHandler = Arc<dyn Fn(serde_json::Value) -> Result<String, ToolError> + Send + Sync>;

/// 可用性检查函数
pub type AvailabilityCheck = Arc<dyn Fn() -> bool + Send + Sync>;

// ── ToolError: 工具错误类型 ──────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("工具执行失败: {0}")]
    ExecutionFailed(String),
    #[error("无效参数: {0}")]
    InvalidArguments(String),
    #[error("权限被拒绝")]
    PermissionDenied,
    #[error("工具不可用: {0}")]
    NotAvailable(String),
}

impl From<ToolError> for AppError {
    fn from(e: ToolError) -> Self {
        AppError::internal(e.to_string())
    }
}

// ── ToolEntry: 注册表条目 ────────────────────────────────────────

/// 注册表中的工具条目
#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub meta: ToolMeta,
}

impl ToolEntry {
    pub fn new(meta: ToolMeta) -> Self {
        Self { meta }
    }

    pub fn is_available(&self) -> bool {
        if let Some(check) = &self.meta.check_fn {
            check()
        } else {
            true
        }
    }
}

// ── Registry: 全局注册中心 ──────────────────────────────────────

lazy_static::lazy_static! {
    static ref REGISTRY: RwLock<Registry> = RwLock::new(Registry::new());
}

/// 工具注册中心
#[derive(Debug, Default)]
pub struct Registry {
    tools: HashMap<String, ToolEntry>,
    toolsets: HashMap<String, Vec<String>>,
}

impl Registry {
    fn new() -> Self {
        Self {
            tools: HashMap::new(),
            toolsets: HashMap::new(),
        }
    }

    pub fn register(&mut self, meta: ToolMeta) {
        let name = meta.name.clone();
        let toolset = meta.toolset.clone();

        self.tools.insert(name.clone(), ToolEntry::new(meta));

        self.toolsets
            .entry(toolset)
            .or_default()
            .push(name);
    }

    pub fn get(&self, name: &str) -> Option<&ToolEntry> {
        self.tools.get(name)
    }

    pub fn get_handler(&self, name: &str) -> Option<ToolHandler> {
        self.tools.get(name).map(|e| e.meta.handler.clone())
    }

    pub fn get_schema(&self, name: &str) -> Option<ToolSchema> {
        self.tools.get(name).map(|e| e.meta.schema.clone())
    }

    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn list_tools_in_toolset(&self, toolset: &str) -> Vec<&str> {
        self.toolsets
            .get(toolset)
            .map(|names| names.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn get_tool_schemas(&self, enabled_toolsets: &[&str]) -> Vec<ToolSchema> {
        let mut schemas = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for toolset in enabled_toolsets {
            if let Some(tool_names) = self.toolsets.get(*toolset) {
                for name in tool_names {
                    if seen.insert(name.clone()) {
                        if let Some(entry) = self.tools.get(name) {
                            if entry.is_available() {
                                schemas.push(entry.meta.schema.clone());
                            }
                        }
                    }
                }
            }
        }

        schemas
    }

    pub fn check_availability(&self, name: &str) -> Result<bool, String> {
        match self.tools.get(name) {
            Some(entry) => {
                if entry.is_available() {
                    Ok(true)
                } else {
                    Err(format!(
                        "Tool '{}' requires: {}",
                        name,
                        entry.meta.requires_env.join(", ")
                    ))
                }
            }
            None => Err(format!("Tool '{}' not found", name)),
        }
    }

    pub fn execute(&self, name: &str, args: JsonValue) -> Result<String, ToolError> {
        match self.tools.get(name) {
            Some(entry) => {
                if !entry.is_available() {
                    return Err(ToolError::NotAvailable(format!(
                        "{} requires: {}",
                        name,
                        entry.meta.requires_env.join(", ")
                    )));
                }
                (entry.meta.handler)(args)
            }
            None => Err(ToolError::NotAvailable(format!("Tool '{}' not found", name))),
        }
    }
}

// ── 公共 API ─────────────────────────────────────────────────────

pub fn register(meta: ToolMeta) {
    tracing::info!(
        tool_name = %meta.name,
        toolset = %meta.toolset,
        "[registry] register: registering tool"
    );
    let mut registry = REGISTRY.write().unwrap();
    registry.register(meta);
    tracing::debug!("[registry] register: tool registered successfully");
}

pub fn get(name: &str) -> Option<ToolEntry> {
    let registry = REGISTRY.read().unwrap();
    registry.get(name).cloned()
}

pub fn get_handler(name: &str) -> Option<ToolHandler> {
    let registry = REGISTRY.read().unwrap();
    registry.get_handler(name)
}

pub fn get_schema(name: &str) -> Option<ToolSchema> {
    let registry = REGISTRY.read().unwrap();
    registry.get_schema(name)
}

pub fn list_tools() -> Vec<String> {
    let registry = REGISTRY.read().unwrap();
    registry.list_tools().into_iter().map(|s| s.to_string()).collect()
}

pub fn get_tool_schemas(enabled_toolsets: &[&str]) -> Vec<ToolSchema> {
    let registry = REGISTRY.read().unwrap();
    registry.get_tool_schemas(enabled_toolsets)
}

pub fn check_availability(name: &str) -> Result<bool, String> {
    let registry = REGISTRY.read().unwrap();
    registry.check_availability(name)
}

pub fn execute(name: &str, args: JsonValue) -> Result<String, ToolError> {
    let registry = REGISTRY.read().unwrap();
    registry.execute(name, args)
}

pub fn toolset_for_tool(name: &str) -> Option<String> {
    let registry = REGISTRY.read().unwrap();
    registry.tools.get(name).map(|e| e.meta.toolset.clone())
}

pub fn list_toolsets() -> Vec<String> {
    let registry = REGISTRY.read().unwrap();
    registry.toolsets.keys().cloned().collect()
}

// ── 内置工具集定义 ──────────────────────────────────────────────

pub const HERMES_CORE_TOOLS: &[&str] = &[
    "file",
    "terminal",
    "search",
    "research",
    "memory",
    "todo",
    "book",
    "mcp",
];

pub const HERMES_OPTIONAL_TOOLSETS: &[&str] = &[
    "browser",
    "vision",
    "image_gen",
    "tts",
    "video",
    "discord",
    "kanban",
    "delegation",
];

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let schema = ToolSchema::new(
            "test_tool",
            "A test tool",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
        );

        let meta = ToolMeta {
            name: "test_tool".to_string(),
            toolset: "test".to_string(),
            schema,
            handler: Arc::new(|_args| Ok("test result".to_string())),
            check_fn: None,
            requires_env: vec![],
            description: "A test tool".to_string(),
            emoji: "🔧".to_string(),
            max_result_size_chars: None,
        };

        register(meta);

        let result = execute("test_tool", serde_json::json!({"input": "hello"}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test result");
    }
}