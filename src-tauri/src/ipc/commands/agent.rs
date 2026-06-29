use std::sync::Arc;
use futures::StreamExt;
use crate::shared::errors::{AppError, IpcResponse};
use crate::infrastructure::llm_client::types::{Message, StreamEvent, FinishReason};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::core::agent::chat_loop::{
    build_system_prompt, agent_tool_specs, execute_tool, load_history,
    compact_history, compact_messages_simple,
};
use crate::core::state::AgentSessionState;
use crate::AppState;
use tauri::State;
use tauri::Emitter;

const MAX_TURNS: usize = 15;
const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;

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

async fn get_cancel_flag(
    agent_states: &tokio::sync::Mutex<std::collections::HashMap<String, AgentSessionState>>,
    session_id: &str,
) -> Arc<tokio::sync::RwLock<bool>> {
    let mut states = agent_states.lock().await;
    states.entry(session_id.to_string())
        .or_insert_with(|| AgentSessionState {
            cancelled: Arc::new(tokio::sync::RwLock::new(false)),
        })
        .cancelled.clone()
}

#[tauri::command]
pub async fn agent_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Message content cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    if let Err(e) = state.db.create_message(&session_id, "user", &content, None, None).await {
        tracing::error!(error = %e, "Failed to save user message");
        return Err(AppError::internal(format!("Failed to save message: {}", e)));
    }

    let _ = state.app_handle.emit("agent-event", AgentEvent::TurnStarted {
        session_id: session_id.clone(),
    });

    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()
            .map_err(|e| AppError::provider_not_found(e.to_string()))?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    let cancel_flag = get_cancel_flag(&state.agent_states, &session_id).await;
    *cancel_flag.write().await = false;

    let tools = agent_tool_specs();
    let project_root = state.data_dir.root().join("workspace");
    let _ = std::fs::create_dir_all(&project_root);

    let system_prompt = {
        let feedback = state.feedback_store.lock().await;
        let skills = state.skill_manager.lock().await;
        build_system_prompt(&feedback, &skills)
    };

    let mut all_messages: Vec<Message> = {
        load_history(&state.db, &session_id).await?
    };

    let budget = crate::infrastructure::ai_services::token_budget::ContextBudget::for_window(DEFAULT_CONTEXT_WINDOW);
    let total_history_tokens: u32 = all_messages.iter()
        .map(|m| crate::infrastructure::ai_services::token_budget::estimate_tokens(&m.content))
        .sum();
    if budget.needs_compaction(total_history_tokens) {
        let max_msgs = budget.max_messages_after_compact(200);
        if compact_history(&mut all_messages, max_msgs) {
            let _ = state.app_handle.emit("agent-event", AgentEvent::CompactionTriggered {
                session_id: session_id.clone(),
            });
        }
    }

    let mut full_response = String::new();
    let mut total_input: u32 = 0;
    let mut total_output: u32 = 0;

    for _turn in 0..MAX_TURNS {
        if *cancel_flag.read().await {
            tracing::warn!(session_id = %session_id, "Agent turn cancelled");
            break;
        }

        let messages_json = serde_json::to_string(&all_messages).unwrap_or_default();
        let tools_json = serde_json::to_string(&tools).unwrap_or_default();
        let llm_call_id = state.db.log_llm_call_start(&session_id, "chat", &model, "default", Some(&system_prompt), &messages_json, Some(&tools_json), None, None).await?;
        let llm_start = std::time::Instant::now();

        let stream = provider.stream(&model, &system_prompt, &all_messages, &tools).await?;

        let mut text_buf = String::new();
        let mut tool_calls: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
        let mut finish_reason = FinishReason::Stop;

        tokio::pin!(stream);
        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::TextDelta { content } => {
                    text_buf.push_str(&content);
                    let _ = state.app_handle.emit("agent-event", AgentEvent::StreamDelta {
                        session_id: session_id.clone(),
                        content,
                    });
                }
                StreamEvent::ToolCallStart { id, name } => {
                    let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallBegin {
                        session_id: session_id.clone(),
                        tool_call_id: id.clone(),
                        tool: name.clone(),
                        args: String::new(),
                    });
                    tool_calls.entry(id).or_insert((name, String::new()));
                }
                StreamEvent::ToolCallDelta { id, args_delta } => {
                    if let Some(entry) = tool_calls.get_mut(&id) {
                        entry.1.push_str(&args_delta);
                    }
                }
                StreamEvent::ToolCallEnd { id } => {
                    if let Some((name, args)) = tool_calls.get(&id) {
                        let name = name.clone();
                        let args_len = args.len();
                        let args_clone = args.clone();
                        let valid_args = match serde_json::from_str::<serde_json::Value>(&args_clone) {
                            Ok(_) => args_clone,
                            Err(_) => {
                                let repaired = format!("{}\"}}", args_clone.trim_end_matches(|c: char| c == ',' || c == ' '));
                                match serde_json::from_str::<serde_json::Value>(&repaired) {
                                    Ok(_) => repaired,
                                    Err(_) => {
                                        tracing::warn!(tool = %name, args = %args_clone, "Tool call args JSON invalid");
                                        args_clone
                                    }
                                }
                            }
                        };
                        if let Some(entry) = tool_calls.get_mut(&id) {
                            entry.1 = valid_args;
                        }
                        let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallEnd {
                            session_id: session_id.clone(),
                            tool_call_id: id.clone(),
                            output: String::new(),
                            is_error: false,
                        });
                        tracing::info!(tool = %name, args_len, "Tool call collected");
                    }
                }
                StreamEvent::Finish { reason, usage } => {
                    finish_reason = reason;
                    total_input += usage.input_tokens;
                    total_output += usage.output_tokens;
                }
                StreamEvent::Error(e) => {
                    let _ = state.app_handle.emit("agent-event", AgentEvent::Error {
                        session_id: session_id.clone(),
                        error: e.clone(),
                    });
                    return Err(AppError::internal(format!("Stream error: {}", e)));
                }
            }
        }

        let _ = state.db.log_llm_call_complete(&llm_call_id, Some(&text_buf), None, Some(&format!("{:?}", finish_reason)), total_input, total_output, llm_start.elapsed().as_millis() as u64).await;

        if !tool_calls.is_empty() && matches!(finish_reason, FinishReason::ToolCalls) {
            let tool_call_requests: Vec<crate::infrastructure::llm_client::types::ToolCallRequest> = tool_calls.iter().map(|(id, (name, args))| {
                crate::infrastructure::llm_client::types::ToolCallRequest {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                }
            }).collect();

            all_messages.push(Message {
                role: "assistant".to_string(),
                content: text_buf.clone(),
                tool_calls: Some(tool_call_requests.clone()),
                tool_call_id: None,
            });

            let tc_json = serde_json::to_string(&tool_call_requests).unwrap_or_default();
            let _ = state.db.create_message(&session_id, "assistant", &text_buf, Some(&tc_json), None).await;

            let sandbox = state.sandbox.lock().await;
            let pve_validator = crate::infrastructure::sandbox::security::InjectionValidator::new();
            for (id, (name, args_str)) in &tool_calls {
                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);

                let tool_exec_id = state.db.log_tool_execution_start(&session_id, Some(&llm_call_id), name, args_str).await?;
                let tool_start = std::time::Instant::now();

                let result = match pve_validator.validate_tool_args(name, &args) {
                    Ok(()) => execute_tool(name, &args, &project_root, &sandbox).await,
                    Err(e) => {
                        tracing::warn!(tool = %name, error = %e, "PVE injection detected");
                        Err(e)
                    }
                };

                let is_error = result.is_err();
                let result_str = match result {
                    Ok(r) => r,
                    Err(e) => format!("Error: {}", e),
                };
                let tool_duration = tool_start.elapsed().as_millis() as u64;

                let _ = state.db.log_tool_execution_complete(
                    &tool_exec_id,
                    Some(&result_str),
                    is_error,
                    if is_error { Some(&result_str) } else { None },
                    tool_duration,
                    true,
                    None,
                    false,
                ).await;

                let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallEnd {
                    session_id: session_id.clone(),
                    tool_call_id: id.clone(),
                    output: result_str.clone(),
                    is_error,
                });

                all_messages.push(Message {
                    role: "tool".to_string(),
                    content: result_str,
                    tool_calls: None,
                    tool_call_id: Some(id.clone()),
                });
            }

            full_response.clear();
            continue;
        }

        full_response = text_buf;
        break;
    }

    if !full_response.is_empty() {
        let _ = state.db.create_message(&session_id, "assistant", &full_response, None, None).await;
    }

    if let Ok(Some(mut session)) = state.db.get_session(&session_id).await {
        session.input_tokens += total_input;
        session.output_tokens += total_output;
        let _ = state.db.update_session(&session).await;
    }

    let _ = state.app_handle.emit("agent-event", AgentEvent::TurnCompleted {
        session_id: session_id.clone(),
        input_tokens: total_input,
        output_tokens: total_output,
    });

    tracing::info!(response_len = full_response.len(), "Agent response generated");
    Ok(IpcResponse::ok(full_response))
}

#[tauri::command]
pub async fn agent_approve_tool(
    _state: State<'_, AppState>,
    tool_call_id: String,
    approved: bool,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(tool_call_id = %tool_call_id, approved, "Tool approval processed");
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_cancel(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::warn!(session_id = %session_id, "Agent cancelled");
    let flag = get_cancel_flag(&state.agent_states, &session_id).await;
    *flag.write().await = true;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_compact(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::info!(session_id = %session_id, "Compaction triggered");

    let messages = state.db.list_messages(&session_id).await
        .map_err(|e| AppError::internal(format!("Failed to load messages: {}", e)))?;

    if messages.len() <= 10 {
        return Ok(IpcResponse::ok(()));
    }

    let summary = compact_messages_simple(&messages);

    let _ = state.db.create_message(&session_id, "system", &summary, None, None).await;

    let _ = state.app_handle.emit("agent-event", AgentEvent::CompactionTriggered {
        session_id: session_id.clone(),
    });

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_restart(
    state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("Agent restarted");
    let mut states = state.agent_states.lock().await;
    states.clear();
    Ok(IpcResponse::ok(()))
}
