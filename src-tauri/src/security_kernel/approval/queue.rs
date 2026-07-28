//! ═══════════════════════════════════════════════════════════════════════════
//! queue - 会话级审批队列
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::shared::error::AppError;

/// 心跳过期阈值：5 分钟无 poll 则过期。
const HEARTBEAT_TTL_SECONDS: i64 = 5 * 60;

// ── 审批请求 ────────────────────────────────────────────────────────────────

/// 审批请求（consent contract）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueApprovalRequest {
    /// 待审批命令。
    pub command: String,
    /// 命令执行上下文。
    pub context: String,
    /// 请求发起者（agent id / user id）。
    pub requested_by: String,
    /// 请求过期时间。
    pub expires_at: DateTime<Utc>,
}

// ── 审批状态 ────────────────────────────────────────────────────────────────

/// 审批状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// 等待审批。
    Pending,
    /// 已批准。
    Approved { by: String },
    /// 已拒绝。
    Denied { reason: String },
    /// 已过期（heartbeat 超时）。
    Expired,
}

/// 审批解决方案（resolve 入参，限制为终态决策）。
#[derive(Debug, Clone)]
pub enum ApprovalResolution {
    Approved { by: String },
    Denied { reason: String },
}

// ── 审批票据 ────────────────────────────────────────────────────────────────

/// 审批 ticket。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalTicket {
    /// ticket 唯一 ID。
    pub ticket_id: String,
    /// 所属 session。
    pub session_id: String,
    /// 审批请求。
    pub request: QueueApprovalRequest,
    /// 当前状态。
    pub status: ApprovalStatus,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
    /// 最后一次 poll 时间（heartbeat）。
    pub last_polled_at: DateTime<Utc>,
}

// ── 审批队列 ────────────────────────────────────────────────────────────────

/// Per-session FIFO 审批队列。
///
/// in-memory 存储，session 级生命周期（非持久化）。
pub struct ApprovalQueue {
    tickets: Arc<RwLock<HashMap<String, ApprovalTicket>>>,
}

impl ApprovalQueue {
    /// 构造空队列。
    pub fn new() -> Self {
        Self {
            tickets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 入队审批请求，返回 ticket。
    pub async fn enqueue(
        &self,
        session_id: impl Into<String>,
        request: QueueApprovalRequest,
    ) -> ApprovalTicket {
        let now = Utc::now();
        let ticket = ApprovalTicket {
            ticket_id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            request,
            status: ApprovalStatus::Pending,
            created_at: now,
            last_polled_at: now,
        };
        let mut tickets = self.tickets.write().await;
        tickets.insert(ticket.ticket_id.clone(), ticket.clone());
        ticket
    }

    /// 查询 ticket 状态（更新 heartbeat）。
    ///
    /// - ticket 不存在 → `Err(not_found)`
    /// - 已终态（Approved/Denied/Expired）→ 直接返回状态（不更新 heartbeat）
    /// - Pending 且超过 5 分钟未 poll → 标记 Expired 并返回
    /// - Pending 且未过期 → 更新 last_polled_at，返回 Pending
    pub async fn poll(&self, ticket_id: &str) -> Result<ApprovalStatus, AppError> {
        let mut tickets = self.tickets.write().await;
        let ticket = tickets
            .get_mut(ticket_id)
            .ok_or_else(|| AppError::not_found(format!("Approval ticket not found: {}", ticket_id)))?;

        // 已终态直接返回（不更新 heartbeat）
        if ticket.status != ApprovalStatus::Pending {
            return Ok(ticket.status.clone());
        }

        let now = Utc::now();
        // heartbeat 过期检查
        if now > ticket.last_polled_at + Duration::seconds(HEARTBEAT_TTL_SECONDS) {
            ticket.status = ApprovalStatus::Expired;
            return Ok(ApprovalStatus::Expired);
        }

        // 更新 heartbeat
        ticket.last_polled_at = now;
        Ok(ticket.status.clone())
    }

    /// 解决审批（用户或 smart approval 调用）。
    ///
    /// - ticket 不存在 → `Err(not_found)`
    /// - ticket 已非 Pending → `Err(invalid_state)`
    /// - 否则更新为终态
    pub async fn resolve(
        &self,
        ticket_id: &str,
        resolution: ApprovalResolution,
    ) -> Result<(), AppError> {
        let mut tickets = self.tickets.write().await;
        let ticket = tickets
            .get_mut(ticket_id)
            .ok_or_else(|| AppError::not_found(format!("Approval ticket not found: {}", ticket_id)))?;

        if ticket.status != ApprovalStatus::Pending {
            return Err(AppError::invalid_state(format!(
                "Approval ticket already resolved: {} (current: {:?})",
                ticket_id, ticket.status
            )));
        }

        ticket.status = match resolution {
            ApprovalResolution::Approved { by } => ApprovalStatus::Approved { by },
            ApprovalResolution::Denied { reason } => ApprovalStatus::Denied { reason },
        };
        Ok(())
    }
}

impl Default for ApprovalQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> QueueApprovalRequest {
        QueueApprovalRequest {
            command: "rm -rf ./build".to_string(),
            context: "cleanup build artifacts".to_string(),
            requested_by: "agent-1".to_string(),
            expires_at: Utc::now() + Duration::seconds(60),
        }
    }

    #[tokio::test]
    async fn test_enqueue_returns_pending_ticket() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        assert!(!ticket.ticket_id.is_empty());
        assert_eq!(ticket.session_id, "session-1");
        assert_eq!(ticket.status, ApprovalStatus::Pending);
        assert_eq!(ticket.request.command, "rm -rf ./build");
    }

    #[tokio::test]
    async fn test_poll_returns_pending_and_updates_heartbeat() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        let status = queue.poll(&ticket.ticket_id).await.unwrap();
        assert_eq!(status, ApprovalStatus::Pending);

        // heartbeat 应被更新
        let tickets = queue.tickets.read().await;
        let stored = tickets.get(&ticket.ticket_id).unwrap();
        assert!(stored.last_polled_at >= stored.created_at);
    }

    #[tokio::test]
    async fn test_poll_nonexistent_returns_err() {
        let queue = ApprovalQueue::new();
        let result = queue.poll("nonexistent").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_resolve_approved() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        queue
            .resolve(
                &ticket.ticket_id,
                ApprovalResolution::Approved { by: "admin".to_string() },
            )
            .await
            .unwrap();

        let status = queue.poll(&ticket.ticket_id).await.unwrap();
        match status {
            ApprovalStatus::Approved { by } => assert_eq!(by, "admin"),
            _ => panic!("expected Approved"),
        }
    }

    #[tokio::test]
    async fn test_resolve_denied() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        queue
            .resolve(
                &ticket.ticket_id,
                ApprovalResolution::Denied { reason: "too risky".to_string() },
            )
            .await
            .unwrap();

        let status = queue.poll(&ticket.ticket_id).await.unwrap();
        match status {
            ApprovalStatus::Denied { reason } => assert_eq!(reason, "too risky"),
            _ => panic!("expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_returns_err() {
        let queue = ApprovalQueue::new();
        let result = queue
            .resolve("nonexistent", ApprovalResolution::Approved { by: "admin".to_string() })
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FOUND");
    }

    #[tokio::test]
    async fn test_resolve_already_resolved_returns_err() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        queue
            .resolve(
                &ticket.ticket_id,
                ApprovalResolution::Approved { by: "admin".to_string() },
            )
            .await
            .unwrap();

        // 二次 resolve 应失败
        let result = queue
            .resolve(
                &ticket.ticket_id,
                ApprovalResolution::Denied { reason: "dup".to_string() },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_STATE");
    }

    #[tokio::test]
    async fn test_heartbeat_expiry() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        // 手动将 last_polled_at 设为 6 分钟前，模拟无 poll 过期
        {
            let mut tickets = queue.tickets.write().await;
            let stored = tickets.get_mut(&ticket.ticket_id).unwrap();
            stored.last_polled_at = Utc::now() - Duration::seconds(HEARTBEAT_TTL_SECONDS + 60);
        }

        // poll 应检测过期并标记 Expired
        let status = queue.poll(&ticket.ticket_id).await.unwrap();
        assert_eq!(status, ApprovalStatus::Expired);

        // 再次 poll 仍返回 Expired（终态不更新 heartbeat）
        let status = queue.poll(&ticket.ticket_id).await.unwrap();
        assert_eq!(status, ApprovalStatus::Expired);
    }

    #[tokio::test]
    async fn test_resolve_expired_returns_err() {
        let queue = ApprovalQueue::new();
        let ticket = queue.enqueue("session-1", make_request()).await;

        // 模拟过期
        {
            let mut tickets = queue.tickets.write().await;
            let stored = tickets.get_mut(&ticket.ticket_id).unwrap();
            stored.last_polled_at = Utc::now() - Duration::seconds(HEARTBEAT_TTL_SECONDS + 60);
        }
        queue.poll(&ticket.ticket_id).await.unwrap(); // 标记 Expired

        // 已过期的 ticket 不能再 resolve
        let result = queue
            .resolve(
                &ticket.ticket_id,
                ApprovalResolution::Approved { by: "admin".to_string() },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_STATE");
    }

    #[tokio::test]
    async fn test_multiple_sessions_isolated() {
        // 不同 session 的 ticket 独立存储
        let queue = ApprovalQueue::new();
        let t1 = queue.enqueue("session-1", make_request()).await;
        let t2 = queue.enqueue("session-2", make_request()).await;

        assert_ne!(t1.ticket_id, t2.ticket_id);
        assert_ne!(t1.session_id, t2.session_id);

        // 解决 session-1 不影响 session-2
        queue
            .resolve(
                &t1.ticket_id,
                ApprovalResolution::Approved { by: "admin".to_string() },
            )
            .await
            .unwrap();

        let s1 = queue.poll(&t1.ticket_id).await.unwrap();
        let s2 = queue.poll(&t2.ticket_id).await.unwrap();
        assert!(matches!(s1, ApprovalStatus::Approved { .. }));
        assert_eq!(s2, ApprovalStatus::Pending);
    }

    #[tokio::test]
    async fn test_default_impl() {
        let queue = ApprovalQueue::default();
        let ticket = queue.enqueue("s", make_request()).await;
        assert_eq!(ticket.status, ApprovalStatus::Pending);
    }
}