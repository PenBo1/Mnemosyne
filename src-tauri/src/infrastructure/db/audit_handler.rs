//! ═══════════════════════════════════════════════════════════════════════════
//! audit_handler - 审计事件数据库持久化处理器
//! ═══════════════════════════════════════════════════════════════════════════

use crate::security_kernel::audit::bus::EventHandler;
use crate::security_kernel::audit::event::{AuditEntry, SecurityEvent};
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::audit::insert_audit_event;

// ── 数据库审计处理器 ────────────────────────────────────────────────────────

/// 把审计事件持久化到 SQLite audit_events 表。
///
/// 作为 EventHandler 订阅 AuditEventBus,每次 emit 都会落盘一行。
/// 写入失败时仅记录警告,不阻断主流程——审计日志不应影响业务执行。
pub struct DbAuditHandler {
    db: Database,
}

impl DbAuditHandler {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl EventHandler for DbAuditHandler {
    fn handle(&self, event: &SecurityEvent, entry: &AuditEntry) {
        let id = entry.id.to_string();
        let event_type = event.event_type();
        let operation = event.operation().map(|s| s.to_string());
        let workspace_id = event.workspace().map(|w| w.0.to_string());
        let is_denied = event.is_denied();
        let is_security_related = event.is_security_related();
        let recorded_at = entry.recorded_at.to_rfc3339();

        // 完整事件序列化为 JSON payload
        let payload = match serde_json::to_string(event) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, event_id = %id, "Failed to serialize audit event");
                return;
            }
        };

        if let Err(e) = insert_audit_event(
            &self.db,
            &id,
            event_type,
            operation.as_deref(),
            workspace_id.as_deref(),
            is_denied,
            is_security_related,
            &payload,
            &recorded_at,
        ) {
            tracing::warn!(error = %e, event_id = %id, "Failed to persist audit event to SQLite");
        }
    }
}