//! ═══════════════════════════════════════════════════════════════════════════
//! 工作区模块 - 工作区授权管理
//! ═══════════════════════════════════════════════════════════════════════════

pub mod registry;
pub mod errors;
pub mod state;

pub use registry::WorkspaceRegistry;
pub use state::WorkspaceState;