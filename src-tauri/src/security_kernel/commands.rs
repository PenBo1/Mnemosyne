// SecurityKernel IPC 命令:审计事件查询、统计、过滤、直方图、kernel 状态、approval 管理。
//
// 供仪表盘 Violations 卡片、审计日志页面、安全拦截面板调用。

use tauri::State;

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::audit::{
    AuditEventFilter, AuditHistogramBucket, AuditEventRow,
};
use crate::security_kernel::approval::{ApprovalId, ApprovalToken};
use crate::security_kernel::kernel::KernelStats;
use crate::security_kernel::state::SecurityKernelState;
use crate::shared::error::{AppError, IpcResponse};

// ── 审计事件查询 ──

/// 查询最近的审计事件(按 recorded_at 倒序)。limit 默认 50,上限 1000。
#[tauri::command]
pub async fn audit_events_query(
    state: State<'_, DbState>,
    limit: Option<i64>,
) -> Result<IpcResponse<Vec<AuditEventRow>>, AppError> {
    let limit = limit.unwrap_or(50);
    let rows = state.db.query_audit_events(limit)?;
    Ok(IpcResponse::ok(rows))
}

/// 审计事件聚合统计:总数 / 拒绝数 / 安全相关数 / 按类型分组。
#[tauri::command]
pub async fn audit_event_stats(
    state: State<'_, DbState>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let stats = state.db.audit_event_stats()?;
    Ok(IpcResponse::ok(serde_json::to_value(stats)?))
}

/// 按过滤条件查询审计事件。
///
/// 支持 workspace_id / operation(LIKE) / event_type / since / until / only_denied / only_security / offset / limit。
#[tauri::command]
pub async fn audit_events_query_filtered(
    filter: AuditEventFilter,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<AuditEventRow>>, AppError> {
    let rows = state.db.query_audit_events_filtered(&filter)?;
    Ok(IpcResponse::ok(rows))
}

/// 审计事件直方图(按时间桶聚合)。
///
/// granularity ∈ {"hour","day","month"}。since/until 为可选 RFC3339 字符串。
#[tauri::command]
pub async fn audit_event_histogram(
    granularity: String,
    since: Option<String>,
    until: Option<String>,
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<AuditHistogramBucket>>, AppError> {
    let buckets = state.db.audit_event_histogram(&granularity, since.as_deref(), until.as_deref())?;
    Ok(IpcResponse::ok(buckets))
}

// ── Kernel 状态 ──

/// 暴露 KernelStats —— 当前策略数 / rate 限频 / approval / quota / session 统计。
#[tauri::command]
pub async fn kernel_stats(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<KernelStats>, AppError> {
    Ok(IpcResponse::ok(state.stats()))
}

// ── Approval 管理 ──

/// ApprovalToken 的前端展示 DTO。剔除 action_hash 等内部字段。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalTokenDto {
    pub id: String,
    pub workspace: String,
    pub risk_level: String,
    pub created_at: String,
    pub expire: String,
    pub remaining_seconds: i64,
    pub is_expired: bool,
}

impl From<&ApprovalToken> for ApprovalTokenDto {
    fn from(t: &ApprovalToken) -> Self {
        Self {
            id: t.id.0.to_string(),
            workspace: t.workspace.0.to_string(),
            risk_level: format!("{:?}", t.risk_level).to_lowercase(),
            created_at: t.created_at.to_rfc3339(),
            expire: t.expire.to_rfc3339(),
            remaining_seconds: t.remaining_seconds(),
            is_expired: t.is_expired(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStatsDto {
    pub pending: usize,
    pub approved: usize,
    pub rejected: usize,
    pub expired: usize,
    pub oldest_pending_age: Option<i64>,
}

/// 列出当前 pending 的 approval tokens。
#[tauri::command]
pub async fn approval_list_pending(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<ApprovalTokenDto>>, AppError> {
    let approval = state.approval_manager().lock().unwrap();
    let tokens = approval.get_pending_tokens();
    let dtos = tokens.iter().map(ApprovalTokenDto::from).collect();
    Ok(IpcResponse::ok(dtos))
}

/// 获取 approval 统计(pending/approved/rejected/expired + oldest_pending_age)。
#[tauri::command]
pub async fn approval_stats(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<ApprovalStatsDto>, AppError> {
    let approval = state.approval_manager().lock().unwrap();
    let stats = approval.stats();
    Ok(IpcResponse::ok(ApprovalStatsDto {
        pending: stats.pending,
        approved: stats.approved,
        rejected: stats.rejected,
        expired: stats.expired,
        oldest_pending_age: stats.oldest_pending_age,
    }))
}

/// 批准一个 pending approval。
#[tauri::command]
pub async fn approval_grant(
    approval_id: String,
    approved_by: String,
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<()>, AppError> {
    let id = parse_approval_id(&approval_id)?;
    state.approve_operation(id, approved_by)?;
    Ok(IpcResponse::ok(()))
}

/// 拒绝一个 pending approval。
#[tauri::command]
pub async fn approval_reject(
    approval_id: String,
    rejected_by: String,
    reason: String,
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<()>, AppError> {
    let id = parse_approval_id(&approval_id)?;
    state.reject_approval(id, rejected_by, reason)?;
    Ok(IpcResponse::ok(()))
}

/// 清理过期的 pending approvals(标记为 expired,从 pending 移除)。
#[tauri::command]
pub async fn approval_cleanup_expired(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<usize>, AppError> {
    let approval = state.approval_manager().lock().unwrap();
    let cleaned = approval.cleanup_expired();
    Ok(IpcResponse::ok(cleaned.len()))
}

fn parse_approval_id(s: &str) -> Result<ApprovalId, AppError> {
    uuid::Uuid::parse_str(s)
        .map(ApprovalId)
        .map_err(|e| AppError::bad_request(format!("Invalid approval_id '{}': {}", s, e)))
}
