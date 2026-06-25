use std::sync::Arc;
use tokio::sync::mpsc;
use crate::errors::{AppError, IpcResponse};
use crate::app::state::MainAgentSessionState;
use crate::infra::fs_utils::validate_id_component;
use crate::AppState;
use tauri::State;
use tauri::Emitter;
use crate::domain::agents::main_agent::{AgentStatus, ConfirmationRequest, ConfirmationResponse};

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

#[tauri::command]
pub async fn main_agent_execute(
    session_id: String,
    goal: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    if goal.trim().is_empty() {
        return Err(AppError::invalid_input("Goal cannot be empty"));
    }
    if goal.len() > 50_000 {
        return Err(AppError::invalid_input("Goal too long (max 50000 chars)"));
    }
    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    let (progress_tx, progress_rx) = mpsc::unbounded_channel();
    let (confirmation_req_tx, confirmation_req_rx) = mpsc::unbounded_channel::<ConfirmationRequest>();
    let (confirmation_resp_tx, confirmation_resp_rx) = mpsc::unbounded_channel::<ConfirmationResponse>();

    // Store session state with the real progress_rx
    {
        let mut states = state.main_agent_states.lock().await;
        states.insert(session_id.clone(), MainAgentSessionState {
            progress_rx,
            confirmation_tx: confirmation_resp_tx,
            cancelled: Arc::new(tokio::sync::RwLock::new(false)),
        });
    }

    // Extract the progress_rx back out for the spawned listener task
    let progress_rx = {
        let mut states = state.main_agent_states.lock().await;
        states.get_mut(&session_id).map(|s| {
            std::mem::replace(&mut s.progress_rx, mpsc::unbounded_channel().1)
        })
    }.unwrap_or_else(|| {
        let (_, rx) = mpsc::unbounded_channel();
        rx
    });

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

    // Spawn progress listener
    let app_clone = app.clone();
    let sid_clone = session_id.clone();
    tokio::spawn(async move {
        let mut rx = progress_rx;
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
    });

    let agent = crate::domain::agents::main_agent::AgentLoop::new(
        ctx,
        progress_tx,
        confirmation_req_tx,
        confirmation_resp_rx,
    );

    // Spawn confirmation listener
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

#[tauri::command]
pub async fn main_agent_respond(
    session_id: String,
    approved: bool,
    modified_args: Option<String>,
    state: State<'_, AppState>,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let states = state.main_agent_states.lock().await;
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

#[tauri::command]
pub async fn main_agent_list_sessions(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<String>>, AppError> {
    let states = state.main_agent_states.lock().await;
    Ok(IpcResponse::ok(states.keys().cloned().collect()))
}

#[tauri::command]
pub async fn main_agent_cancel(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    let states = state.main_agent_states.lock().await;
    if let Some(session) = states.get(&session_id) {
        let mut cancelled = session.cancelled.write().await;
        *cancelled = true;
    }
    Ok(IpcResponse::ok("Cancel signal sent".to_string()))
}
