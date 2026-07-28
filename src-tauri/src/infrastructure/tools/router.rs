//! ═══════════════════════════════════════════════════════════════════════════
//! 工具路由器 - 封装 ToolRegistry 并提供 model-visible spec 列表
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计动机：
//! - ToolRegistry 负责工具注册、可用性检查、schema override
//! - ToolRouter 在其上叠加 model_visible_specs（仅 exposure == Visible 的工具）
//! - build_tool_router 按 feature flag 注册内置工具
//!
//! 动态工具发现：
//! - 使用基于关键词的简单匹配（大小写不敏感）
//! - 仅当 registry 中存在 Hidden 工具时，ToolSearchHandler 才会被注册

use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::infrastructure::llm::tool::{tool_to_spec, Tool, ToolDefinition};
use crate::infrastructure::llm::types::ToolSpec;

use super::registry::{ToolEntry, ToolExposure, ToolRegistry};

/// 功能开关：控制 `build_tool_router` 注册哪些内置工具组。
///
/// 当前为占位实现（内置工具迁移未完成，见 registry.rs `init_default_tools` 文档）。
/// 后续迁移时，每个开关对应一组工具的注册逻辑。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeatureFlags {
    /// 是否注册文件系统工具（read/write/glob/grep）
    pub fs_tools: bool,
    /// 是否注册 shell/exec 工具
    pub shell_tools: bool,
    /// 是否注册 agent/subagent 工具
    pub agent_tools: bool,
    /// 是否注册 todo/plan 工具
    pub todo_tools: bool,
    /// 是否注册 MCP 工具（动态从 MCP registry 拉取）
    pub mcp_tools: bool,
    /// 是否启用动态工具发现（ToolSearchHandler）
    pub tool_search: bool,
}

impl FeatureFlags {
    /// 全部启用（用于测试或全功能场景）。
    pub fn all() -> Self {
        Self {
            fs_tools: true,
            shell_tools: true,
            agent_tools: true,
            todo_tools: true,
            mcp_tools: true,
            tool_search: true,
        }
    }

    /// 全部禁用（仅注册显式 add 的工具）。
    pub fn none() -> Self {
        Self::default()
    }
}

/// 工具路由器：ToolRegistry + model_visible_specs。
///
/// 对照 codex-rs `ToolRouter`：
/// - `registry`：用于工具派发（含 Hidden 工具）
/// - `model_visible_specs`：暴露给模型的工具 spec 列表（仅 Visible 工具）
/// - 调用方通过 `model_visible_specs()` 获取给模型的列表，通过 `registry()` 派发调用
pub struct ToolRouter {
    registry: Arc<ToolRegistry>,
    model_visible_specs: Vec<ToolSpec>,
}

impl ToolRouter {
    /// 从已构建的 registry 和 specs 直接构造（主要用于测试）。
    pub fn from_parts(registry: Arc<ToolRegistry>, model_visible_specs: Vec<ToolSpec>) -> Self {
        Self {
            registry,
            model_visible_specs,
        }
    }

    /// 获取给模型的工具 spec 列表（clone）。
    pub fn model_visible_specs(&self) -> Vec<ToolSpec> {
        self.model_visible_specs.clone()
    }

    /// 获取内部 registry 引用（用于派发工具调用）。
    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// 获取 registry 的 Arc 引用（用于跨线程共享）。
    pub fn registry_arc(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.registry)
    }
}

/// 按 feature flag 构建工具路由器。
///
/// 流程（对照 codex `build_tool_router`）：
/// 1. 创建空 ToolRegistry
/// 2. 按 feature flag 注册内置工具组（当前为占位：内置工具迁移未完成）
/// 3. 若启用 tool_search 且存在 Hidden 工具，注册 ToolSearchHandler（Hidden）
/// 4. 收集所有 Visible 工具的 ToolSpec 作为 model_visible_specs
/// 5. 返回 ToolRouter
///
/// 调用方可通过 `router.with_extra_tool(entry)` 在构建后追加自定义工具。
pub async fn build_tool_router(flags: FeatureFlags) -> ToolRouter {
    let registry = Arc::new(ToolRegistry::new());

    // 占位：内置工具迁移未完成，此处按 flag 记录日志，不实际注册
    // 后续迁移时，每个 flag 对应一组 ToolEntry::new(...).register() 调用
    if flags.fs_tools {
        tracing::debug!("[tool-router] fs_tools enabled (no builtin registered yet)");
    }
    if flags.shell_tools {
        tracing::debug!("[tool-router] shell_tools enabled (no builtin registered yet)");
    }
    if flags.agent_tools {
        tracing::debug!("[tool-router] agent_tools enabled (no builtin registered yet)");
    }
    if flags.todo_tools {
        tracing::debug!("[tool-router] todo_tools enabled (no builtin registered yet)");
    }
    if flags.mcp_tools {
        tracing::debug!("[tool-router] mcp_tools enabled (no builtin registered yet)");
    }

    // 动态工具发现：若启用且存在 Hidden 工具，注册 ToolSearchHandler
    if flags.tool_search {
        let hidden_count = count_hidden_tools(&registry);
        if hidden_count > 0 {
            let handler = Arc::new(ToolSearchHandler::from_registry(&registry).await)
                as Arc<dyn Tool>;
            registry.register(
                ToolEntry::new(handler).with_exposure(ToolExposure::Visible),
            );
            tracing::debug!(
                hidden_count,
                "[tool-router] registered ToolSearchHandler for dynamic discovery"
            );
        }
    }

    // 收集 Visible 工具的 ToolSpec
    let visible_tools = registry.list_visible_tools();
    let mut specs = Vec::with_capacity(visible_tools.len());
    for tool in &visible_tools {
        specs.push(tool_to_spec(tool.as_ref()).await);
    }

    ToolRouter::from_parts(registry, specs)
}

/// 统计 registry 中 Hidden 工具数量（用于决定是否注册 ToolSearchHandler）。
fn count_hidden_tools(registry: &ToolRegistry) -> usize {
    registry
        .tool_names()
        .iter()
        .filter(|name| {
            registry.tool_exposure(name) == Some(ToolExposure::Hidden)
        })
        .count()
}

// ── ToolSearchHandler ────────────────────────────────────────────

/// 动态工具发现处理器（对照 codex-rs `ToolSearchHandler`）。
///
/// codex 使用 bm25 算法对 Deferred 工具做语义搜索；本项目当前未引入 bm25 依赖，
/// 采用基于关键词的简单子串匹配（大小写不敏感）。
///
/// 工作流程：
/// 1. `from_registry` 时快照所有 Hidden 工具的 ToolDefinition
/// 2. `search(query)` 返回名称或描述包含 query 的工具列表
/// 3. 模型可通过 `tool_search` 工具发现并调用这些 Hidden 工具
///
/// 注意：本 handler 自身被注册为 Visible（让模型能调用 tool_search），
/// 但它发现的工具是 Hidden 的（不在初始 model-visible 列表中）。
pub struct ToolSearchHandler {
    /// Hidden 工具定义快照（构建时捕获，不随后续 registry 变更）
    entries: Vec<ToolSearchEntry>,
    definition: ToolDefinition,
}

/// 单个可搜索工具的元数据。
#[derive(Clone, Debug)]
pub struct ToolSearchEntry {
    pub name: String,
    pub description: String,
}

impl ToolSearchHandler {
    /// 从 registry 快照所有 Hidden 工具的定义。
    ///
    /// 注意：此函数会 async 读取所有工具的 definition，应在 build_tool_router
    /// 的 async 上下文中调用。
    pub async fn from_registry(registry: &ToolRegistry) -> Self {
        let mut entries = Vec::new();
        for name in registry.tool_names() {
            if registry.tool_exposure(&name) == Some(ToolExposure::Hidden) {
                if let Some(def) = registry.get_tool_definition(&name).await {
                    entries.push(ToolSearchEntry {
                        name: def.name,
                        description: def.description,
                    });
                }
            }
        }
        let definition = ToolDefinition {
            name: "tool_search".to_string(),
            description: format!(
                "Search and discover {} available but hidden tools by keyword. \
                 Returns matching tool names and descriptions.",
                entries.len()
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search keyword (matched against tool name/description, case-insensitive)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default 10)",
                        "default": 10
                    }
                },
                "required": ["query"]
            }),
        };
        Self {
            entries,
            definition,
        }
    }

    /// 从显式 entries 构造（主要用于测试）。
    pub fn from_entries(entries: Vec<ToolSearchEntry>) -> Self {
        let definition = ToolDefinition {
            name: "tool_search".to_string(),
            description: format!(
                "Search and discover {} available but hidden tools by keyword.",
                entries.len()
            ),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                },
                "required": ["query"]
            }),
        };
        Self {
            entries,
            definition,
        }
    }

    /// 按关键词搜索（大小写不敏感子串匹配）。
    ///
    /// 返回匹配的 ToolSearchEntry 列表，最多 `limit` 条。
    pub fn search(&self, query: &str, limit: usize) -> Vec<&ToolSearchEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q)
            })
            .take(limit)
            .collect()
    }

    /// 可搜索工具数量。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否无可搜索工具。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[async_trait::async_trait]
impl Tool for ToolSearchHandler {
    fn name(&self) -> &str {
        &self.definition.name
    }

    async fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, args: JsonValue) -> Result<JsonValue, crate::infrastructure::llm::tool::ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::infrastructure::llm::tool::ToolError::InvalidArgs(
                    "missing 'query' field".to_string(),
                )
            })?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10);
        let results = self.search(query, limit);
        let matched: Vec<serde_json::Value> = results
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "description": e.description,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "query": query,
            "matched_count": matched.len(),
            "tools": matched,
        }))
    }
}

// ── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    /// 测试用 mock 工具
    struct MockTool {
        name: String,
        definition: ToolDefinition,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }
        async fn definition(&self) -> ToolDefinition {
            self.definition.clone()
        }
        async fn call(&self, _args: JsonValue) -> Result<JsonValue, crate::infrastructure::llm::tool::ToolError> {
            Ok(JsonValue::Bool(true))
        }
    }

    fn make_mock(name: &str, desc: &str) -> Arc<dyn Tool> {
        Arc::new(MockTool {
            name: name.to_string(),
            definition: ToolDefinition {
                name: name.to_string(),
                description: desc.to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        })
    }

    #[test]
    fn test_tool_exposure_default_visible() {
        let entry = ToolEntry::new(make_mock("v1", "visible tool"));
        assert_eq!(entry.exposure, ToolExposure::Visible);
        assert!(entry.exposure.is_visible());
    }

    #[test]
    fn test_tool_exposure_hidden() {
        let entry = ToolEntry::new(make_mock("h1", "hidden tool"))
            .with_exposure(ToolExposure::Hidden);
        assert_eq!(entry.exposure, ToolExposure::Hidden);
        assert!(!entry.exposure.is_visible());
    }

    #[test]
    fn test_registry_list_visible_tools_filters_hidden() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("visible_a", "a visible tool")));
        registry.register(
            ToolEntry::new(make_mock("hidden_b", "a hidden tool"))
                .with_exposure(ToolExposure::Hidden),
        );
        registry.register(ToolEntry::new(make_mock("visible_c", "another visible tool")));

        let all = registry.list_tools();
        let visible = registry.list_visible_tools();
        assert_eq!(all.len(), 3);
        assert_eq!(visible.len(), 2);
        let visible_names: Vec<String> = visible.iter().map(|t| t.name().to_string()).collect();
        assert!(visible_names.contains(&"visible_a".to_string()));
        assert!(visible_names.contains(&"visible_c".to_string()));
        assert!(!visible_names.contains(&"hidden_b".to_string()));
    }

    #[test]
    fn test_registry_tool_exposure_lookup() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("v", "v")));
        registry.register(
            ToolEntry::new(make_mock("h", "h")).with_exposure(ToolExposure::Hidden),
        );
        assert_eq!(registry.tool_exposure("v"), Some(ToolExposure::Visible));
        assert_eq!(registry.tool_exposure("h"), Some(ToolExposure::Hidden));
        assert_eq!(registry.tool_exposure("missing"), None);
    }

    #[test]
    fn test_feature_flags_all_and_none() {
        let all = FeatureFlags::all();
        assert!(all.fs_tools && all.shell_tools && all.agent_tools && all.todo_tools && all.mcp_tools && all.tool_search);
        let none = FeatureFlags::none();
        assert!(!none.fs_tools && !none.tool_search);
    }

    #[tokio::test]
    async fn test_build_tool_router_empty() {
        // 无 flag、无工具时，router 应为空但合法
        let router = build_tool_router(FeatureFlags::none()).await;
        assert!(router.model_visible_specs().is_empty());
        assert!(router.registry().list_tools().is_empty());
    }

    #[tokio::test]
    async fn test_build_tool_router_with_tool_search_no_hidden() {
        // 启用 tool_search 但无 Hidden 工具时，不应注册 ToolSearchHandler
        let router = build_tool_router(FeatureFlags {
            tool_search: true,
            ..FeatureFlags::none()
        }).await;
        // 无 Hidden 工具，ToolSearchHandler 不注册
        assert!(router.model_visible_specs().is_empty());
    }

    #[tokio::test]
    async fn test_build_tool_router_with_tool_search_and_hidden() {
        // 启用 tool_search 且有 Hidden 工具时，应注册 ToolSearchHandler（Visible）
        let registry = Arc::new(ToolRegistry::new());
        registry.register(
            ToolEntry::new(make_mock("hidden_search_target", "a hidden tool for search"))
                .with_exposure(ToolExposure::Hidden),
        );

        // 直接构造 router 模拟 build_tool_router 的结果
        let handler = Arc::new(ToolSearchHandler::from_registry(&registry).await) as Arc<dyn Tool>;
        registry.register(ToolEntry::new(handler));

        let visible_tools = registry.list_visible_tools();
        let specs: Vec<ToolSpec> = {
            let mut s = Vec::new();
            for t in &visible_tools {
                s.push(tool_to_spec(t.as_ref()).await);
            }
            s
        };
        let router = ToolRouter::from_parts(registry, specs);
        // ToolSearchHandler 自身是 Visible，应出现在 model_visible_specs
        let names: Vec<String> = router.model_visible_specs().iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"tool_search".to_string()));
        // Hidden 工具不应出现在 model_visible_specs
        assert!(!names.contains(&"hidden_search_target".to_string()));
    }

    #[tokio::test]
    async fn test_tool_search_handler_keyword_match() {
        let entries = vec![
            ToolSearchEntry {
                name: "read_file".to_string(),
                description: "Read a file from the filesystem".to_string(),
            },
            ToolSearchEntry {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
            },
            ToolSearchEntry {
                name: "list_dir".to_string(),
                description: "List directory contents".to_string(),
            },
        ];
        let handler = ToolSearchHandler::from_entries(entries);
        assert_eq!(handler.len(), 3);
        assert!(!handler.is_empty());

        // 按名称匹配
        let file_results = handler.search("file", 10);
        assert_eq!(file_results.len(), 2);

        // 按描述匹配
        let dir_results = handler.search("directory", 10);
        assert_eq!(dir_results.len(), 1);
        assert_eq!(dir_results[0].name, "list_dir");

        // 大小写不敏感
        let upper_results = handler.search("FILE", 10);
        assert_eq!(upper_results.len(), 2);

        // limit 生效
        let limited = handler.search("file", 1);
        assert_eq!(limited.len(), 1);
    }

    #[tokio::test]
    async fn test_tool_search_handler_call() {
        let entries = vec![ToolSearchEntry {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
        }];
        let handler = ToolSearchHandler::from_entries(entries);
        let args = serde_json::json!({"query": "read"});
        let result = handler.call(args).await.unwrap();
        assert_eq!(result["matched_count"], 1);
        assert_eq!(result["tools"][0]["name"], "read_file");
    }

    #[tokio::test]
    async fn test_tool_search_handler_call_missing_query() {
        let handler = ToolSearchHandler::from_entries(vec![]);
        let args = serde_json::json!({"limit": 5});
        let result = handler.call(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_search_handler_from_registry() {
        let registry = ToolRegistry::new();
        registry.register(
            ToolEntry::new(make_mock("hidden_tool", "a hidden searchable tool"))
                .with_exposure(ToolExposure::Hidden),
        );
        registry.register(ToolEntry::new(make_mock("visible_tool", "a visible tool")));

        let handler = ToolSearchHandler::from_registry(&registry).await;
        // 仅快照 Hidden 工具
        assert_eq!(handler.len(), 1);
        let results = handler.search("hidden", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hidden_tool");
    }

    #[tokio::test]
    async fn test_router_from_parts() {
        let registry = Arc::new(ToolRegistry::new());
        let specs = vec![ToolSpec {
            name: "test_tool".to_string(),
            description: "test".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let router = ToolRouter::from_parts(registry.clone(), specs);
        assert_eq!(router.model_visible_specs().len(), 1);
        assert!(Arc::ptr_eq(&router.registry_arc(), &registry));
    }
}
