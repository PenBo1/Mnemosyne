//! ═══════════════════════════════════════════════════════════════════════════
//! 工具注册中心 - 统一工具管理模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 基于 async Tool trait 的统一工具注册中心。
//! 提供 check_fn TTL 缓存与 dynamic schema overrides，
//! 作为统一工具注册入口。

pub mod registry;
pub mod router;

pub use registry::{
    registry, register_tool, AvailabilityCheck, SchemaOverride, ToolEntry, ToolExposure,
    ToolRegistry,
};
pub use router::{
    build_tool_router, FeatureFlags, ToolRouter, ToolSearchEntry, ToolSearchHandler,
};
