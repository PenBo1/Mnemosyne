//! ═══════════════════════════════════════════════════════════════════════════
//! Application - 应用层模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 应用层负责协调核心层与基础设施层，提供面向前端的 IPC 命令实现。

pub mod session;
pub mod workspace;
pub mod skill;
pub mod init;
pub mod loop_engine;
pub mod bridges;
pub mod llm;