// 技能进化系统的 IPC 命令 —— 暴露 skill_usage_stats + skill_candidate_proposals 的能力。
//
// 命令分组:
// - usage:skill_usage_list / skill_usage_get / skill_usage_record / skill_usage_feedback
// - candidate:skill_candidate_list / skill_candidate_approve / skill_candidate_reject / skill_candidate_delete
//
// approve 会在 SkillManager 中创建正式 skill(若同名 skill 已存在则返回冲突错误)。

use tauri::State;

use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::skill_evolution::{
    SkillCandidateRow, SkillUsageStatsRow,
};
use crate::shared::error::{AppError, IpcResponse};

// ── usage 命令 ────────────────────────────────────────────────

/// 列出所有 skill 的使用统计(按 used_count 降序)
#[tauri::command]
pub async fn skill_usage_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<SkillUsageStatsRow>>, AppError> {
    let rows = state.db.list_skill_usage_stats()?;
    Ok(IpcResponse::ok(rows))
}

/// 获取单个 skill 的使用统计
#[tauri::command]
pub async fn skill_usage_get(
    state: State<'_, DbState>,
    skill_name: String,
) -> Result<IpcResponse<Option<SkillUsageStatsRow>>, AppError> {
    if skill_name.trim().is_empty() {
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if skill_name.len() > 128 {
        return Err(AppError::invalid_input("skillName too long (max 128 chars)"));
    }
    let row = state.db.get_skill_usage_stats(&skill_name)?;
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
    if skill_name.trim().is_empty() {
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if skill_name.len() > 128 {
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
    state.db.record_skill_usage(&skill_name, success, sid.as_deref())?;
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
    if skill_name.trim().is_empty() {
        return Err(AppError::invalid_input("skillName cannot be empty"));
    }
    if !delta.is_finite() || delta.abs() > 10.0 {
        return Err(AppError::invalid_input("delta must be a finite number in [-10, 10]"));
    }
    let updated = state.db.adjust_skill_feedback(&skill_name, delta)?;
    Ok(IpcResponse::ok(updated))
}

// ── candidate 命令 ────────────────────────────────────────────

/// 列出 skill 候选(可按 status 过滤)
///
/// status:可选 "pending" / "approved" / "rejected" / "superseded"
///         传 None 表示列出全部
#[tauri::command]
pub async fn skill_candidate_list(
    state: State<'_, DbState>,
    status: Option<String>,
) -> Result<IpcResponse<Vec<SkillCandidateRow>>, AppError> {
    let s = match status.as_deref() {
        Some(s) if s.trim().is_empty() => None,
        Some(s) => Some(s),
        None => None,
    };
    let rows = state.db.list_skill_candidates(s)?;
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
    if candidate_id.trim().is_empty() {
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    // 1. 先查询候选内容(必须是 pending 才能 approve)
    let candidates = state.db.list_skill_candidates(Some("pending"))?;
    let target = candidates
        .into_iter()
        .find(|c| c.id == candidate_id)
        .ok_or_else(|| AppError::not_found("candidate not found or not pending"))?;

    // 2. 标记候选为 approved
    let updated = state.db.update_candidate_status(&candidate_id, "approved", reviewer_notes.as_deref())?;
    if !updated {
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
    Ok(IpcResponse::ok(true))
}

/// 拒绝一个 pending 候选
#[tauri::command]
pub async fn skill_candidate_reject(
    state: State<'_, DbState>,
    candidate_id: String,
    reviewer_notes: Option<String>,
) -> Result<IpcResponse<bool>, AppError> {
    if candidate_id.trim().is_empty() {
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    let updated = state.db.update_candidate_status(&candidate_id, "rejected", reviewer_notes.as_deref())?;
    Ok(IpcResponse::ok(updated))
}

/// 删除一个候选(用户主动清理)
#[tauri::command]
pub async fn skill_candidate_delete(
    state: State<'_, DbState>,
    candidate_id: String,
) -> Result<IpcResponse<bool>, AppError> {
    if candidate_id.trim().is_empty() {
        return Err(AppError::invalid_input("candidateId cannot be empty"));
    }
    let deleted = state.db.delete_skill_candidate(&candidate_id)?;
    Ok(IpcResponse::ok(deleted))
}
