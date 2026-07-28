//! ═══════════════════════════════════════════════════════════════════════════
//! Loop Engine - 循环引擎应用层模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供前端 loop_* IPC 命令的后端实现，包括：
//! - types：与前端对齐的 DTO（camelCase serde）
//! - builtin_patterns：内置模式定义 + DB 种子
//! - commands：IPC 命令实现

pub mod types;
pub mod builtin_patterns;
pub mod commands;
pub mod runner;

pub use types::{
    CostConfigDto, CreateLoopStateRequest, LoopConfigDto, LoopPatternDto, LoopRunLogDto,
    LoopRunResultDto, LoopStateDto, PhaseDefDto, PhaseResultDto, UpdateLoopStateRequest,
    UpsertLoopPatternRequest,
};
pub use builtin_patterns::ensure_builtin_patterns;
