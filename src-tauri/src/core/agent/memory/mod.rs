//! ═══════════════════════════════════════════════════════════════════════════
//! Memory - 可插拔记忆系统抽象层
//! ═══════════════════════════════════════════════════════════════════════════

pub mod manager;
pub mod provider;
pub mod phase1;
pub mod phase2;
pub mod artifact;
pub mod scrubber;

pub use manager::{format_memory_context, MemoryManager, CORE_TOOL_NAMES};
pub use provider::{MemoryProvider, ToolSchema};
