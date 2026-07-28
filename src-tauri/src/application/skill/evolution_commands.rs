//! ═══════════════════════════════════════════════════════════════════════════
//! Evolution Commands - 技能进化 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 暴露技能使用统计和候选管理的 IPC 命令：
//! - usage：使用统计相关命令
//! - candidate：候选技能相关命令

use tauri::State;

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::skill_evolution::{
    SkillCandidateRow, SkillUsageStatsRow,
};
use crate::shared::error::{AppError, IpcResponse};
use std::time::Instant;

// ── usage 命令 ────────────────────────────────────────────────────────

/// 列出所有 skill 的使用统计(按 used_count 降序)
#[tauri::command]
pub async fn skill_usage_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<SkillUsageStatsRow>>, AppError> {
    let start = Instant::now();
    tracing::info!("skill_usage_list: enter");
    
    let rows = state.db.list_skill_usage_stats().map_err(|e| {
        tracing::error!(error = %e, "skill_usage_list: Failed to list skill usage stats");
        e
    })?;
    
    tracing::info!(
        count = rows.len(),
        duration_ms = start.elapsed().as_millis(),
        "skill_usage_list: exit"
    );
    Ok(IpcResponse::ok(rows))
}

/// 获取单个 skill 的使用统计
#[tauri::command]
pub async fn skill_usage_get(
    state: State<'_, DbState>,
    skill_name: String,
) -> Result<IpcResponse<Option<SkillUsageStatsRow>>, AppError> {
    let start = Instant::now();
    tracing::info!(skill_name = %skill_name, "skill_usage_get: enter");
    
    if skill_name.trim().is_empty() {
        tracing::error!("skill_usage_get: skillName cannot be empty");
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if skill_name.len() > 128 {
        tracing::error!(len = skill_name.len(), "skill_usage_get: skillName too long");
        return Err(AppError::invalid_input("skillName too long (max 128 chars)"));
    }
    
    let row = state.db.get_skill_usage_stats(&skill_name).map_err(|e| {
        tracing::error!(skill_name = %skill_name, error = %e, "skill_usage_get: Failed to get skill usage stats");
        e
    })?;
    
    tracing::info!(
        skill_name = %skill_name,
        found = row.is_some(),
        duration_ms = start.elapsed().as_millis(),
        "skill_usage_get: exit"
    );
    Ok(IpcResponse::ok(row))
}

/// 手动记录一次 skill 使用(前端调试或外部集成用)
///
/// success:本次使用是否成功
/// session_id:可选,关联的 session
#[tauri::command]
pub async fn skill_usage_record(
    state: State<'_, DbState>,
    skill_name: String,
    success: bool,
    session_id: Option<String>,
) -> Result<IpcResponse<()>, AppError> {
    let start = Instant::now();
    tracing::info!(
        skill_name = %skill_name,
        success,
        session_id = ?session_id,
        "skill_usage_record: enter"
    );
    
    if skill_name.trim().is_empty() {
        tracing::error!("skill_usage_record: skillName cannot be empty");
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if skill_name.len() > 128 {
        tracing::error!(len = skill_name.len(), "skill_usage_record: skillName too long");
        return Err(AppError::invalid_input("skillName too long (max 128 chars)"));
    }
    let sid = session_id.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else if s.len() > 128 {
            Some(s.chars().take(128).collect::<String>())
        } else {
            Some(s)
        }
    });
    
    state.db.record_skill_usage(&skill_name, success, sid.as_deref()).map_err(|e| {
        tracing::error!(skill_name = %skill_name, error = %e, "skill_usage_record: Failed to record skill usage");
        e
    })?;
    
    tracing::info!(
        skill_name = %skill_name,
        duration_ms = start.elapsed().as_millis(),
        "skill_usage_record: exit"
    );
    Ok(IpcResponse::ok(()))
}

/// 调整用户反馈分(每次用户给 +1/-1 分时调用)
///
/// delta:正负分(典型 ±1.0)
#[tauri::command]
pub async fn skill_usage_feedback(
    state: State<'_, DbState>,
    skill_name: String,
    delta: f64,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        skill_name = %skill_name,
        delta,
        "skill_usage_feedback: enter"
    );
    
    if skill_name.trim().is_empty() {
        tracing::error!("skill_usage_feedback: skillName cannot be empty");
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if !delta.is_finite() || delta.abs() > 10.0 {
        tracing::error!(delta, "skill_usage_feedback: delta must be finite in [-10, 10]");
        return Err(AppError::invalid_input("delta must be a finite number in [-10, 10]"));
    }
    
    let updated = state.db.adjust_skill_feedback(&skill_name, delta).map_err(|e| {
        tracing::error!(skill_name = %skill_name, error = %e, "skill_usage_feedback: Failed to adjust feedback");
        e
    })?;
    
    tracing::info!(
        skill_name = %skill_name,
        updated,
        duration_ms = start.elapsed().as_millis(),
        "skill_usage_feedback: exit"
    );
    Ok(IpcResponse::ok(updated))
}

// ── candidate 命令 ────────────────────────────────────────────────────────

/// 列出 skill 候选(可按 status 过滤)
///
/// status:可选 "pending" / "approved" / "rejected" / "superseded"
///         传 None 表示列出全部
#[tauri::command]
pub async fn skill_candidate_list(
    state: State<'_, DbState>,
    status: Option<String>,
) -> Result<IpcResponse<Vec<SkillCandidateRow>>, AppError> {
    let start = Instant::now();
    tracing::info!(status = ?status, "skill_candidate_list: enter");
    
    let s = match status.as_deref() {
        Some(s) if s.trim().is_empty() => None,
        Some(s) => Some(s),
        None => None,
    };
    
    let rows = state.db.list_skill_candidates(s).map_err(|e| {
        tracing::error!(error = %e, "skill_candidate_list: Failed to list skill candidates");
        e
    })?;
    
    tracing::info!(
        count = rows.len(),
        duration_ms = start.elapsed().as_millis(),
        "skill_candidate_list: exit"
    );
    Ok(IpcResponse::ok(rows))
}

/// 批准一个 pending 候选,将其转化为正式 skill
///
/// reviewer_notes:可选审核备注
#[tauri::command]
pub async fn skill_candidate_approve(
    state: State<'_, DbState>,
    candidate_id: String,
    reviewer_notes: Option<String>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        candidate_id = %candidate_id,
        reviewer_notes = ?reviewer_notes,
        "skill_candidate_approve: enter"
    );
    
    if candidate_id.trim().is_empty() {
        tracing::error!("skill_candidate_approve: candidateId cannot be empty");
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    // 1. 先查询候选内容(必须是 pending 才能 approve)
    let candidates = state.db.list_skill_candidates(Some("pending")).map_err(|e| {
        tracing::error!(error = %e, "skill_candidate_approve: Failed to list pending candidates");
        e
    })?;
    let target = candidates
        .into_iter()
        .find(|c| c.id == candidate_id)
        .ok_or_else(|| {
            tracing::error!(candidate_id = %candidate_id, "skill_candidate_approve: Candidate not found or not pending");
            AppError::not_found("candidate not found or not pending")
        })?;

    // 2. 标记候选为 approved
    let updated = state.db.update_candidate_status(&candidate_id, "approved", reviewer_notes.as_deref()).map_err(|e| {
        tracing::error!(candidate_id = %candidate_id, error = %e, "skill_candidate_approve: Failed to update candidate status");
        e
    })?;
    if !updated {
        tracing::info!(
            candidate_id = %candidate_id,
            duration_ms = start.elapsed().as_millis(),
            "skill_candidate_approve: exit (no update)"
        );
        return Ok(IpcResponse::ok(false));
    }

    // 3. 在 SkillManager 中创建正式 skill
    //    注意:此处只返回 approved 标记,真正的 skill 创建由前端调用 skill_create 完成
    //    (避免 skill_store 与 db 在此处交叉依赖)
    tracing::info!(
        candidate_id = %target.id,
        candidate_name = %target.candidate_name,
        "Skill candidate approved, ready for skill creation"
    );
    
    tracing::info!(
        candidate_id = %candidate_id,
        duration_ms = start.elapsed().as_millis(),
        "skill_candidate_approve: exit"
    );
    Ok(IpcResponse::ok(true))
}

/// 拒绝一个 pending 候选
#[tauri::command]
pub async fn skill_candidate_reject(
    state: State<'_, DbState>,
    candidate_id: String,
    reviewer_notes: Option<String>,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(
        candidate_id = %candidate_id,
        reviewer_notes = ?reviewer_notes,
        "skill_candidate_reject: enter"
    );
    
    if candidate_id.trim().is_empty() {
        tracing::error!("skill_candidate_reject: candidateId cannot be empty");
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    
    let updated = state.db.update_candidate_status(&candidate_id, "rejected", reviewer_notes.as_deref()).map_err(|e| {
        tracing::error!(candidate_id = %candidate_id, error = %e, "skill_candidate_reject: Failed to reject candidate");
        e
    })?;
    
    tracing::info!(
        candidate_id = %candidate_id,
        updated,
        duration_ms = start.elapsed().as_millis(),
        "skill_candidate_reject: exit"
    );
    Ok(IpcResponse::ok(updated))
}

/// 删除一个候选(用户主动清理)
#[tauri::command]
pub async fn skill_candidate_delete(
    state: State<'_, DbState>,
    candidate_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    let start = Instant::now();
    tracing::info!(candidate_id = %candidate_id, "skill_candidate_delete: enter");
    
    if candidate_id.trim().is_empty() {
        tracing::error!("skill_candidate_delete: candidateId cannot be empty");
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    
    let deleted = state.db.delete_skill_candidate(&candidate_id).map_err(|e| {
        tracing::error!(candidate_id = %candidate_id, error = %e, "skill_candidate_delete: Failed to delete candidate");
        e
    })?;
    
    tracing::info!(
        candidate_id = %candidate_id,
        deleted,
        duration_ms = start.elapsed().as_millis(),
        "skill_candidate_delete: exit"
    );
    Ok(IpcResponse::ok(deleted))
}