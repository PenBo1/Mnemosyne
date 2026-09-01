//! ═══════════════════════════════════════════════════════════════════════════
//! AuditStore Trait - 审计存储抽象接口
//! ═══════════════════════════════════════════════════════════════════════════

use async_trait::async_trait;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::shared::error::AppError;

/// 审计统计数据
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub denied_count: usize,
    pub approval_pending: usize,
}

/// 审计存储抽象接口
#[async_trait]
pub trait AuditStore: Send + Sync {
    async fn insert(&self, event: super::event::AuditEntry) -> Result<Uuid, AppError>;
    async fn query(&self, filter: &super::event::AuditFilter) -> Result<Vec<super::event::AuditEntry>, AppError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<super::event::AuditEntry>, AppError>;
    async fn stats(&self) -> Result<AuditStats, AppError>;
}
