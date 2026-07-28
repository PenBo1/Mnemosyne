//! ═══════════════════════════════════════════════════════════════════════════
//! store - 审批存储模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::Utc;

use super::policy::ApprovalCacheEntry;
use super::token::{ApprovalId, ApprovalToken};

// ── 审批存储 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ApprovalStore {
    pending: HashMap<ApprovalId, ApprovalToken>,
    approved: HashSet<ApprovalId>,
    rejected: HashSet<ApprovalId>,
    expired: HashSet<ApprovalId>,
    /// 会话级审批缓存：action_hash → 缓存条目。
    ///
    /// 当用户选择"本次会话内始终允许"时，记录此条目，后续相同操作直接放行。
    /// 会话结束时调用 `clear_session_cache` 清空。
    session_cache: HashMap<String, ApprovalCacheEntry>,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            approved: HashSet::new(),
            rejected: HashSet::new(),
            expired: HashSet::new(),
            session_cache: HashMap::new(),
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
        self.session_cache.clear();
    }

    pub fn remove_expired_from_records(&mut self) {
        self.expired.clear();
    }

    // ── 会话级缓存 ────────────────────────────────────────────────────────

    /// 记录会话级审批决策。
    ///
    /// 当用户对某操作选择"本次会话内始终允许"或"拒绝"时，调用此方法记录决策。
    /// 后续相同 action_hash 的操作可直接从缓存读取，无需重复提示。
    pub fn record_session_approval(&mut self, action_hash: &str, entry: ApprovalCacheEntry) {
        self.session_cache.insert(action_hash.to_string(), entry);
    }

    /// 查询某操作是否在会话级缓存中被批准。
    ///
    /// 返回 `true` 表示用户此前选择了"本次会话内始终允许"，可直接放行。
    pub fn is_approved_for_session(&self, action_hash: &str) -> bool {
        self.session_cache
            .get(action_hash)
            .map(|e| e.is_session_approved())
            .unwrap_or(false)
    }

    /// 查询会话级缓存中的决策（若存在）。
    pub fn get_session_decision(&self, action_hash: &str) -> Option<&ApprovalCacheEntry> {
        self.session_cache.get(action_hash)
    }

    /// 清空会话级缓存。
    ///
    /// 应在会话结束时调用，确保下一会话重新提示。
    pub fn clear_session_cache(&mut self) {
        self.session_cache.clear();
    }

    /// 会话级缓存条目数。
    pub fn session_cache_count(&self) -> usize {
        self.session_cache.len()
    }
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── 审批统计 ────────────────────────────────────────────────────────────────

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

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::policy::{ApprovalCacheEntry, ReviewDecision};

    #[test]
    fn test_session_cache_initially_empty() {
        let store = ApprovalStore::new();
        assert_eq!(store.session_cache_count(), 0);
        assert!(!store.is_approved_for_session("any_hash"));
    }

    #[test]
    fn test_record_session_approval_approved() {
        let mut store = ApprovalStore::new();
        store.record_session_approval("action_123", ApprovalCacheEntry::approved_for_session());

        assert_eq!(store.session_cache_count(), 1);
        assert!(store.is_approved_for_session("action_123"));
    }

    #[test]
    fn test_record_session_approval_denied() {
        let mut store = ApprovalStore::new();
        store.record_session_approval("action_456", ApprovalCacheEntry::denied());

        assert!(!store.is_approved_for_session("action_456"));
        let entry = store.get_session_decision("action_456").unwrap();
        assert_eq!(entry.decision, ReviewDecision::Denied);
    }

    #[test]
    fn test_clear_session_cache() {
        let mut store = ApprovalStore::new();
        store.record_session_approval("action_1", ApprovalCacheEntry::approved_for_session());
        store.record_session_approval("action_2", ApprovalCacheEntry::approved_for_session());
        assert_eq!(store.session_cache_count(), 2);

        store.clear_session_cache();
        assert_eq!(store.session_cache_count(), 0);
        assert!(!store.is_approved_for_session("action_1"));
        assert!(!store.is_approved_for_session("action_2"));
    }

    #[test]
    fn test_clear_all_clears_session_cache() {
        let mut store = ApprovalStore::new();
        store.record_session_approval("action_1", ApprovalCacheEntry::approved_for_session());
        assert_eq!(store.session_cache_count(), 1);

        store.clear_all();
        assert_eq!(store.session_cache_count(), 0);
    }

    #[test]
    fn test_get_session_decision_returns_none_for_missing() {
        let store = ApprovalStore::new();
        assert!(store.get_session_decision("nonexistent").is_none());
    }

    #[test]
    fn test_record_session_approval_overwrites() {
        let mut store = ApprovalStore::new();
        store.record_session_approval("action_1", ApprovalCacheEntry::denied());
        assert!(!store.is_approved_for_session("action_1"));

        // 覆盖为 approved
        store.record_session_approval("action_1", ApprovalCacheEntry::approved_for_session());
        assert!(store.is_approved_for_session("action_1"));
        assert_eq!(store.session_cache_count(), 1);
    }
}