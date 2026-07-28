//! ═══════════════════════════════════════════════════════════════════════════
//! 工具注册表 - 基于 async Tool trait 的统一工具注册中心
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计要点：
//! - 单例：lazy_static 持有全局 ToolRegistry
//! - 快照读：list_tools() 返回 Vec<Arc<dyn Tool>>，DashMap 分片读不加全局锁，
//!   返回的 Vec 是 owned 快照，调用方持有期间不阻塞其他读写
//! - check_fn 缓存：30s TTL，避免每次可用性检查都执行 check
//! - schema override：零参 callable 返回 parameters JSON Schema，优先于 tool.definition().parameters

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde_json::Value as JsonValue;

use crate::infrastructure::llm::tool::{Tool, ToolDefinition};

/// check_fn 结果缓存 TTL：30 秒。
const CHECK_FN_TTL: Duration = Duration::from_secs(30);

/// 可用性检查函数类型：返回工具当前是否可用。
pub type AvailabilityCheck = Arc<dyn Fn() -> bool + Send + Sync>;

/// 动态 schema override 函数类型：返回 parameters JSON Schema。
pub type SchemaOverride = Arc<dyn Fn() -> JsonValue + Send + Sync>;

/// 工具暴露级别：控制工具是否在 model-visible 工具列表中可见。
///
/// - `Visible`：正常注册并暴露给模型（默认）
/// - `Hidden`：仅注册用于 dispatch，不暴露给模型（如内部辅助工具、
///   动态发现工具的派发入口等）
///
/// 对照 codex-rs 的 `ToolExposure`（Direct/Deferred/DirectModelOnly/Hidden），
/// 本项目当前仅区分 Visible/Hidden 两档；未来如需 Deferred（动态发现）
/// 可在此枚举扩展。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum ToolExposure {
    /// 暴露给模型（默认）。
    #[default]
    Visible,
    /// 仅注册用于 dispatch，不进入 model-visible 工具列表。
    Hidden,
}

impl ToolExposure {
    /// 是否对模型可见。
    pub fn is_visible(self) -> bool {
        matches!(self, Self::Visible)
    }
}

/// 注册表条目：工具实例 + 可选可用性检查 + 可选 schema override + 暴露级别。
pub struct ToolEntry {
    pub tool: Arc<dyn Tool>,
    pub check_fn: Option<AvailabilityCheck>,
    pub schema_override: Option<SchemaOverride>,
    pub exposure: ToolExposure,
}

impl ToolEntry {
    /// 构造仅有工具实例的条目（无 check_fn、无 override、暴露级别默认 Visible）。
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self {
            tool,
            check_fn: None,
            schema_override: None,
            exposure: ToolExposure::Visible,
        }
    }

    /// 设置可用性检查函数。
    pub fn with_check_fn(mut self, check_fn: AvailabilityCheck) -> Self {
        self.check_fn = Some(check_fn);
        self
    }

    /// 设置动态 schema override。
    pub fn with_schema_override(mut self, schema_override: SchemaOverride) -> Self {
        self.schema_override = Some(schema_override);
        self
    }

    /// 设置工具暴露级别。
    pub fn with_exposure(mut self, exposure: ToolExposure) -> Self {
        self.exposure = exposure;
        self
    }
}

/// 工具注册中心（单例）。
///
/// 使用 DashMap 分片存储，读操作无需全局锁；list_tools() 返回快照（owned Vec），
/// 调用方持有快照期间不阻塞其他读写。
pub struct ToolRegistry {
    tools: DashMap<String, ToolEntry>,
    /// check_fn 结果缓存：name -> (available, computed_at)
    availability_cache: RwLock<HashMap<String, (bool, Instant)>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
            availability_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 注册工具条目。
    ///
    /// 若同名工具已存在则覆盖。注册时清除该工具的可用性缓存，确保下次检查使用新 check_fn。
    pub fn register(&self, entry: ToolEntry) {
        let name = entry.tool.name().to_string();
        tracing::debug!(tool_name = %name, "[tool-registry] register");
        self.tools.insert(name.clone(), entry);
        // 清除可用性缓存，避免旧 check_fn 结果残留
        if let Ok(mut cache) = self.availability_cache.write() {
            cache.remove(&name);
        }
    }

    /// 查询工具实例。
    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|e| e.tool.clone())
    }

    /// 返回所有已注册工具的快照（DashMap 分片读，返回 owned Vec）。
    pub fn list_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.iter().map(|e| e.tool.clone()).collect()
    }

    /// 返回所有对模型可见的工具快照（exposure == Visible）。
    ///
    /// 用于构建 model-visible tool spec 列表；Hidden 工具仍可通过 get_tool 派发，
    /// 但不会出现在给模型的工具列表中。
    pub fn list_visible_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools
            .iter()
            .filter(|e| e.exposure.is_visible())
            .map(|e| e.tool.clone())
            .collect()
    }

    /// 查询工具的暴露级别。
    pub fn tool_exposure(&self, name: &str) -> Option<ToolExposure> {
        self.tools.get(name).map(|e| e.exposure)
    }

    /// 返回所有工具名快照（含 Hidden）。
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|e| e.tool.name().to_string()).collect()
    }

    /// 查询工具是否可用（带 30s TTL 缓存）。
    ///
    /// 缓存逻辑：
    /// - 先查缓存，若存在且未过期（< 30s）则直接返回缓存值
    /// - 否则执行 check_fn（无 check_fn 视为可用）并更新缓存
    /// - 缺失工具返回 false，且不写入缓存（避免负缓存导致后续注册后仍判不可用）
    pub fn is_tool_available(&self, name: &str) -> bool {
        // 1. 查缓存（未过期则直接返回）
        if let Ok(cache) = self.availability_cache.read() {
            if let Some((available, computed_at)) = cache.get(name) {
                if computed_at.elapsed() < CHECK_FN_TTL {
                    return *available;
                }
            }
        }

        // 2. 缓存未命中或过期：先 clone 出 check_fn，立即释放 DashMap 分片读锁，
        //    避免 check_fn 重入 registry 写路径时跨分片锁死锁
        let check_fn = match self.tools.get(name) {
            Some(entry) => entry.check_fn.clone(),
            None => return false, // 工具不存在，不缓存
        };
        let available = match check_fn {
            Some(check) => check(),
            None => true,
        };

        // 3. 更新缓存
        if let Ok(mut cache) = self.availability_cache.write() {
            cache.insert(name.to_string(), (available, Instant::now()));
        }

        available
    }

    /// 获取工具定义（async）。
    ///
    /// 若该工具注册了 schema_override，则用 override 返回的 parameters JSON Schema
    /// 替换 tool.definition().parameters；否则直接返回 tool.definition()。
    ///
    /// name/description 仍取自 tool.definition()（静态字段），仅 parameters 为动态部分。
    ///
    /// 注意：先 clone 出 Arc 再 await，避免跨 .await 持有 DashMap 分片读锁（死锁风险 / future 非 Send）。
    pub async fn get_tool_definition(&self, name: &str) -> Option<ToolDefinition> {
        let (tool, schema_override) = self
            .tools
            .get(name)
            .map(|e| (e.tool.clone(), e.schema_override.clone()))?;
        let mut def = tool.definition().await;
        if let Some(override_fn) = &schema_override {
            def.parameters = override_fn();
        }
        Some(def)
    }

    /// 显式注册所有内置工具（占位实现）。
    ///
    /// 现有内置工具均需运行时上下文（workspace_root / approval / data_dir / engine），
    /// 无法在全局单例中静态注册（会导致 workspace 串扰）。实际工具注册将在后续迁移任务中
    /// 实现（届时 build_tools 将改为向 registry 注入 workspace 作用域工具）。
    ///
    /// 本任务仅提供 API 钩子，不迁移任何现有工具。
    pub fn init_default_tools(&self) {
        tracing::debug!("[tool-registry] init_default_tools: no-op (migration deferred)");
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── 全局单例与便捷 API ──────────────────────────────────────────

lazy_static::lazy_static! {
    static ref REGISTRY: ToolRegistry = ToolRegistry::new();
}

/// 获取全局 ToolRegistry 单例引用。
pub fn registry() -> &'static ToolRegistry {
    &REGISTRY
}

/// 全局注册工具（便捷函数）。
///
/// 构造无 check_fn、无 override 的 ToolEntry 并注册到全局单例。
/// 需要自定义 check_fn 或 schema_override 时，请直接调用
/// `registry().register(ToolEntry::new(tool).with_check_fn(...))`。
pub fn register_tool(tool: Arc<dyn Tool>) {
    registry().register(ToolEntry::new(tool));
}

// ── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::infrastructure::llm::tool::ToolError;

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
        async fn call(&self, _args: JsonValue) -> Result<JsonValue, ToolError> {
            Ok(JsonValue::Bool(true))
        }
    }

    fn make_mock(name: &str) -> Arc<dyn Tool> {
        Arc::new(MockTool {
            name: name.to_string(),
            definition: ToolDefinition {
                name: name.to_string(),
                description: format!("mock {} tool", name),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string" }
                    }
                }),
            },
        })
    }

    #[test]
    fn test_register_and_get() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("alpha")));
        assert!(registry.get_tool("alpha").is_some());
        assert!(registry.get_tool("missing").is_none());
    }

    #[test]
    fn test_list_tools_snapshot() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("a")));
        registry.register(ToolEntry::new(make_mock("b")));
        registry.register(ToolEntry::new(make_mock("c")));
        let snapshot = registry.list_tools();
        assert_eq!(snapshot.len(), 3);
        let names: Vec<String> = snapshot.iter().map(|t| t.name().to_string()).collect();
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
        assert!(names.contains(&"c".to_string()));
    }

    #[test]
    fn test_check_fn_ttl_cache() {
        let registry = ToolRegistry::new();
        let call_count = Arc::new(AtomicU32::new(0));
        let count_clone = call_count.clone();
        let check_fn: AvailabilityCheck = Arc::new(move || {
            count_clone.fetch_add(1, Ordering::SeqCst);
            true
        });
        registry.register(ToolEntry::new(make_mock("ttl_tool")).with_check_fn(check_fn));

        // 30s 内两次调用，check_fn 只应执行一次（缓存命中）
        let r1 = registry.is_tool_available("ttl_tool");
        let r2 = registry.is_tool_available("ttl_tool");
        assert!(r1 && r2);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_check_fn_recompute_after_reregister() {
        // 重新注册会清除缓存：check_fn 应重新执行
        let registry = ToolRegistry::new();
        let call_count = Arc::new(AtomicU32::new(0));

        let c1 = call_count.clone();
        registry.register(
            ToolEntry::new(make_mock("recompute_tool"))
                .with_check_fn(Arc::new(move || {
                    c1.fetch_add(1, Ordering::SeqCst);
                    true
                })),
        );
        registry.is_tool_available("recompute_tool");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // 重新注册触发缓存清除
        let c2 = call_count.clone();
        registry.register(
            ToolEntry::new(make_mock("recompute_tool"))
                .with_check_fn(Arc::new(move || {
                    c2.fetch_add(1, Ordering::SeqCst);
                    true
                })),
        );
        registry.is_tool_available("recompute_tool");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_no_check_fn_defaults_available() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("no_check")));
        assert!(registry.is_tool_available("no_check"));
        // 缺失工具视为不可用
        assert!(!registry.is_tool_available("does_not_exist"));
    }

    #[tokio::test]
    async fn test_schema_override_takes_precedence() {
        let registry = ToolRegistry::new();
        let override_params = serde_json::json!({
            "type": "object",
            "properties": {
                "dynamic": { "type": "number" }
            },
            "required": ["dynamic"]
        });
        let override_clone = override_params.clone();
        registry.register(
            ToolEntry::new(make_mock("override_tool"))
                .with_schema_override(Arc::new(move || override_clone.clone())),
        );
        let def = registry.get_tool_definition("override_tool").await.unwrap();
        assert_eq!(def.name, "override_tool");
        assert_eq!(def.parameters, override_params);
        // override 的 parameters 应不同于原始 mock definition 的 parameters
        assert!(def.parameters.get("properties").unwrap().get("dynamic").is_some());
    }

    #[tokio::test]
    async fn test_no_override_uses_tool_definition() {
        let registry = ToolRegistry::new();
        registry.register(ToolEntry::new(make_mock("plain_tool")));
        let def = registry.get_tool_definition("plain_tool").await.unwrap();
        assert_eq!(def.name, "plain_tool");
        assert!(def.parameters.get("properties").unwrap().get("input").is_some());
    }

    #[test]
    fn test_concurrent_register() {
        let registry = Arc::new(ToolRegistry::new());
        std::thread::scope(|s| {
            for i in 0..8 {
                let r = registry.clone();
                s.spawn(move || {
                    r.register(ToolEntry::new(make_mock(&format!("t{}", i))));
                });
            }
        });
        let snapshot = registry.list_tools();
        assert_eq!(snapshot.len(), 8);
        for i in 0..8 {
            assert!(registry.get_tool(&format!("t{}", i)).is_some());
        }
    }

    #[test]
    fn test_global_register_tool() {
        // 使用唯一名称避免与其他全局测试冲突
        let unique = "global_unique_test_tool_xyz";
        register_tool(make_mock(unique));
        assert!(registry().get_tool(unique).is_some());
        assert!(registry().list_tools().iter().any(|t| t.name() == unique));
    }
}
