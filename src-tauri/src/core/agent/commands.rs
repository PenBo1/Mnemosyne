use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::collaboration_style::CollaborationStyle;
use crate::core::agent::effort::EffortLevel;
use crate::core::agent::engine::AgentEngine;
use crate::core::agent::types::ChatEvent;
use crate::shared::error::{AppError, IpcResponse};

pub struct AgentState {
    pub engine: AgentEngine,
    pub approval_managers: DashMap<String, Arc<ApprovalManager>>,
    /// 每个会话的事件转发任务句柄，chat_stop 时可 abort
    forwarding_tasks: DashMap<String, tokio::task::JoinHandle<()>>,
}

impl AgentState {
    pub fn new(engine: AgentEngine) -> Self {
        Self {
            engine,
            approval_managers: DashMap::new(),
            forwarding_tasks: DashMap::new(),
        }
    }
}

/// chat 会话清理 guard：确保 `send_message` panic 时也能移除
/// approval_managers / forwarding_tasks 中的条目并 abort 转发任务。
/// 正常路径在函数末尾将 `cleaned_up` 置 true，Drop 变为 no-op。
struct ChatSessionGuard<'a> {
    session_id: String,
    approval_managers: &'a DashMap<String, Arc<ApprovalManager>>,
    forwarding_tasks: &'a DashMap<String, tokio::task::JoinHandle<()>>,
    cleaned_up: bool,
}

impl Drop for ChatSessionGuard<'_> {
    fn drop(&mut self) {
        if self.cleaned_up {
            return;
        }
        self.approval_managers.remove(&self.session_id);
        if let Some((_, handle)) = self.forwarding_tasks.remove(&self.session_id) {
            handle.abort();
        }
        tracing::warn!(
            session_id = %self.session_id,
            "Chat session cleaned up via panic guard (fallback path)"
        );
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub session_id: String,
    pub content: String,
    pub context_text: Option<String>,
    pub custom_instructions: Option<String>,
    /// Effort 级别覆盖(low/medium/high/ultra)
    /// 缺省时由 AgentEngine 使用 DEFAULT_EFFORT(Medium)
    #[serde(default)]
    pub effort: Option<EffortLevel>,
    /// 协作风格覆盖(efficient/thoughtful/patient/decisive)
    /// 缺省时使用 CollaborationStyle::default()(Efficient)
    #[serde(default)]
    pub collaboration_style: Option<CollaborationStyle>,
}

#[tauri::command]
pub async fn chat_send_message(
    request: SendMessageRequest,
    workspace_root: Option<String>,
    on_event: Channel<ChatEvent>,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<()>, AppError> {
    let root: PathBuf = workspace_root
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // mpsc buffer=64：LLM 流式输出 + tool 事件 bursts 时缓冲足够，
    // 前端消费慢时 send().await 会自然 backpressure 阻塞 producer（engine.send_message）。
    // 不用 unbounded channel：避免前端挂起时事件无限堆积导致 OOM。
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatEvent>(64);

    // Create approval manager and register it
    let approval = ApprovalManager::new(tx.clone());
    state.approval_managers.insert(request.session_id.clone(), approval.clone());

    // Forward events from internal channel to Tauri Channel
    // 句柄存入 state，chat_stop 时可 abort
    let on_event_clone = on_event.clone();
    let handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if on_event_clone.send(event).is_err() {
                break;
            }
        }
    });
    state.forwarding_tasks.insert(request.session_id.clone(), handle);

    let chat_request = crate::core::agent::types::ChatRequest {
        session_id: request.session_id.clone(),
        content: request.content,
        context_text: request.context_text,
        custom_instructions: request.custom_instructions,
        effort: request.effort,
        collaboration_style: request.collaboration_style,
    };

    // panic-safe guard：send_message panic 时由 Drop 兜底清理
    let mut guard = ChatSessionGuard {
        session_id: request.session_id.clone(),
        approval_managers: &state.approval_managers,
        forwarding_tasks: &state.forwarding_tasks,
        cleaned_up: false,
    };

    let result = state.engine.send_message(chat_request, root, approval, tx).await;

    // 正常路径：显式清理并标记 guard 为 no-op
    guard.cleaned_up = true;
    state.approval_managers.remove(&request.session_id);
    // 清理转发任务句柄（任务应已自然结束：tx 被丢弃后 rx 返回 None）
    if let Some((_, handle)) = state.forwarding_tasks.remove(&request.session_id) {
        handle.abort();
    }

    match result {
        Ok(()) => Ok(IpcResponse::ok(())),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn chat_stop(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<()>, AppError> {
    state.engine.stop(&session_id).await?;
    // abort 事件转发任务，防止悬挂
    if let Some((_, handle)) = state.forwarding_tasks.remove(&session_id) {
        handle.abort();
    }
    Ok(IpcResponse::ok(()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRespondRequest {
    pub session_id: String,
    pub request_id: String,
    pub approved: bool,
}

#[tauri::command]
pub async fn chat_tool_respond(
    request: ToolRespondRequest,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<bool>, AppError> {
    let found = if let Some(approval) = state.approval_managers.get(&request.session_id) {
        approval.respond(&request.request_id, request.approved).await
    } else {
        false
    };
    Ok(IpcResponse::ok(found))
}
