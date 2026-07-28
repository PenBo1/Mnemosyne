//! ═══════════════════════════════════════════════════════════════════════════
//! Approval - 工具审批管理器
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::types::ChatEvent;

/// 审批请求超时时间（秒）
const APPROVAL_TIMEOUT_SECS: u64 = 300;

/// 工具审批管理器
/// 
/// 管理需要用户审批的工具调用请求，支持异步等待用户响应。
pub struct ApprovalManager {
    tx: mpsc::Sender<ChatEvent>,
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ApprovalManager {
    /// 创建审批管理器实例
    /// 
    /// # 参数
    /// - `tx`: 用于发送审批事件的消息发送端
    /// 
    /// # 返回值
    /// 返回包装在 Arc 中的审批管理器实例
    pub fn new(tx: mpsc::Sender<ChatEvent>) -> Arc<Self> {
        tracing::debug!("[approval] ApprovalManager created");
        Arc::new(Self {
            tx,
            pending: Mutex::new(HashMap::new()),
        })
    }

    /// 请求用户审批
    /// 
    /// # 参数
    /// - `name`: 工具名称
    /// - `args`: 工具参数
    /// 
    /// # 返回值
    /// 返回用户是否批准该工具调用
    pub async fn request_approval(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> bool {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();

        tracing::info!(
            request_id = %request_id,
            tool_name = %name,
            args_preview = %serde_json::to_string(args).unwrap_or_default(),
            "[approval] requesting user approval"
        );

        {
            let mut pending = self.pending.lock().await;
            pending.insert(request_id.clone(), response_tx);
            tracing::debug!(
                request_id = %request_id,
                pending_count = pending.len(),
                "[approval] request registered"
            );
        }

        if let Err(e) = self.tx.send(ChatEvent::ToolApprovalRequired {
            request_id: request_id.clone(),
            name: name.to_string(),
            args: args.clone(),
        }).await {
            tracing::error!(
                request_id = %request_id,
                error = %e,
                "[approval] failed to send ToolApprovalRequired event"
            );
            return false;
        }

        tracing::debug!(
            request_id = %request_id,
            timeout_secs = APPROVAL_TIMEOUT_SECS,
            "[approval] waiting for user response"
        );

        let result = match tokio::time::timeout(
            Duration::from_secs(APPROVAL_TIMEOUT_SECS),
            response_rx,
        ).await {
            Ok(Ok(approved)) => {
                tracing::info!(
                    request_id = %request_id,
                    approved = approved,
                    "[approval] user response received"
                );
                approved
            }
            Ok(Err(_)) => {
                tracing::warn!(
                    request_id = %request_id,
                    tool_name = %name,
                    "[approval] response channel closed without response"
                );
                false
            }
            Err(_) => {
                tracing::warn!(
                    request_id = %request_id,
                    tool_name = %name,
                    timeout_secs = APPROVAL_TIMEOUT_SECS,
                    "[approval] request timed out, treating as rejected"
                );
                false
            }
        };

        {
            let mut pending = self.pending.lock().await;
            pending.remove(&request_id);
            tracing::debug!(
                request_id = %request_id,
                pending_count = pending.len(),
                "[approval] request removed from pending"
            );
        }

        result
    }

    /// 响应审批请求
    /// 
    /// # 参数
    /// - `request_id`: 请求标识符
    /// - `approved`: 是否批准
    /// 
    /// # 返回值
    /// 返回是否成功找到并响应了该请求
    pub async fn respond(&self, request_id: &str, approved: bool) -> bool {
        tracing::debug!(
            request_id = %request_id,
            approved = approved,
            "[approval] respond called"
        );

        let mut pending = self.pending.lock().await;
        if let Some(tx) = pending.remove(request_id) {
            if tx.send(approved).is_err() {
                tracing::warn!(
                    request_id = %request_id,
                    "[approval] failed to send response (receiver dropped)"
                );
            }
            tracing::debug!(
                request_id = %request_id,
                pending_count = pending.len(),
                "[approval] response sent"
            );
            true
        } else {
            tracing::warn!(
                request_id = %request_id,
                "[approval] request not found (may have timed out)"
            );
            false
        }
    }
}