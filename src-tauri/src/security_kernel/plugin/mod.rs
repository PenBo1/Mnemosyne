//! ═══════════════════════════════════════════════════════════════════════════
//! plugin - 插件管理模块
//! ═══════════════════════════════════════════════════════════════════════════

mod manifest;
mod permission;
mod registry;
mod security;

pub use manifest::*;
pub use permission::*;
pub use registry::*;
pub use security::*;