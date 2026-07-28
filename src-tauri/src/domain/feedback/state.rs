//! ═══════════════════════════════════════════════════════════════════════════
//! 反馈状态 - 反馈模块全局状态
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use tokio::sync::Mutex;
use super::store::FeedbackStore;

// ── 状态类型 ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FeedbackState {
    pub store: Arc<Mutex<FeedbackStore>>,
}

impl FeedbackState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(FeedbackStore::new())),
        }
    }
}

impl Default for FeedbackState {
    fn default() -> Self {
        Self::new()
    }
}