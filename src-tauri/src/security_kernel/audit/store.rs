use std::collections::VecDeque;
use std::sync::RwLock;

use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use super::super::WorkspaceId;
use super::event::{AuditEntry, AuditFilter, SecurityEvent};

const DEFAULT_MAX_ENTRIES: usize = 10000;
const DEFAULT_MAX_AGE_DAYS: i64 = 7;

pub struct AuditStore {
    entries: RwLock<VecDeque<AuditEntry>>,
    max_entries: usize,
    max_age: Duration,
}

impl AuditStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(DEFAULT_MAX_ENTRIES)),
            max_entries: DEFAULT_MAX_ENTRIES,
            max_age: Duration::days(DEFAULT_MAX_AGE_DAYS),
        }
    }

    pub fn with_limits(max_entries: usize, max_age_days: i64) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
            max_entries,
            max_age: Duration::days(max_age_days),
        }
    }

    pub fn store(&self, event: SecurityEvent) -> Uuid {
        let entry = AuditEntry::new(event);
        let id = entry.id;

        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());

        if entries.len() >= self.max_entries {
            entries.pop_front();
        }

        entries.push_back(entry);
        id
    }

    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.iter().filter(|e| filter.matches(e)).cloned().collect()
    }

    pub fn get_by_id(&self, id: Uuid) -> Option<AuditEntry> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.iter().find(|e| e.id == id).cloned()
    }

    pub fn get_latest(&self, count: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.iter().rev().take(count).cloned().collect()
    }

    pub fn get_for_workspace(&self, workspace: WorkspaceId) -> Vec<AuditEntry> {
        let filter = AuditFilter::for_workspace(workspace);
        self.query(&filter)
    }

    pub fn get_security_events(&self) -> Vec<AuditEntry> {
        let filter = AuditFilter::security_events();
        self.query(&filter)
    }

    pub fn get_denied_events(&self) -> Vec<AuditEntry> {
        let filter = AuditFilter::denied_events();
        self.query(&filter)
    }

    pub fn get_in_time_range(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Vec<AuditEntry> {
        let filter = AuditFilter::in_time_range(since, until);
        self.query(&filter)
    }

    pub fn count(&self) -> usize {
        self.entries.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn count_for_workspace(&self, workspace: WorkspaceId) -> usize {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.iter().filter(|e| e.event.workspace() == Some(&workspace)).count()
    }

    pub fn count_by_type(&self, event_type: &str) -> usize {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.iter().filter(|e| e.event.event_type() == event_type).count()
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now();
        let cutoff = now - self.max_age;

        let before = entries.len();
        entries.retain(|e| e.recorded_at >= cutoff);
        before - entries.len()
    }

    pub fn clear_workspace(&self, workspace: WorkspaceId) -> usize {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let before = entries.len();
        entries.retain(|e| e.event.workspace() != Some(&workspace));
        before - entries.len()
    }

    pub fn clear_all(&self) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        entries.clear();
    }
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new()
    }
}