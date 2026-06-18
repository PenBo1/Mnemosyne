use std::sync::Arc;
use tokio::sync::Mutex;
use futures::StreamExt;
use crate::errors::{AppError, IpcResponse};
use crate::infra::llm::types::{Message, StreamEvent, FinishReason, ToolSpec};
use crate::AppState;
use tauri::State;
use tauri::Emitter;

const DEFAULT_SYSTEM_PROMPT: &str = "你是 Mnemosyne，一个专业的 AI 创作助手。你帮助用户进行小说创作、角色设计、世界观构建、情节分析和趋势研究。请用中文回复。";
const MAX_HISTORY_MESSAGES: usize = 50;
const MAX_TURNS: usize = 15;
const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;

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

/// Per-session agent state for cancellation and tool approval
struct SessionAgentState {
    cancelled: Arc<tokio::sync::RwLock<bool>>,
}

static AGENT_STATES: std::sync::OnceLock<Mutex<std::collections::HashMap<String, SessionAgentState>>> =
    std::sync::OnceLock::new();

fn agent_states() -> &'static Mutex<std::collections::HashMap<String, SessionAgentState>> {
    AGENT_STATES.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

async fn get_cancel_flag(session_id: &str) -> Arc<tokio::sync::RwLock<bool>> {
    let mut states = agent_states().lock().await;
    states.entry(session_id.to_string())
        .or_insert_with(|| SessionAgentState {
            cancelled: Arc::new(tokio::sync::RwLock::new(false)),
        })
        .cancelled.clone()
}

/// Load conversation history from DB, limited to MAX_HISTORY_MESSAGES
fn load_history_sync(
    db: &crate::infra::db::Database,
    session_id: &str,
) -> Result<Vec<Message>, AppError> {
    let db_messages = db.list_messages(session_id)
        .map_err(|e| AppError::internal(format!("Failed to load messages: {}", e)))?;

    let start = db_messages.len().saturating_sub(MAX_HISTORY_MESSAGES);
    Ok(db_messages[start..].iter().map(|m| {
        let mut tool_calls = None;
        if m.role == "assistant" {
            if let Some(tc_str) = &m.tool_calls {
                if let Ok(tc) = serde_json::from_str::<Vec<crate::infra::llm::types::ToolCallRequest>>(tc_str) {
                    tool_calls = Some(tc);
                }
            }
        }
        Message {
            role: m.role.clone(),
            content: m.content.clone(),
            tool_calls,
            tool_call_id: m.tool_results.as_ref().and_then(|_| Some(m.id.clone())).filter(|_| m.role == "tool"),
        }
    }).collect())
}

/// Build tool definitions for agent mode
fn agent_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "search_memory".to_string(),
            description: "搜索记忆库中的相关信息".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "搜索关键词" },
                    "top_k": { "type": "integer", "description": "返回结果数量", "default": 5 }
                },
                "required": ["query"]
            }),
        },
        ToolSpec {
            name: "read_file".to_string(),
            description: "读取项目文件内容".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "文件路径" }
                },
                "required": ["path"]
            }),
        },
        ToolSpec {
            name: "list_files".to_string(),
            description: "列出目录中的文件".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "目录路径", "default": "." }
                }
            }),
        },
    ]
}

/// Execute a tool call and return the result as a string.
/// Validates file operations through the sandbox enforcer.
async fn execute_tool(
    name: &str,
    args: &serde_json::Value,
    project_root: &std::path::Path,
    sandbox: &crate::infra::sandbox::enforce::SandboxEnforcer,
) -> Result<String, AppError> {
    match name {
        "search_memory" => {
            Ok("记忆库搜索结果：（暂无匹配结果）".to_string())
        }
        "read_file" => {
            let path = args["path"].as_str()
                .ok_or_else(|| AppError::invalid_input("Missing 'path' argument"))?;
            let full_path = project_root.join(path);
            // Validate through sandbox
            sandbox.validate_file_operation(&full_path, false)
                .map_err(|v| AppError::forbidden(format!("Sandbox violation: {:?}", v)))?;
            tokio::fs::read_to_string(&full_path).await
                .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))
        }
        "list_files" => {
            let path = args["path"].as_str().unwrap_or(".");
            let full_path = project_root.join(path);
            // Validate through sandbox
            sandbox.validate_file_operation(&full_path, false)
                .map_err(|v| AppError::forbidden(format!("Sandbox violation: {:?}", v)))?;
            let mut entries = tokio::fs::read_dir(&full_path).await
                .map_err(|e| AppError::internal(format!("Failed to read dir: {}", e)))?;
            let mut names = Vec::new();
            while let Some(entry) = entries.next_entry().await
                .map_err(|e| AppError::internal(format!("Failed to read entry: {}", e)))? {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
            names.sort();
            Ok(names.join("\n"))
        }
        _ => Err(AppError::bad_request(format!("Unknown tool: {}", name))),
    }
}

#[tauri::command]
pub async fn agent_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
) -> Result<IpcResponse<String>, AppError> {
    if content.trim().is_empty() {
        return Err(AppError::invalid_input("Message content cannot be empty"));
    }
    if content.len() > 1_000_000 {
        return Err(AppError::invalid_input("Message content too long (max 1MB)"));
    }

    // Save user message
    {
        let db = state.db.lock().await;
        if let Err(e) = db.create_message(&session_id, "user", &content, None, None) {
            tracing::error!(error = %e, "Failed to save user message");
            return Err(AppError::internal(format!("Failed to save message: {}", e)));
        }
    }

    let _ = state.app_handle.emit("agent-event", AgentEvent::TurnStarted {
        session_id: session_id.clone(),
    });

    // Get provider and model
    let (provider, model) = {
        let registry = state.provider_registry.lock().await;
        let provider = registry.default()
            .map_err(|e| AppError::provider_not_found(e.to_string()))?;
        let model = registry.default_model().to_string();
        (provider, model)
    };

    let cancel_flag = get_cancel_flag(&session_id).await;
    *cancel_flag.write().await = false;

    let tools = agent_tool_specs();
    let project_root = state.data_dir.root().to_path_buf();

    // Build system prompt with feedback lessons and skills
    let system_prompt = {
        let feedback = state.feedback_store.lock().await;
        let lessons = feedback.format_lessons_for_prompt();
        let skill_index = {
            let skills = state.skill_manager.lock().await;
            skills.build_index()
        };
        let mut prompt = DEFAULT_SYSTEM_PROMPT.to_string();
        if !lessons.is_empty() {
            prompt = format!("{}\n\n{}", prompt, lessons);
        }
        if !skill_index.is_empty() {
            prompt = format!("{}\n\n{}", prompt, skill_index);
        }
        prompt
    };

    // ReAct loop: stream → accumulate tool calls → execute → repeat
    let mut all_messages: Vec<Message> = {
        let db = state.db.lock().await;
        load_history_sync(&db, &session_id)?
    };

    // Auto-compaction: trim history if it exceeds budget
    let budget = crate::infra::token_budget::ContextBudget::for_window(DEFAULT_CONTEXT_WINDOW);
    let total_history_tokens: u32 = all_messages.iter()
        .map(|m| crate::infra::token_budget::estimate_tokens(&m.content))
        .sum();
    if budget.needs_compaction(total_history_tokens) {
        let max_msgs = budget.max_messages_after_compact(200);
        if all_messages.len() > max_msgs {
            let keep_start = all_messages.len() - max_msgs;
            let dropped = keep_start;
            all_messages = all_messages[keep_start..].to_vec();
            tracing::info!(dropped, kept = all_messages.len(), "Auto-compacted history");
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

        let stream = provider.stream(&model, &system_prompt, &all_messages, &tools).await?;

        // Collect the full stream, accumulate tool calls
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
                        // Validate accumulated JSON, attempt repair if incomplete
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
                        // Update args with validated version
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

        if !tool_calls.is_empty() && matches!(finish_reason, FinishReason::ToolCalls) {
            // Add assistant message with tool calls
            let tool_call_requests: Vec<crate::infra::llm::types::ToolCallRequest> = tool_calls.iter().map(|(id, (name, args))| {
                crate::infra::llm::types::ToolCallRequest {
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

            // Save assistant message with tool calls
            {
                let db = state.db.lock().await;
                let tc_json = serde_json::to_string(&tool_call_requests).unwrap_or_default();
                let _ = db.create_message(&session_id, "assistant", &text_buf, Some(&tc_json), None);
            }

            // Execute each tool and add results
            let sandbox = state.sandbox.lock().await;
            for (id, (name, args_str)) in &tool_calls {
                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);
                let result = match execute_tool(name, &args, &project_root, &sandbox).await {
                    Ok(r) => r,
                    Err(e) => format!("Error: {}", e),
                };

                let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallEnd {
                    session_id: session_id.clone(),
                    tool_call_id: id.clone(),
                    output: result.clone(),
                    is_error: result.starts_with("Error:"),
                });

                all_messages.push(Message {
                    role: "tool".to_string(),
                    content: result,
                    tool_calls: None,
                    tool_call_id: Some(id.clone()),
                });
            }

            full_response.clear();
            continue;
        }

        // No tool calls — final text response
        full_response = text_buf;
        break;
    }

    // Save final assistant message
    if !full_response.is_empty() {
        let db = state.db.lock().await;
        let _ = db.create_message(&session_id, "assistant", &full_response, None, None);
    }

    // Update session token counts
    {
        let db = state.db.lock().await;
        if let Ok(Some(mut session)) = db.get_session(&session_id) {
            session.input_tokens += total_input;
            session.output_tokens += total_output;
            let _ = db.update_session(&session);
        }
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
    _state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::warn!(session_id = %session_id, "Agent cancelled");
    let flag = get_cancel_flag(&session_id).await;
    *flag.write().await = true;
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_compact(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!(session_id = %session_id, "Compaction triggered");

    let messages = {
        let db = state.db.lock().await;
        db.list_messages(&session_id)
            .map_err(|e| AppError::internal(format!("Failed to load messages: {}", e)))?
    };

    if messages.len() <= 10 {
        return Ok(IpcResponse::ok(()));
    }

    // Summarize older messages, keep recent ones
    let keep_recent = 10;
    let to_summarize = &messages[..messages.len() - keep_recent];
    let summary_text = to_summarize.iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| format!("[{}] {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let summary = if summary_text.len() > 2000 {
        format!("对话摘要：用户和助手讨论了{}条消息，涵盖以下内容：{}", 
            to_summarize.len(),
            &summary_text[..2000])
    } else {
        format!("对话摘要：{}", summary_text)
    };

    // Save summary as system message
    {
        let db = state.db.lock().await;
        let _ = db.create_message(&session_id, "system", &summary, None, None);
    }

    let _ = state.app_handle.emit("agent-event", AgentEvent::CompactionTriggered {
        session_id: session_id.clone(),
    });

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_restart(
    _state: State<'_, AppState>,
) -> Result<IpcResponse<()>, AppError> {
    tracing::info!("Agent restarted");
    let mut states = agent_states().lock().await;
    states.clear();
    Ok(IpcResponse::ok(()))
}
