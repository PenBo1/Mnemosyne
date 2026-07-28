//! ═══════════════════════════════════════════════════════════════════════════
//! 工具限制模块 - 每对话工具执行上限配置
//! ═══════════════════════════════════════════════════════════════════════════

pub mod config;
pub mod commands;
pub mod state;

pub use config::ToolLimitsConfig;
pub use state::ToolLimitsState;
