use std::sync::{Arc, RwLock};

use uuid::Uuid;

use super::event::{AuditEntry, AuditFilter, SecurityEvent};
use super::store::AuditStore;

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &SecurityEvent, entry: &AuditEntry);
}

type HandlerList = Vec<Box<dyn EventHandler>>;

pub struct AuditEventBus {
    store: AuditStore,
    handlers: RwLock<HandlerList>,
}

impl AuditEventBus {
    pub fn new() -> Self {
        Self {
            store: AuditStore::new(),
            handlers: RwLock::new(Vec::new()),
        }
    }

    pub fn with_store(store: AuditStore) -> Self {
        Self {
            store,
            handlers: RwLock::new(Vec::new()),
        }
    }

    pub fn emit(&self, event: SecurityEvent) -> Uuid {
        let id = self.store.store(event.clone());

        let entry = self.store.get_by_id(id).expect("Stored entry must exist");

        let handlers = self.handlers.read().unwrap();
        for handler in handlers.iter() {
            handler.handle(&event, &entry);
        }

        tracing::debug!(
            event_id = %id,
            event_type = event.event_type(),
            "Audit event emitted"
        );

        id
    }

    pub fn subscribe(&self, handler: Box<dyn EventHandler>) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push(handler);
        tracing::debug!(handler_count = handlers.len(), "Handler subscribed to audit bus");
    }

    pub fn unsubscribe_all(&self) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.clear();
        tracing::debug!("All handlers unsubscribed from audit bus");
    }

    pub fn handler_count(&self) -> usize {
        self.handlers.read().unwrap().len()
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

pub struct SharedAuditEventBus(Arc<AuditEventBus>);

impl SharedAuditEventBus {
    pub fn new() -> Self {
        Self(Arc::new(AuditEventBus::new()))
    }

    pub fn from(bus: AuditEventBus) -> Self {
        Self(Arc::new(bus))
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
        *self.denied_count.read().unwrap()
    }

    pub fn security_count(&self) -> usize {
        *self.security_count.read().unwrap()
    }

    pub fn reset(&self) {
        *self.denied_count.write().unwrap() = 0;
        *self.security_count.write().unwrap() = 0;
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
            *self.denied_count.write().unwrap() += 1;
        }
        if event.is_security_related() {
            *self.security_count.write().unwrap() += 1;
        }
    }
}