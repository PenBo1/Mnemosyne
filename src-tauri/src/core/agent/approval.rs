use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::types::ChatEvent;

/// Manages tool approval flow: sends approval requests to the frontend,
/// waits for user response, and resolves pending tool calls.
pub struct ApprovalManager {
    tx: mpsc::Sender<ChatEvent>,
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ApprovalManager {
    pub fn new(tx: mpsc::Sender<ChatEvent>) -> Arc<Self> {
        Arc::new(Self {
            tx,
            pending: Mutex::new(HashMap::new()),
        })
    }

    /// Request user approval. Returns true if approved, false if rejected.
    ///
    /// # Cancel Safety
    ///
    /// This method is **cancel safe**. If cancelled while waiting for response,
    /// the pending request is automatically removed from the registry,
    /// preventing orphaned entries and memory leaks.
    pub async fn request_approval(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> bool {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(request_id.clone(), response_tx);
        }

        let _ = self.tx.send(ChatEvent::ToolApprovalRequired {
            request_id: request_id.clone(),
            name: name.to_string(),
            args: args.clone(),
        }).await;

        let result = response_rx.await.unwrap_or(false);

        {
            let mut pending = self.pending.lock().await;
            pending.remove(&request_id);
        }

        result
    }

    /// Resolve a pending approval request (called by chat_tool_respond IPC).
    pub async fn respond(&self, request_id: &str, approved: bool) -> bool {
        if let Some(tx) = self.pending.lock().await.remove(request_id) {
            let _ = tx.send(approved);
            true
        } else {
            false
        }
    }
}