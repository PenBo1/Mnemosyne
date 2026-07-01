//! 子 Agent IPC 命令 — 展示当前会话中活跃/已完成的子 Agent，并支持取消运行中的子 Agent。
//!
//! 子 Agent 由主 Agent 通过 `spawn_subagent` 工具自主调用 spawn，前端不直接 spawn。
//! 这些命令仅用于查询状态与取消运行中的任务，遵循 IPC 层 "只校验+委派" 的约束。

use tauri::State;

use crate::core::agent::sub_agent::SubAgentInfo;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::shared::errors::{AppError, IpcResponse};
use crate::AppState;

/// 列出指定会话（parent_thread_id = session_id）下的所有子 Agent。
///
/// `session_id` 同时充当 parent_thread_id：主 Agent 直接 spawn 的子 Agent 的
/// `parent_thread_id` 字段即为主 Agent 的 session_id。
#[tauri::command]
pub async fn sub_agent_list(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<SubAgentInfo>>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let children = state.sub_agent_control.list_children(&session_id).await;
    Ok(IpcResponse::ok(children))
}

/// 查询单个子 Agent 的最新元信息。
///
/// 返回 registry 中的快照（status / role / task / depth / started_at 等）。
/// 注意：`SubAgentInfo` 不含执行结果（output/artifacts/error/duration_ms），
/// 结果通过 `spawn_subagent` 工具同步回传给主 Agent，不在此暴露。
#[tauri::command]
pub async fn sub_agent_get(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<IpcResponse<SubAgentInfo>, AppError> {
    validate_id_component(&task_id, "task_id")?;
    let info = state
        .sub_agent_control
        .registry()
        .get(&task_id)
        .await
        .ok_or_else(|| AppError::not_found(format!("Sub-agent {} not found", task_id)))?;
    Ok(IpcResponse::ok(info))
}

/// 取消运行中的子 Agent。
///
/// 通过设置取消标志实现：子 Agent 在下一轮 ReAct 循环开始时检查并退出。
/// 对已完成或不存在的 task_id 幂等返回成功（与 `SubAgentControl::cancel` 一致）。
#[tauri::command]
pub async fn sub_agent_cancel(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&task_id, "task_id")?;
    state.sub_agent_control.cancel(&task_id).await?;
    Ok(IpcResponse::no_content())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_session_id() {
        let result = validate_id_component("", "session_id");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_traversal_in_task_id() {
        let result = validate_id_component("../escape", "task_id");
        assert!(result.is_err());
    }
}
