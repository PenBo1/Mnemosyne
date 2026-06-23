use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::errors::{AppError, IpcResponse};
use crate::AppState;
use tauri::State;
use tauri::Emitter;
use crate::domain::agents::main_agent::{AgentStatus, ProgressUpdate, ConfirmationRequest, ConfirmationResponse};

/// Agent progress event emitted to frontend
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum MainAgentEvent {
    Progress {
        session_id: String,
        status: AgentStatus,
        current_step: Option<u32>,
        total_steps: Option<u32>,
        message: String,
    },
    ConfirmationRequired {
        session_id: String,
        step_id: u32,
        description: String,
        details: String,
        risk_level: String,
    },
    Completed {
        session_id: String,
        result: String,
    },
    Failed {
        session_id: String,
        error: String,
    },
}

/// Per-session main agent state
struct MainAgentSessionState {
    progress_rx: mpsc::UnboundedReceiver<ProgressUpdate>,
    confirmation_tx: mpsc::UnboundedSender<ConfirmationResponse>,
    cancelled: Arc<tokio::sync::RwLock<bool>>,
}

static MAIN_AGENT_STATES: std::sync::OnceLock<Mutex<std::collections::HashMap<String, MainAgentSessionState>>> =
    std::sync::OnceLock::new();

fn main_agent_states() -> &'static Mutex<std::collections::HashMap<String, MainAgentSessionState>> {
    MAIN_AGENT_STATES.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Start autonomous execution of a user goal
#[tauri::command]
pub async fn main_agent_execute(
    session_id: String,
    goal: String,
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<IpcResponse<String>, AppError> {
    // Get provider and model from registry
    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    // Create channels
    let (progress_tx, progress_rx) = mpsc::unbounded_channel();
    let (confirmation_req_tx, confirmation_req_rx) = mpsc::unbounded_channel::<ConfirmationRequest>();
    let (confirmation_resp_tx, confirmation_resp_rx) = mpsc::unbounded_channel::<ConfirmationResponse>();

    // Store session state
    {
        let mut states = main_agent_states().lock().await;
        states.insert(session_id.clone(), MainAgentSessionState {
            progress_rx,
            confirmation_tx: confirmation_resp_tx,
            cancelled: Arc::new(tokio::sync::RwLock::new(false)),
        });
    }

    // Build AgentContext
    let work_dir = state.data_dir.root().to_path_buf();

    let memory = Arc::new(tokio::sync::RwLock::new(
        crate::domain::agents::base::MemorySystem::new(20)
    ));

    let mut tools = crate::domain::agents::base::ToolRegistry::new();
    use crate::domain::agents::tools::{ReadFileTool, WriteFileTool, ListFilesTool, BashTool, SearchMemoryTool, ArchiveMemoryTool};

    tools.register("read_file", Box::new(ReadFileTool::new(work_dir.clone())));
    tools.register("write_file", Box::new(WriteFileTool::new(work_dir.clone())));
    tools.register("list_files", Box::new(ListFilesTool::new(work_dir.clone())));
    tools.register("bash", Box::new(BashTool::new(work_dir.clone(), None)));
    tools.register("search_memory", Box::new(SearchMemoryTool::new(memory.clone())));
    tools.register("archive_memory", Box::new(ArchiveMemoryTool::new(memory.clone())));

    let ctx = crate::domain::agents::base::AgentContext {
        provider,
        model,
        project_root: work_dir.clone(),
        book_id: None,
        tools: Arc::new(tools),
        memory,
        iteration_budget: Arc::new(crate::domain::agents::iteration_budget::IterationBudget::new(50)),
        tool_guardrails: Arc::new(tokio::sync::Mutex::new(
            crate::domain::agents::tool_guardrails::ToolCallGuardrailController::new(
                crate::domain::agents::tool_guardrails::ToolGuardrailConfig::default()
            )
        )),
        context_compressor: Arc::new(tokio::sync::Mutex::new(
            crate::domain::agents::context_compressor::ContextCompressor::new(
                crate::domain::agents::context_compressor::CompressorConfig::default()
            )
        )),
        skill_manager: None,
        user_profile: None,
    };

    // Spawn progress listener — forward updates to frontend
    let app_clone = app.clone();
    let sid_clone = session_id.clone();
    let states_ref = main_agent_states();
    tokio::spawn(async move {
        let progress_rx = {
            let mut states = states_ref.lock().await;
            states.remove(&sid_clone).map(|s| s.progress_rx)
        };

        if let Some(mut rx) = progress_rx {
            while let Some(update) = rx.recv().await {
                match update.status {
                    AgentStatus::Completed => {
                        let _ = app_clone.emit("main-agent:progress", MainAgentEvent::Completed {
                            session_id: sid_clone.clone(),
                            result: update.message,
                        });
                    }
                    AgentStatus::Failed => {
                        let _ = app_clone.emit("main-agent:progress", MainAgentEvent::Failed {
                            session_id: sid_clone.clone(),
                            error: update.message,
                        });
                    }
                    _ => {
                        let _ = app_clone.emit("main-agent:progress", MainAgentEvent::Progress {
                            session_id: sid_clone.clone(),
                            status: update.status,
                            current_step: update.current_step,
                            total_steps: update.total_steps,
                            message: update.message,
                        });
                    }
                }
            }
        }
    });

    // Create and run agent loop
    let agent = crate::domain::agents::main_agent::AgentLoop::new(
        ctx,
        progress_tx,
        confirmation_req_tx,
        confirmation_resp_rx,
    );

    // Spawn confirmation listener — forward requests to frontend
    let app_clone2 = app.clone();
    let sid_clone2 = session_id.clone();
    tokio::spawn(async move {
        let mut req_rx = confirmation_req_rx;
        while let Some(req) = req_rx.recv().await {
            let _ = app_clone2.emit("main-agent:progress", MainAgentEvent::ConfirmationRequired {
                session_id: sid_clone2.clone(),
                step_id: req.step_id,
                description: req.description,
                details: req.details,
                risk_level: format!("{:?}", req.risk_level),
            });
        }
    });

    // Run execution
    let sid = session_id.clone();
    let app_handle = app.clone();
    tokio::spawn(async move {
        match agent.execute(&goal).await {
            Ok(result) => {
                let _ = app_handle.emit("main-agent:progress", MainAgentEvent::Completed {
                    session_id: sid,
                    result,
                });
            }
            Err(e) => {
                let _ = app_handle.emit("main-agent:progress", MainAgentEvent::Failed {
                    session_id: sid,
                    error: e.to_string(),
                });
            }
        }
    });

    Ok(IpcResponse::ok("Execution started".to_string()))
}

/// Respond to a confirmation request
#[tauri::command]
pub async fn main_agent_respond(
    session_id: String,
    approved: bool,
    modified_args: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<IpcResponse<String>, AppError> {
    let states = main_agent_states().lock().await;
    let session = states.get(&session_id)
        .ok_or_else(|| AppError::not_found("Session not found"))?;

    let response = if approved {
        ConfirmationResponse::Approved
    } else if let Some(args) = modified_args {
        ConfirmationResponse::Modified(args)
    } else {
        ConfirmationResponse::Rejected
    };

    session.confirmation_tx.send(response)
        .map_err(|_| AppError::internal("Failed to send confirmation response"))?;

    Ok(IpcResponse::ok("Response sent".to_string()))
}

/// Get active main agent sessions
#[tauri::command]
pub async fn main_agent_list_sessions(
    _state: State<'_, Arc<AppState>>,
) -> Result<IpcResponse<Vec<String>>, AppError> {
    let states = main_agent_states().lock().await;
    Ok(IpcResponse::ok(states.keys().cloned().collect()))
}

/// Cancel a running main agent execution
#[tauri::command]
pub async fn main_agent_cancel(
    session_id: String,
    _state: State<'_, Arc<AppState>>,
) -> Result<IpcResponse<String>, AppError> {
    let states = main_agent_states().lock().await;
    if let Some(session) = states.get(&session_id) {
        let mut cancelled = session.cancelled.write().await;
        *cancelled = true;
    }
    Ok(IpcResponse::ok("Cancel signal sent".to_string()))
}
