//! ═══════════════════════════════════════════════════════════════════════════
//! bus - 审计事件总线
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::{mpsc::{self, SyncSender, TrySendError}, Arc, RwLock};

use uuid::Uuid;

use super::event::{AuditEntry, AuditFilter, SecurityEvent};
use super::store::AuditStore;

// ── 事件处理器 trait ────────────────────────────────────────────────────────

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &SecurityEvent, entry: &AuditEntry);
}

type HandlerList = Vec<Box<dyn EventHandler>>;

// ── 常量配置 ────────────────────────────────────────────────────────────────

/// 后台派发 channel 的容量。channel 满时 emit 丢弃事件并告警,
/// 避免审计阻塞业务主流程。
const DISPATCH_CHANNEL_CAPACITY: usize = 1024;

// ── 内部派发项 ────────────────────────────────────────────────────────────────

/// 后台派发任务的事件项（event + entry 一起发送,避免后台 task 再查 store）。
struct DispatchItem {
    event: SecurityEvent,
    entry: AuditEntry,
}

// ── 审计事件总线 ────────────────────────────────────────────────────────

pub struct AuditEventBus {
    store: AuditStore,
    handlers: RwLock<HandlerList>,
    // 后台派发 channel sender。None 时 emit 降级为同步派发（用于无 runtime 场景如测试）。
    dispatch_tx: RwLock<Option<SyncSender<DispatchItem>>>,
}

impl AuditEventBus {
    pub fn new() -> Self {
        Self {
            store: AuditStore::new(),
            handlers: RwLock::new(Vec::new()),
            dispatch_tx: RwLock::new(None),
        }
    }

    pub fn with_store(store: AuditStore) -> Self {
        Self {
            store,
            handlers: RwLock::new(Vec::new()),
            dispatch_tx: RwLock::new(None),
        }
    }

    /// 启动后台派发 task。必须在 tokio runtime 上下文中调用。
    /// 启动后 emit 会将事件推入 channel,由后台 spawn_blocking task 调用 handlers,
    /// 避免 handler 的同步 I/O（如 DbAuditHandler 写 SQLite）阻塞 emit 调用方。
    /// 无 runtime 时安全跳过,emit 降级为同步派发。
    pub fn start_dispatch_task(self: &Arc<Self>) {
        let handle = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => {
                tracing::warn!(
                    "No tokio runtime available, audit events will be dispatched synchronously"
                );
                return;
            }
        };

        let (tx, rx) = mpsc::sync_channel::<DispatchItem>(DISPATCH_CHANNEL_CAPACITY);
        *self.dispatch_tx.write().unwrap_or_else(|e| e.into_inner()) = Some(tx);

        let bus = Arc::clone(self);
        handle.spawn_blocking(move || {
            // 后台 task:阻塞接收事件,逐个调用 handlers。
            // 所有 sender drop 后 recv 返回 Err,task 自动退出。
            while let Ok(item) = rx.recv() {
                let handlers = bus.handlers.read().unwrap_or_else(|e| e.into_inner());
                for handler in handlers.iter() {
                    handler.handle(&item.event, &item.entry);
                }
            }
        });
        tracing::info!("Audit dispatch background task started");
    }

    pub fn emit(&self, event: SecurityEvent) -> Uuid {
        let id = self.store.store(event.clone());

        // 查不到 entry 时记录错误但不 panic,审计主流程不应中断
        let entry = match self.store.get_by_id(id) {
            Some(entry) => entry,
            None => {
                tracing::error!(event_id = %id, "Stored audit entry not found after store");
                return id;
            }
        };

        // 优先通过后台 channel 派发,避免 handler 同步 I/O 阻塞当前线程。
        // clone Sender 以便尽快释放 dispatch_tx 读锁。
        let tx = {
            let guard = self.dispatch_tx.read().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().cloned()
        };

        match tx {
            Some(tx) => match tx.try_send(DispatchItem { event, entry }) {
                Ok(()) => {
                    tracing::debug!(event_id = %id, "Audit event dispatched to background task");
                    id
                }
                Err(TrySendError::Full(_)) => {
                    // channel 满:丢弃事件,审计不应阻断业务
                    tracing::warn!(event_id = %id, "Audit dispatch channel full, event dropped");
                    id
                }
                Err(TrySendError::Disconnected(item)) => {
                    // channel 关闭（后台 task 已退出）:降级同步派发
                    tracing::warn!(
                        event_id = %id,
                        "Audit dispatch channel disconnected, falling back to sync dispatch"
                    );
                    let handlers = self.handlers.read().unwrap_or_else(|e| e.into_inner());
                    for handler in handlers.iter() {
                        handler.handle(&item.event, &item.entry);
                    }
                    id
                }
            },
            None => {
                // 无 channel（未启动 dispatch task 或无 runtime）:同步派发
                let handlers = self.handlers.read().unwrap_or_else(|e| e.into_inner());
                for handler in handlers.iter() {
                    handler.handle(&event, &entry);
                }
                tracing::debug!(
                    event_id = %id,
                    event_type = event.event_type(),
                    "Audit event emitted (sync)"
                );
                id
            }
        }
    }

    pub fn subscribe(&self, handler: Box<dyn EventHandler>) {
        let mut handlers = self.handlers.write().unwrap_or_else(|e| e.into_inner());
        handlers.push(handler);
        tracing::debug!(handler_count = handlers.len(), "Handler subscribed to audit bus");
    }

    pub fn unsubscribe_all(&self) {
        let mut handlers = self.handlers.write().unwrap_or_else(|e| e.into_inner());
        handlers.clear();
        tracing::debug!("All handlers unsubscribed from audit bus");
    }

    pub fn handler_count(&self) -> usize {
        self.handlers.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        self.store.query(filter)
    }

    pub fn get_by_id(&self, id: Uuid) -> Option<AuditEntry> {
        self.store.get_by_id(id)
    }

    pub fn get_latest(&self, count: usize) -> Vec<AuditEntry> {
        self.store.get_latest(count)
    }

    pub fn get_for_workspace(&self, workspace: super::super::WorkspaceId) -> Vec<AuditEntry> {
        self.store.get_for_workspace(workspace)
    }

    pub fn get_security_events(&self) -> Vec<AuditEntry> {
        self.store.get_security_events()
    }

    pub fn get_denied_events(&self) -> Vec<AuditEntry> {
        self.store.get_denied_events()
    }

    pub fn total_entries(&self) -> usize {
        self.store.count()
    }

    pub fn cleanup_expired(&self) -> usize {
        self.store.cleanup_expired()
    }

    pub fn clear_workspace(&self, workspace: super::super::WorkspaceId) -> usize {
        self.store.clear_workspace(workspace)
    }

    pub fn clear_all(&self) {
        self.store.clear_all()
    }

    pub fn store(&self) -> &AuditStore {
        &self.store
    }
}

impl Default for AuditEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ── 共享审计事件总线 ────────────────────────────────────────────────────────

pub struct SharedAuditEventBus(Arc<AuditEventBus>);

impl SharedAuditEventBus {
    pub fn new() -> Self {
        Self(Arc::new(AuditEventBus::new()))
    }

    pub fn from(bus: AuditEventBus) -> Self {
        Self(Arc::new(bus))
    }

    /// 启动后台派发 task。委托给内部 AuditEventBus。
    pub fn start_dispatch_task(&self) {
        self.0.start_dispatch_task();
    }

    pub fn emit(&self, event: SecurityEvent) -> Uuid {
        self.0.emit(event)
    }

    pub fn subscribe(&self, handler: Box<dyn EventHandler>) {
        self.0.subscribe(handler)
    }

    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        self.0.query(filter)
    }

    pub fn get_latest(&self, count: usize) -> Vec<AuditEntry> {
        self.0.get_latest(count)
    }

    pub fn cleanup_expired(&self) -> usize {
        self.0.cleanup_expired()
    }

    pub fn total_entries(&self) -> usize {
        self.0.total_entries()
    }

    pub fn inner(&self) -> &Arc<AuditEventBus> {
        &self.0
    }

    pub fn clone_inner(&self) -> Arc<AuditEventBus> {
        Arc::clone(&self.0)
    }
}

impl Default for SharedAuditEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedAuditEventBus {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

// ── 日志处理器 ────────────────────────────────────────────────────────────────

pub struct LoggingHandler;

impl EventHandler for LoggingHandler {
    fn handle(&self, event: &SecurityEvent, _entry: &AuditEntry) {
        if event.is_denied() {
            tracing::warn!(
                event_type = event.event_type(),
                "Security audit: denied event"
            );
        } else if event.is_security_related() {
            tracing::info!(
                event_type = event.event_type(),
                "Security audit: security event"
            );
        }
    }
}

// ── 指标处理器 ────────────────────────────────────────────────────────────────

pub struct MetricsHandler {
    denied_count: RwLock<usize>,
    security_count: RwLock<usize>,
}

impl MetricsHandler {
    pub fn new() -> Self {
        Self {
            denied_count: RwLock::new(0),
            security_count: RwLock::new(0),
        }
    }

    pub fn denied_count(&self) -> usize {
        *self.denied_count.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn security_count(&self) -> usize {
        *self.security_count.read().unwrap_or_else(|e| e.into_inner())
    }

    pub fn reset(&self) {
        *self.denied_count.write().unwrap_or_else(|e| e.into_inner()) = 0;
        *self.security_count.write().unwrap_or_else(|e| e.into_inner()) = 0;
    }
}

impl Default for MetricsHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for MetricsHandler {
    fn handle(&self, event: &SecurityEvent, _entry: &AuditEntry) {
        if event.is_denied() {
            *self.denied_count.write().unwrap_or_else(|e| e.into_inner()) += 1;
        }
        if event.is_security_related() {
            *self.security_count.write().unwrap_or_else(|e| e.into_inner()) += 1;
        }
    }
}