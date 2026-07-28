//! ═══════════════════════════════════════════════════════════════════════════
//! tauri_emit - Tauri 事件桥接模块
//! ═══════════════════════════════════════════════════════════════════════════

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::security_kernel::audit::bus::EventHandler;
use crate::security_kernel::audit::event::{AuditEntry, SecurityEvent};

// ── 安全事件载荷 ────────────────────────────────────────────────────────────────

/// 推送到前端的 payload —— 包含 entry 元数据 + 完整 SecurityEvent。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityEventPayload {
    pub event_id: Uuid,
    pub event_type: String,
    pub recorded_at: String,
    pub operation: Option<String>,
    pub workspace_id: Option<String>,
    pub is_denied: bool,
    pub is_security_related: bool,
    pub event: SecurityEvent,
}

// ── Tauri 事件桥 ────────────────────────────────────────────────────────────────

/// Tauri 事件桥 —— 把审计事件实时推送到前端。
///
/// 持有 AppHandle 的克隆(AppHandle 内部是 Arc,克隆廉价)。
pub struct TauriEmitHandler {
    app: AppHandle,
}

impl TauriEmitHandler {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventHandler for TauriEmitHandler {
    fn handle(&self, event: &SecurityEvent, entry: &AuditEntry) {
        let payload = SecurityEventPayload {
            event_id: entry.id,
            event_type: event.event_type().to_string(),
            recorded_at: entry.recorded_at.to_rfc3339(),
            operation: event.operation().map(|s| s.to_string()),
            workspace_id: event.workspace().map(|w| w.0.to_string()),
            is_denied: event.is_denied(),
            is_security_related: event.is_security_related(),
            event: event.clone(),
        };

        // 使用 emit 而非 emit_to —— 广播到所有窗口
        if let Err(e) = self.app.emit("security://event", &payload) {
            tracing::warn!(
                error = %e,
                event_type = payload.event_type,
                "Failed to emit security event to frontend"
            );
        }
    }
}

// ── 注册函数 ────────────────────────────────────────────────────────────────

/// 在 setup 中注册 TauriEmitHandler 到 SecurityKernelState 的 audit_bus。
///
/// 由 lib.rs setup 调用 —— 需要 app.manage(SecurityKernelState) 已完成。
pub fn register_tauri_emit_handler(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<crate::security_kernel::state::SecurityKernelState>() {
        state.kernel().audit_bus().subscribe(Box::new(TauriEmitHandler::new(app.clone())));
        tracing::info!("TauriEmitHandler subscribed to audit bus");
    } else {
        tracing::warn!("SecurityKernelState not managed — TauriEmitHandler not registered");
    }
}