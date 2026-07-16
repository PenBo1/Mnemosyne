use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::Utc;

use super::token::{ApprovalId, ApprovalToken};

#[derive(Debug, Clone)]
pub struct ApprovalStore {
    pending: HashMap<ApprovalId, ApprovalToken>,
    approved: HashSet<ApprovalId>,
    rejected: HashSet<ApprovalId>,
    expired: HashSet<ApprovalId>,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            approved: HashSet::new(),
            rejected: HashSet::new(),
            expired: HashSet::new(),
        }
    }

    pub fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }

    pub fn add_pending(&mut self, token: ApprovalToken) {
        self.pending.insert(token.id, token);
    }

    pub fn get_pending(&self, id: &ApprovalId) -> Option<&ApprovalToken> {
        self.pending.get(id)
    }

    pub fn approve(&mut self, id: ApprovalId) -> Option<ApprovalToken> {
        if let Some(token) = self.pending.remove(&id) {
            self.approved.insert(id);
            return Some(token);
        }
        None
    }

    pub fn reject(&mut self, id: ApprovalId) -> Option<ApprovalToken> {
        if let Some(token) = self.pending.remove(&id) {
            self.rejected.insert(id);
            return Some(token);
        }
        None
    }

    pub fn mark_expired(&mut self, id: ApprovalId) -> Option<ApprovalToken> {
        if let Some(token) = self.pending.remove(&id) {
            self.expired.insert(id);
            return Some(token);
        }
        None
    }

    pub fn is_approved(&self, id: &ApprovalId) -> bool {
        self.approved.contains(id)
    }

    pub fn is_rejected(&self, id: &ApprovalId) -> bool {
        self.rejected.contains(id)
    }

    pub fn is_pending(&self, id: &ApprovalId) -> bool {
        self.pending.contains_key(id)
    }

    pub fn is_expired(&self, id: &ApprovalId) -> bool {
        self.expired.contains(id)
    }

    pub fn cleanup_expired(&mut self) -> Vec<ApprovalToken> {
        let expired_ids: Vec<ApprovalId> = self.pending
            .iter()
            .filter(|(_, token)| token.is_expired())
            .map(|(id, _)| *id)
            .collect();

        let removed: Vec<ApprovalToken> = expired_ids
            .into_iter()
            .filter_map(|id| self.mark_expired(id))
            .collect();

        // 防止 approved/rejected/expired 集合无限增长
        // 当超过上限时清空（这些集合仅用于近期 ID 查询，丢失旧记录不影响功能）
        const MAX_RECORDS: usize = 10000;
        if self.approved.len() > MAX_RECORDS {
            self.approved.clear();
        }
        if self.rejected.len() > MAX_RECORDS {
            self.rejected.clear();
        }
        if self.expired.len() > MAX_RECORDS {
            self.expired.clear();
        }

        removed
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn approved_count(&self) -> usize {
        self.approved.len()
    }

    pub fn rejected_count(&self) -> usize {
        self.rejected.len()
    }

    pub fn expired_count(&self) -> usize {
        self.expired.len()
    }

    pub fn total_count(&self) -> usize {
        self.pending_count() + self.approved_count() + self.rejected_count() + self.expired_count()
    }

    pub fn get_pending_tokens(&self) -> Vec<&ApprovalToken> {
        self.pending.values().collect()
    }

    pub fn get_pending_for_workspace(&self, workspace: &crate::security_kernel::WorkspaceId) -> Vec<&ApprovalToken> {
        self.pending
            .values()
            .filter(|token| token.matches_workspace(workspace))
            .collect()
    }

    pub fn clear_all(&mut self) {
        self.pending.clear();
        self.approved.clear();
        self.rejected.clear();
        self.expired.clear();
    }

    pub fn remove_expired_from_records(&mut self) {
        self.expired.clear();
    }
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ApprovalStats {
    pub pending: usize,
    pub approved: usize,
    pub rejected: usize,
    pub expired: usize,
    pub oldest_pending_age: Option<i64>,
}

impl ApprovalStore {
    pub fn stats(&self) -> ApprovalStats {
        let now = Utc::now();
        let oldest_pending_age = self.pending
            .values()
            .map(|token| (now - token.created_at).num_seconds())
            .min();

        ApprovalStats {
            pending: self.pending_count(),
            approved: self.approved_count(),
            rejected: self.rejected_count(),
            expired: self.expired_count(),
            oldest_pending_age,
        }
    }
}