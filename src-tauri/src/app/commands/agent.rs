use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use tauri::State;
use tauri::Emitter;

/// Shared agent event types
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    TurnStarted { session_id: String },
    StreamDelta { session_id: String, content: String },
    ToolCallBegin { session_id: String, tool_call_id: String, tool: String, args: String },
    ToolCallEnd { session_id: String, tool_call_id: String, output: String, is_error: bool },
    TurnCompleted { session_id: String, input_tokens: u32, output_tokens: u32 },
    Error { session_id: String, error: String },
    CompactionTriggered { session_id: String },
}

/// In-memory agent state (simplified for now)
static AGENT_STATE: std::sync::OnceLock<tokio::sync::Mutex<AgentState>> = std::sync::OnceLock::new();

struct AgentState {
    running: bool,
    session_id: Option<String>,
}

impl AgentState {
    fn new() -> Self {
        Self { running: false, session_id: None }
    }
}

fn get_agent_state() -> &'static tokio::sync::Mutex<AgentState> {
    AGENT_STATE.get_or_init(|| tokio::sync::Mutex::new(AgentState::new()))
}

#[tauri::command]
pub async fn agent_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    tracing::info!(session_id = %session_id, content_len = content.len(), "agent_send_message called");

    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Message content cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    // Save user message to DB
    {
        let db = state.db.lock().await;
        if let Err(e) = db.create_message(&session_id, "user", &content, None, None) {
            tracing::error!(error = %e, "Failed to save user message");
            return Err(AppError::internal(format!("Failed to save message: {}", e)));
        }
    }

    // Emit turn started event
    let _ = state.app_handle.emit("agent-event", AgentEvent::TurnStarted {
        session_id: session_id.clone(),
    });

    // Get or create provider
    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()
            .map_err(|e| AppError::provider_not_found(e.to_string()))?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    // Build system prompt
    let system = "你是 Mnemosyne，一个专业的 AI 创作助手。你帮助用户进行小说创作、角色设计、世界观构建、情节分析和趋势研究。请用中文回复。".to_string();

    // Call LLM
    let messages = vec![crate::infra::llm::Message {
        role: "user".to_string(),
        content: content.clone(),
        tool_calls: None,
        tool_call_id: None,
    }];

    match provider.complete(&model, &system, &messages).await {
        Ok(response) => {
            // Save assistant message
            {
                let db = state.db.lock().await;
                let _ = db.create_message(&session_id, "assistant", &response, None, None);
            }

            // Emit turn completed
            let _ = state.app_handle.emit("agent-event", AgentEvent::TurnCompleted {
                session_id: session_id.clone(),
                input_tokens: 0,
                output_tokens: 0,
            });

            tracing::info!("Agent response generated");
            Ok(IpcResponse::ok(response))
        }
        Err(e) => {
            let _ = state.app_handle.emit("agent-event", AgentEvent::Error {
                session_id: session_id.clone(),
                error: e.to_string(),
            });
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn agent_approve_tool(
    _state: State<'_, AppState>,
    tool_call_id: String,
    approved: bool,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(tool_call_id = %tool_call_id, approved, "Tool approval processed");
    // Tool approval is handled by the agent loop
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_cancel(
    _state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::warn!(session_id = %session_id, "Agent cancelled");
    let mut agent_state = get_agent_state().lock().await;
    agent_state.running = false;
    agent_state.session_id = None;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_compact(
    _state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(session_id = %session_id, "Compaction triggered");
    // Compaction is handled by the context transform
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_restart(
    _state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("Agent restarted");
    let mut agent_state = get_agent_state().lock().await;
    agent_state.running = false;
    agent_state.session_id = None;
    Ok(IpcResponse::ok(()))
}
