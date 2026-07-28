//! ═══════════════════════════════════════════════════════════════════════════
//! commands - 安全内核 IPC 命令模块
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::State;
use std::time::Instant;

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
    let start = Instant::now();
    tracing::info!("audit_event_stats: enter");
    
    let stats = state.db.audit_event_stats()?;
    
    tracing::info!(
        duration_ms = start.elapsed().as_millis(),
        "audit_event_stats: exit"
    );
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
    let approval = state.approval_manager().lock().unwrap_or_else(|e| e.into_inner());
    let tokens = approval.get_pending_tokens();
    let dtos = tokens.iter().map(ApprovalTokenDto::from).collect();
    Ok(IpcResponse::ok(dtos))
}

/// 获取 approval 统计(pending/approved/rejected/expired + oldest_pending_age)。
#[tauri::command]
pub async fn approval_stats(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<ApprovalStatsDto>, AppError> {
    let approval = state.approval_manager().lock().unwrap_or_else(|e| e.into_inner());
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
    let start = Instant::now();
    tracing::info!(approval_id = %approval_id, rejected_by = %rejected_by, "approval_reject: enter");
    
    let id = parse_approval_id(&approval_id)?;
    tracing::debug!(approval_id = %approval_id, "approval_reject: approval_id parsed");
    
    state.reject_approval(id, rejected_by, reason)?;
    
    tracing::info!(
        approval_id = %approval_id,
        duration_ms = start.elapsed().as_millis(),
        "approval_reject: exit"
    );
    Ok(IpcResponse::ok(()))
}

/// 清理过期的 pending approvals(标记为 expired,从 pending 移除)。
#[tauri::command]
pub async fn approval_cleanup_expired(
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<usize>, AppError> {
    let approval = state.approval_manager().lock().unwrap_or_else(|e| e.into_inner());
    let cleaned = approval.cleanup_expired();
    Ok(IpcResponse::ok(cleaned.len()))
}

fn parse_approval_id(s: &str) -> Result<ApprovalId, AppError> {
    uuid::Uuid::parse_str(s)
        .map(ApprovalId)
        .map_err(|e| AppError::bad_request(format!("Invalid approval_id '{}': {}", s, e)))
}

// ── PVE（Prompt Validator Executor）命令 ──

/// PVE 工具调用验证请求 DTO。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PveValidateToolCallRequest {
    pub tool_name: String,
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub content_segments: Vec<ContentSegmentDto>,
    #[serde(default)]
    pub declared_intent: Option<String>,
    #[serde(default)]
    pub target_paths: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ContentSegmentDto {
    pub content: String,
    pub source: String,
}

/// 验证工具调用参数（PVE Layer 检查）。
///
/// 执行参数校验、注入扫描、意图检查。
#[tauri::command]
pub async fn pve_validate_tool_call(
    request: PveValidateToolCallRequest,
    state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<crate::security_kernel::pve::PveResult>, AppError> {
    use crate::security_kernel::pve::{ToolCallContext, ContentSource};

    let mut ctx = ToolCallContext::new(request.tool_name, request.parameters);

    for segment in request.content_segments {
        let source = match segment.source.to_lowercase().as_str() {
            "retrieved" => ContentSource::Retrieved,
            "user_message" => ContentSource::UserMessage,
            "tool_output" => ContentSource::ToolOutput,
            "agent_generated" => ContentSource::AgentGenerated,
            "system_prompt" => ContentSource::SystemPrompt,
            _ => ContentSource::UserMessage,
        };
        ctx = ctx.with_content(segment.content, source);
    }

    if let Some(intent) = request.declared_intent {
        ctx = ctx.with_intent(intent);
    }

    for path in request.target_paths {
        ctx = ctx.with_target_path(path);
    }

    let pve = state.pve().read().unwrap_or_else(|e| e.into_inner());
    let result = pve.execute(&ctx)?;

    Ok(IpcResponse::ok(result))
}

/// 获取 PVE 注入模式列表。
#[tauri::command]
pub async fn pve_get_injection_patterns(
    _state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<crate::security_kernel::pve::InjectionPattern>>, AppError> {
    use crate::security_kernel::pve::get_default_injection_patterns;

    let patterns = get_default_injection_patterns().to_vec();
    Ok(IpcResponse::ok(patterns))
}

/// 获取 PVE 敏感表面列表。
#[tauri::command]
pub async fn pve_get_sensitive_surfaces(
    _state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<crate::security_kernel::pve::SensitiveSurface>>, AppError> {
    use crate::security_kernel::pve::IntentChecker;

    let checker = IntentChecker::new();
    let surfaces = checker.sensitive_surfaces().to_vec();

    Ok(IpcResponse::ok(surfaces))
}

/// 检查路径是否为敏感表面。
#[tauri::command]
pub async fn pve_check_sensitive_path(
    path: String,
    _state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<crate::security_kernel::pve::SensitiveSurfaceMatch>>, AppError> {
    use crate::security_kernel::pve::IntentChecker;

    let checker = IntentChecker::new();
    let matches = checker.check_sensitive_surface(&path);

    Ok(IpcResponse::ok(matches))
}

/// 扫描内容中的注入模式。
#[tauri::command]
pub async fn pve_scan_content(
    content: String,
    source: String,
    _state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<Vec<crate::security_kernel::pve::InjectionMatchDto>>, AppError> {
    use crate::security_kernel::pve::{InjectionScanner, ContentSource};

    let content_source = match source.to_lowercase().as_str() {
        "retrieved" => ContentSource::Retrieved,
        "user_message" => ContentSource::UserMessage,
        "tool_output" => ContentSource::ToolOutput,
        "agent_generated" => ContentSource::AgentGenerated,
        "system_prompt" => ContentSource::SystemPrompt,
        _ => ContentSource::UserMessage,
    };

    let scanner = InjectionScanner::new();
    let matches = scanner.scan(&content, content_source);

    let dtos: Vec<_> = matches.iter()
        .map(crate::security_kernel::pve::InjectionMatchDto::from)
        .collect();

    Ok(IpcResponse::ok(dtos))
}
