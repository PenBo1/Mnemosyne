//! ═══════════════════════════════════════════════════════════════════════════
//! 雷达操作桥接 - Pipeline → Radar 跨域调用抽象
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 架构约束（AGENTS.md）：domain/ 模块间禁止横向依赖。
//! domain::pipeline::scheduler 需要触发雷达扫描，但不能直接 import domain::radar::agent。
//! 本 trait 将"执行雷达扫描"抽象为 trait 方法，由 application/bridges.rs 提供具体实现，
//! 经 SchedulerState 构造时注入，从而消除 pipeline → radar 的横向依赖。

use std::sync::Arc;

use async_trait::async_trait;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

/// 雷达扫描结果（最小 DTO，避免泄露 domain::radar::agent::ScanOutcome）。
pub struct RadarScanOutcome {
    pub recommendations_count: usize,
}

/// 雷达扫描操作 trait：由 application 层实现并注入到 Scheduler。
#[async_trait]
pub trait RadarScanOps: Send + Sync {
    async fn scan(&self, engine: &AgentEngine) -> Result<RadarScanOutcome, AppError>;
}

/// Tauri State 包装（供 IPC 命令直接访问雷达扫描能力时使用）。
pub struct RadarScanOpsState {
    pub ops: Arc<dyn RadarScanOps>,
}
