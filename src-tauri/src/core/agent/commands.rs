//! ═══════════════════════════════════════════════════════════════════════════
//! Commands - Agent IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::Deserialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::core::agent::approval::ApprovalManager;
use crate::core::agent::collaboration_style::CollaborationStyle;
use crate::core::agent::effort::EffortLevel;
use crate::core::agent::engine::AgentEngine;
use crate::core::agent::types::ChatEvent;
use crate::core::agent::user_profile::{UserProfileProvider, UserProfileSnapshot};
use crate::shared::error::{AppError, IpcResponse};

// ── Agent 状态管理 ───────────────────────────────────────────────────────────

/// Agent 状态容器
/// 
/// 管理引擎实例、审批管理器和事件转发任务。
pub struct AgentState {
    pub engine: AgentEngine,
    pub approval_managers: DashMap<String, Arc<ApprovalManager>>,
    /// 每个会话的事件转发任务句柄, chat_stop 时可 abort
    forwarding_tasks: DashMap<String, tokio::task::JoinHandle<()>>,
    /// 用户画像提供者 (由 application/ 层注入)
    user_profile_provider: Option<Arc<dyn UserProfileProvider>>,
}

impl AgentState {
    /// 创建 Agent 状态实例
    pub fn new(engine: AgentEngine, user_profile_provider: Option<Arc<dyn UserProfileProvider>>) -> Self {
        Self {
            engine,
            approval_managers: DashMap::new(),
            forwarding_tasks: DashMap::new(),
            user_profile_provider,
        }
    }
}

// ── 会话清理守卫 ─────────────────────────────────────────────────────────────

/// 聊天会话清理守卫
/// 
/// 在 panic 或异常退出时自动清理会话资源。
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
            "[agent] Chat session cleaned up via panic guard (fallback path)"
        );
    }
}

// ── 请求结构体 ──────────────────────────────────────────────────────────────

/// 发送消息请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// 会话标识符
    pub session_id: String,
    /// 消息内容
    pub content: String,
    /// 上下文文本
    pub context_text: Option<String>,
    /// 自定义指令
    pub custom_instructions: Option<String>,
    /// 努力程度
    #[serde(default)]
    pub effort: Option<EffortLevel>,
    /// 协作风格
    #[serde(default)]
    pub collaboration_style: Option<CollaborationStyle>,
    /// 工具白名单
    #[serde(default)]
    pub tool_whitelist: Option<Vec<String>>,
}

// ── IPC 命令 ─────────────────────────────────────────────────────────────────

/// 发送聊天消息
/// 
/// 接收前端消息请求,创建事件转发任务并调用引擎处理。
#[tauri::command]
pub async fn chat_send_message(
    request: SendMessageRequest,
    workspace_root: Option<String>,
    on_event: Channel<ChatEvent>,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<()>, AppError> {
    let request_id = uuid::Uuid::new_v4();
    let start_time = std::time::Instant::now();
    let content_preview: String = request.content.chars().take(50).collect();

    tracing::info!(
        request_id = %request_id,
        session_id = %request.session_id,
        content_preview = %content_preview,
        has_context = request.context_text.is_some(),
        has_custom_instructions = request.custom_instructions.is_some(),
        effort = ?request.effort,
        collaboration_style = ?request.collaboration_style,
        "[agent] chat_send_message: IPC received"
    );

    let root: PathBuf = workspace_root
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    tracing::debug!(
        request_id = %request_id,
        workspace_root = %root.display(),
        "[agent] workspace root resolved"
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatEvent>(64);

    let approval = ApprovalManager::new(tx.clone());
    state.approval_managers.insert(request.session_id.clone(), approval.clone());
    tracing::debug!(
        request_id = %request_id,
        session_id = %request.session_id,
        "[agent] approval manager created and registered"
    );

    let on_event_clone = on_event.clone();
    let session_id_for_task = request.session_id.clone();
    let request_id_for_task = request_id;
    let handle = tokio::spawn(async move {
        tracing::debug!(
            request_id = %request_id_for_task,
            session_id = %session_id_for_task,
            "[agent] event forwarding task started"
        );
        let mut event_count: u64 = 0;
        let mut first_event_logged: bool = false;
        while let Some(event) = rx.recv().await {
            let event_kind = match &event {
                ChatEvent::TextDelta { .. } => "textDelta",
                ChatEvent::ReasoningDelta { .. } => "reasoningDelta",
                ChatEvent::ToolCallStart { .. } => "toolCallStart",
                ChatEvent::ToolCallDelta { .. } => "toolCallDelta",
                ChatEvent::ToolCallEnd { .. } => "toolCallEnd",
                ChatEvent::ToolApprovalRequired { .. } => "toolApprovalRequired",
                ChatEvent::Retry { .. } => "retry",
                ChatEvent::Finish { .. } => "finish",
                ChatEvent::Error { .. } => "error",
            };
            if !first_event_logged {
                first_event_logged = true;
                tracing::info!(
                    request_id = %request_id_for_task,
                    session_id = %session_id_for_task,
                    event_kind = event_kind,
                    "[forwarding] first event received"
                );
            }
            // 包装 5s 超时保护, 防止前端 channel 阻塞导致转发任务卡死
            match tokio::time::timeout(Duration::from_secs(5), async { on_event_clone.send(event) }).await {
                Ok(Ok(_)) => {
                    event_count += 1;
                    tracing::debug!(
                        request_id = %request_id_for_task,
                        session_id = %session_id_for_task,
                        event_kind = event_kind,
                        event_count = event_count,
                        "[forwarding] event sent"
                    );
                }
                Ok(Err(e)) => {
                    tracing::debug!(
                        request_id = %request_id_for_task,
                        session_id = %session_id_for_task,
                        event_kind = event_kind,
                        error = %e,
                        "[forwarding] send failed"
                    );
                    break;
                }
                Err(_elapsed) => {
                    tracing::error!(
                        request_id = %request_id_for_task,
                        session_id = %session_id_for_task,
                        event_kind = event_kind,
                        event_count = event_count,
                        "[forwarding] on_event.send timeout after 5s, skipping event"
                    );
                }
            }
        }
        tracing::info!(
            request_id = %request_id_for_task,
            session_id = %session_id_for_task,
            total_events = event_count,
            "[forwarding] task finished"
        );
    });
    state.forwarding_tasks.insert(request.session_id.clone(), handle);

    let chat_request = crate::core::agent::types::ChatRequest {
        session_id: request.session_id.clone(),
        content: request.content,
        context_text: request.context_text,
        custom_instructions: request.custom_instructions,
        effort: request.effort,
        collaboration_style: request.collaboration_style,
        tool_whitelist: request.tool_whitelist,
    };

    let mut guard = ChatSessionGuard {
        session_id: request.session_id.clone(),
        approval_managers: &state.approval_managers,
        forwarding_tasks: &state.forwarding_tasks,
        cleaned_up: false,
    };

    tracing::debug!(
        request_id = %request_id,
        session_id = %request.session_id,
        "[agent] calling engine.send_message"
    );

    // 用户画像快照由 UserProfileProvider 提供
    let user_profile: Option<UserProfileSnapshot> = state
        .user_profile_provider
        .as_ref()
        .map(|p| p.load_user_profile());

    let result = state
        .engine
        .send_message(chat_request, root, approval, tx, user_profile)
        .await;

    guard.cleaned_up = true;
    state.approval_managers.remove(&request.session_id);
    if let Some((_, handle)) = state.forwarding_tasks.remove(&request.session_id) {
        handle.abort();
    }

    match result {
        Ok(()) => {
            tracing::info!(
                request_id = %request_id,
                session_id = %request.session_id,
                duration_ms = start_time.elapsed().as_millis() as u64,
                success = true,
                "[agent] chat_send_message: IPC completed"
            );
            Ok(IpcResponse::ok(()))
        }
        Err(ref err) => {
            tracing::error!(
                request_id = %request_id,
                session_id = %request.session_id,
                error = ?err,
                duration_ms = start_time.elapsed().as_millis() as u64,
                "[agent] chat_send_message: IPC failed"
            );
            Err(result.unwrap_err())
        }
    }
}

/// 停止聊天会话
#[tauri::command]
pub async fn chat_stop(
    session_id: String,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(
        session_id = %session_id,
        "[agent] chat_stop: IPC received"
    );

    state.engine.stop(&session_id).await?;

    if let Some((_, handle)) = state.forwarding_tasks.remove(&session_id) {
        handle.abort();
        tracing::debug!(
            session_id = %session_id,
            "[agent] forwarding task aborted"
        );
    }

    tracing::info!(
        session_id = %session_id,
        "[agent] chat_stop: completed"
    );

    Ok(IpcResponse::ok(()))
}

/// 工具响应请求参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRespondRequest {
    /// 会话标识符
    pub session_id: String,
    /// 请求标识符
    pub request_id: String,
    /// 是否批准
    pub approved: bool,
}

/// 响应工具审批请求
#[tauri::command]
pub async fn chat_tool_respond(
    request: ToolRespondRequest,
    state: State<'_, AgentState>,
) -> Result<IpcResponse<bool>, AppError> {
    tracing::info!(
        session_id = %request.session_id,
        request_id = %request.request_id,
        approved = request.approved,
        "[agent] chat_tool_respond: IPC received"
    );

    let found = if let Some(approval) = state.approval_managers.get(&request.session_id) {
        let result = approval.respond(&request.request_id, request.approved).await;
        tracing::debug!(
            session_id = %request.session_id,
            request_id = %request.request_id,
            found = result,
            "[agent] approval response sent"
        );
        result
    } else {
        tracing::warn!(
            session_id = %request.session_id,
            request_id = %request.request_id,
            "[agent] approval manager not found for session"
        );
        false
    };

    tracing::info!(
        session_id = %request.session_id,
        request_id = %request.request_id,
        found = found,
        "[agent] chat_tool_respond: completed"
    );

    Ok(IpcResponse::ok(found))
}