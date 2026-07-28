//! ═══════════════════════════════════════════════════════════════════════════
//! store - 速率限制存储模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc, Duration};

use super::super::WorkspaceId;

type RecordKey = (WorkspaceId, String);

pub struct RateStore {
    records: RwLock<HashMap<RecordKey, Vec<DateTime<Utc>>>>,
}

impl RateStore {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_records_in_window(
        &self,
        workspace: WorkspaceId,
        operation: &str,
        window: Duration,
    ) -> Vec<DateTime<Utc>> {
        let records = self.records.read().unwrap_or_else(|e| e.into_inner());
        let key = (workspace, operation.to_string());
        
        let now = Utc::now();
        let window_start = now - window;

        records
            .get(&key)
            .map(|timestamps| {
                timestamps
                    .iter()
                    .filter(|&ts| *ts >= window_start)
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn count_in_window(
        &self,
        workspace: WorkspaceId,
        operation: &str,
        window: Duration,
    ) -> u32 {
        self.get_records_in_window(workspace, operation, window).len() as u32
    }

    pub fn add_record(
        &self,
        workspace: WorkspaceId,
        operation: &str,
        timestamp: DateTime<Utc>,
    ) {
        let mut records = self.records.write().unwrap_or_else(|e| e.into_inner());
        let key = (workspace, operation.to_string());
        
        records
            .entry(key)
            .or_default()
            .push(timestamp);
    }

    pub fn cleanup_expired(&self, max_age: Duration) {
        let mut records = self.records.write().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now();
        let cutoff = now - max_age;

        for (_, timestamps) in records.iter_mut() {
            timestamps.retain(|ts| *ts >= cutoff);
        }

        records.retain(|_, timestamps| !timestamps.is_empty());
    }

    pub fn clear_workspace(&self, workspace: WorkspaceId) {
        let mut records = self.records.write().unwrap_or_else(|e| e.into_inner());
        records.retain(|(ws, _), _| *ws != workspace);
    }

    pub fn clear_all(&self) {
        let mut records = self.records.write().unwrap_or_else(|e| e.into_inner());
        records.clear();
    }

    pub fn total_records(&self) -> usize {
        self.records.read().unwrap_or_else(|e| e.into_inner()).len()
    }
}

impl Default for RateStore {
    fn default() -> Self {
        Self::new()
    }
}