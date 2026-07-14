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
}

impl AgentState {
    pub fn new(engine: AgentEngine) -> Self {
        Self {
            engine,
            approval_managers: DashMap::new(),
        }
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

    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatEvent>(64);

    // Create approval manager and register it
    let approval = ApprovalManager::new(tx.clone());
    state.approval_managers.insert(request.session_id.clone(), approval.clone());

    // Forward events from internal channel to Tauri Channel
    let on_event_clone = on_event.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if on_event_clone.send(event).is_err() {
                break;
            }
        }
    });

    let chat_request = crate::core::agent::types::ChatRequest {
        session_id: request.session_id.clone(),
        content: request.content,
        context_text: request.context_text,
        custom_instructions: request.custom_instructions,
        effort: request.effort,
        collaboration_style: request.collaboration_style,
    };

    let result = state.engine.send_message(chat_request, root, approval, tx).await;

    // Clean up approval manager
    state.approval_managers.remove(&request.session_id);

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
