//! ═══════════════════════════════════════════════════════════════════════════
//! Provider - 可插拔记忆系统抽象层
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::infrastructure::llm::types::Message;
use crate::shared::error::AppError;

// ── ToolSchema ──────────────────────────────────────────────────────────────

/// 记忆工具的 JSON Schema 定义（OpenAI 兼容格式）
///
/// 独立于 `core::agent::tools::ToolSchema`，作为 MemoryProvider trait 的边界类型，
/// 使外部 provider 实现无需依赖完整工具注册中心。AgentEngine 在合并工具集时负责
/// 将其转换为内部 ToolSchema。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: serde_json::Value,
}

impl ToolSchema {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

// ── MemoryProvider Trait ───────────────────────────────────────────────────

/// 可插拔记忆系统 Provider 抽象
///
/// builtin provider（如基于 SQLite 的本地记忆）与 external provider（如向量库、
/// 图谱记忆）实现同一 trait，由 `MemoryManager` 统一编排。
///
/// 方法分类（供 MemoryManager 决定串行/并行策略）：
/// - 读操作：`system_prompt_block` / `get_tool_schemas` / `handle_tool_call`
/// - 写操作：`initialize` / `prefetch` / `sync_turn` / `shutdown` / session hooks
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// Provider 名称（用于 memory context block 标注与日志）
    fn name(&self) -> &str;

    /// 是否为 builtin provider（默认 false，builtin 实现覆写为 true）
    fn is_builtin(&self) -> bool {
        false
    }

    /// 初始化（如打开索引、加载配置）。`workspace_root` 可用于限定记忆作用域。
    async fn initialize(&self, workspace_root: Option<&Path>) -> Result<(), AppError>;

    /// 返回注入 system prompt 的记忆上下文块（纯文本）。可为空字符串。
    async fn system_prompt_block(&self) -> Result<String, AppError>;

    /// 用户消息到达前的预取（缓存预热 / 检索）。写操作，由 manager 串行化。
    async fn prefetch(&self, session_id: &str, user_message: &str) -> Result<(), AppError>;

    /// 同步一轮对话（写入 / 更新记忆）。写操作，由 manager 串行化。
    async fn sync_turn(&self, session_id: &str, messages: &[Message]) -> Result<(), AppError>;

    /// 暴露给 Agent 的记忆工具 schema 列表（如 memory_search / memory_save）。
    async fn get_tool_schemas(&self) -> Vec<ToolSchema>;

    /// 处理记忆工具调用，返回 JSON 结果。
    async fn handle_tool_call(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, AppError>;

    /// 关闭并释放资源。
    async fn shutdown(&self) -> Result<(), AppError>;

    // ── 可选 session 生命周期 hooks ──
    async fn on_session_start(&self, _session_id: &str) {}
    async fn on_session_end(&self, _session_id: &str) {}
}
