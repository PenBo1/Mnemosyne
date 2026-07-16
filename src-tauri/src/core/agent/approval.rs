use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::types::ChatEvent;

/// 审批请求的等待超时（秒）。
///
/// 超过此时间未收到前端响应视为拒绝，避免 agent 在用户离开时永久阻塞。
const APPROVAL_TIMEOUT_SECS: u64 = 300;

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

    /// Request user approval. Returns true if approved, false if rejected or timed out.
    ///
    /// # Cancel Safety
    ///
    /// This method is **cancel safe**. If cancelled while waiting for response,
    /// the pending request is automatically removed from the registry,
    /// preventing orphaned entries and memory leaks.
    ///
    /// 超时策略：超过 `APPROVAL_TIMEOUT_SECS` 未响应视为拒绝（避免 agent 永久阻塞）。
    /// 超时时显式清理 pending 表项，并在下方 `pending.remove` 兜底。
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

        // 用 timeout 包裹 oneshot，超时视为拒绝（No silent fallback：超时显式 log warn）
        let result = match tokio::time::timeout(
            Duration::from_secs(APPROVAL_TIMEOUT_SECS),
            response_rx,
        ).await {
            Ok(Ok(approved)) => approved,
            Ok(Err(_)) => {
                // sender 被 drop（不应发生，但 cancel-safe 处理）
                tracing::warn!(
                    request_id = %request_id,
                    name = name,
                    "Approval response channel closed without response"
                );
                false
            }
            Err(_) => {
                tracing::warn!(
                    request_id = %request_id,
                    name = name,
                    timeout_secs = APPROVAL_TIMEOUT_SECS,
                    "Approval request timed out, treating as rejected"
                );
                false
            }
        };

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