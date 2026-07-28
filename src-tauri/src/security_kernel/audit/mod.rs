//! ═══════════════════════════════════════════════════════════════════════════
//! audit - 审计日志模块
//! ═══════════════════════════════════════════════════════════════════════════

pub mod bus;
pub mod event;
pub mod persistence;
pub mod store;
pub mod tauri_emit;

pub use bus::{AuditEventBus, EventHandler, SharedAuditEventBus, LoggingHandler, MetricsHandler};
pub use event::{SecurityEvent, AuditEntry, AuditFilter};
pub use persistence::DbAuditHandler;
pub use store::AuditStore;
pub use tauri_emit::{TauriEmitHandler, SecurityEventPayload, register_tauri_emit_handler};