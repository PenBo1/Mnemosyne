// Agent Registry 模块 —— 统一 Agent 元数据注册表。
//
// 设计目标(AGENTS.md "统一抽象"):
// - 前端 Persona / 后端 SubAgentRole / pipeline agents / loop skill 四类 agent
//   此前无统一抽象，本模块提供 AgentDescriptor + AgentRegistry 统一查询入口
//
// 轻量原则:
// - 不接管 agent 执行（仍由 AgentEngine / PipelineRunner / SubAgentExecutor 负责）
// - 不强制重构现有 agent 实现
// - 仅作为元数据注册表，描述 "有哪些 agent、各自什么角色、用什么工具"
//
// 架构约束:
// - core/agent/registry 依赖 core/agent/subagent（读取 SubAgentRole 元数据）
//   与 core/agent/prompts（读取 ALL_ROLES），符合 core 内部依赖规则
// - commands.rs 暴露 IPC，依赖 Tauri State

pub mod builtin;
pub mod commands;
pub mod registry;
pub mod types;

pub use commands::AgentRegistryState;
pub use registry::AgentRegistry;
pub use types::{AgentCategory, AgentDescriptor};
