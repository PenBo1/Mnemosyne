//! ═══════════════════════════════════════════════════════════════════════════
//! Registry - Agent 元数据注册表
//! ═══════════════════════════════════════════════════════════════════════════

pub mod builtin;
pub mod commands;
pub mod store;
pub mod types;

pub use commands::AgentRegistryState;
pub use store::AgentRegistry;
pub use types::{AgentCategory, AgentDescriptor};
