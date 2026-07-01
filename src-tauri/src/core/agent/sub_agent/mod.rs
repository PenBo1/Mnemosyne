//! 子 Agent 模块 — 主 Agent 自主 spawn 的轻量子 Agent 系统。
//!
//! 设计参考 codex 的 sub-agent 机制，适配 Thalia 的角色工具白名单需求。
//!
//! ## 核心架构
//!
//! - `SubAgentControl`（单实例）：子 Agent 系统的入口，管理 spawn/cancel/查询
//! - `ThreadRegistry`：所有子 Agent 的元信息注册表（`RwLock<HashMap>`）
//! - `SpawnAgentTool`：注册到主 Agent 的 ToolRegistry，让 LLM 自主调用 spawn
//! - 结果通过 `oneshot::channel` 异步回传给调用方
//!
//! ## 与 codex 的差异
//!
//! | 特性 | codex | Thalia |
//! |------|-------|--------|
//! | 角色工具白名单 | 无 | 有（Researcher/Critic 只读，Outliner 限写） |
//! | 结果格式 | 纯文本 | 结构化（status/output/artifacts） |
//! | 协作骨架 | 单实例 AgentControl + ThreadId 树 | 保留 |
//! | 并发上限 | 6 | 4（更保守） |
//! | 深度限制 | agent_max_depth | MAX_DEPTH = 3 |
//!
//! ## 安全边界
//!
//! - 子 Agent 工具集由角色白名单决定，最小权限原则
//! - 子 Agent 不携带 `spawn_subagent` 工具，防止递归 spawn
//! - 深度和并发上限作为安全网
//! - 取消通过 `Arc<RwLock<bool>>` 标志实现（与 AgentSessionState 一致）

pub mod types;
pub mod role;
pub mod registry;
pub mod completion;
pub mod control;
pub mod spawn_tool;

// 关键类型 re-export
pub use control::{ParentAgentRefs, SubAgentControl};
pub use registry::{ThreadRegistry, MAX_CONCURRENT, MAX_DEPTH};
pub use role::{build_role_tool_registry, system_prompt_fragment, RoleConfig};
pub use spawn_tool::SpawnAgentTool;
pub use types::{
    SubAgentInfo, SubAgentResult, SubAgentRole, SubAgentSpawnRequest, SubAgentStatus,
};
