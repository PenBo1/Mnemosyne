
use std::sync::Arc;
use tokio::sync::Mutex;
use super::store::FeedbackStore;

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