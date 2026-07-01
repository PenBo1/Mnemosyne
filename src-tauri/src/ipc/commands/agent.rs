use std::sync::Arc;
use futures::StreamExt;
use crate::shared::errors::{AppError, IpcResponse};
use crate::infrastructure::llm_client::types::{Message, StreamEvent, FinishReason, ToolCallRequest, ToolSpec};
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::core::agent::chat_loop::{
    build_system_prompt, load_history,
    compact_history, compact_messages_simple,
};
use crate::core::agent::context_compressor::{
    ContextCompressor, CompressorConfig, CompressibleMessage, ToolCallRef,
};
use crate::core::agent::attachments::{AttachmentSpec, resolve_attachments, format_attachments_context};
use crate::core::agent::base::ToolRegistry;
use crate::core::agent::main_agent::{SafetyGate, RiskLevel, ConfirmationResponse};
use crate::core::state::AgentSessionState;
use crate::AppState;
use tauri::State;
use tauri::Emitter;

/// 单次 agent_send_message 的最大 ReAct 轮数。
///
/// 合并 main agent 能力后，chat agent 需要处理多步骤任务（如循环写 100 章），
/// 15 轮太少（写 5 章就到上限）。提升到 30 与 main agent 对齐。
const MAX_TURNS: usize = 30;
const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    TurnStarted { session_id: String },
    StreamDelta { session_id: String, content: String },
    /// 模型推理过程增量（reasoning_content / thinking_delta），与正文分离。
    ReasoningDelta { session_id: String, content: String },
    ToolCallBegin { session_id: String, tool_call_id: String, tool: String, args: String },
    /// 工具调用参数增量（streaming args）。
    /// 与 StreamDelta/ReasoningDelta 同范式：前端累积 args_delta 还原完整 args。
    ToolCallDelta { session_id: String, tool_call_id: String, args_delta: String },
    ToolCallEnd { session_id: String, tool_call_id: String, output: String, is_error: bool },
    /// SafetyGate 触发的用户确认请求。
    /// 高风险操作（create_novel/write_next_chapter/spawn_subagent/写文件/bash）
    /// 会先 emit 此事件，等待用户通过 `agent_respond_confirmation` 响应后才执行。
    ConfirmationRequired {
        session_id: String,
        step_id: u32,
        tool_call_id: String,
        tool: String,
        description: String,
        details: String,
        risk_level: String,
    },
    TurnCompleted { session_id: String, input_tokens: u32, output_tokens: u32 },
    Error { session_id: String, error: String },
    CompactionTriggered { session_id: String },
}

/// 获取或创建 session 的完整状态引用。
///
/// 首次调用时建立 confirmation channel，整个 session 生命周期复用。
/// 返回的 `AgentSessionRefs` 包含所有需要的字段 clone（Arc/Sender 都是廉价的引用克隆）。
async fn ensure_agent_session(
    agent_states: &tokio::sync::Mutex<std::collections::HashMap<String, AgentSessionState>>,
    session_id: &str,
) -> AgentSessionRefs {
    let mut states = agent_states.lock().await;
    let entry = states.entry(session_id.to_string())
        .or_insert_with(|| {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            AgentSessionState {
                cancelled: Arc::new(tokio::sync::RwLock::new(false)),
                confirmation_tx: tx,
                confirmation_rx: Arc::new(tokio::sync::Mutex::new(rx)),
                auto_approved_tools: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
            }
        });
    AgentSessionRefs {
        cancelled: entry.cancelled.clone(),
        confirmation_tx: entry.confirmation_tx.clone(),
        confirmation_rx: entry.confirmation_rx.clone(),
        auto_approved_tools: entry.auto_approved_tools.clone(),
    }
}

/// session 状态的引用集合（所有字段都是 Arc/Sender 的廉价克隆）。
#[derive(Clone)]
struct AgentSessionRefs {
    cancelled: Arc<tokio::sync::RwLock<bool>>,
    confirmation_tx: tokio::sync::mpsc::UnboundedSender<ConfirmationResponse>,
    confirmation_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<ConfirmationResponse>>>,
    auto_approved_tools: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
}

#[tauri::command]
pub async fn agent_send_message(
    state: State<'_, AppState>,
    session_id: String,
    content: String,
    attachments: Option<Vec<AttachmentSpec>>,
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

    let session_refs = ensure_agent_session(&state.agent_states, &session_id).await;
    *session_refs.cancelled.write().await = false;

    let project_root = state.data_dir.root().join("workspace");
    let _ = std::fs::create_dir_all(&project_root);

    // 解析 session 绑定的 workspace：同时提取 path（附件解析用）和 id（小说工具注册用）。
    // 失败 fail-soft：workspace 不存在时返回 None，agent 仍可运行基础工具。
    let (workspace_path, workspace_id): (Option<std::path::PathBuf>, Option<String>) =
        match state.db.get_session(&session_id).await? {
            Some(session) => match session.workspace_id {
                Some(wid) if !wid.is_empty() => match state.db.get_workspace(&wid).await? {
                    Some(ws) if !ws.path.is_empty() =>
                        (Some(std::path::PathBuf::from(ws.path)), Some(wid)),
                    _ => (None, Some(wid)),
                },
                _ => (None, None),
            },
            None => (None, None),
        };

    // 构造 ToolRegistry：基础文件/记忆工具 + spawn_subagent + 小说工具（需绑定 workspace）。
    // 所有工具走同一条执行路径，SafetyGate 在执行前统一拦截高风险调用。
    let tool_registry = build_chat_tool_registry(
        &state,
        workspace_id.as_deref(),
        &session_id,
        provider.clone(),
        model.clone(),
    ).await;
    let tools: Vec<ToolSpec> = tool_registry.definitions().into_iter()
        .map(|d| ToolSpec {
            name: d.name,
            description: d.description,
            parameters: d.parameters,
        })
        .collect();

    let system_prompt = {
        let feedback = state.feedback_store.lock().await;
        let skills = state.skill_manager.lock().await;
        let mut prompt = build_system_prompt(&feedback, &skills);
        if let Some(atts) = &attachments {
            if !atts.is_empty() {
                let resolved = resolve_attachments(&state.db, atts, workspace_path.as_deref()).await;
                if !resolved.is_empty() {
                    prompt.push_str(&format_attachments_context(&resolved));
                }
            }
        }
        prompt
    };

    let mut all_messages: Vec<Message> = {
        load_history(&state.db, &session_id).await?
    };

    let budget = crate::infrastructure::ai_services::token_budget::ContextBudget::for_window(DEFAULT_CONTEXT_WINDOW);
    let total_history_tokens: u32 = all_messages.iter()
        .map(|m| crate::infrastructure::ai_services::token_budget::estimate_tokens(&m.content))
        .sum();
    if budget.needs_compaction(total_history_tokens) {
        // 升级：用 LLM 摘要中间消息（保留最近 N 条），而非粗暴丢旧消息。
        // 失败时显式降级到 compact_history 并打 warning，不静默吞错。
        let compressible: Vec<CompressibleMessage> = all_messages.iter().map(|m| CompressibleMessage {
            role: m.role.clone(),
            content: m.content.clone(),
            tool_calls: m.tool_calls.as_ref().map(|tcs| tcs.iter().map(|tc| ToolCallRef {
                id: tc.id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            }).collect()),
            tool_call_id: m.tool_call_id.clone(),
        }).collect();

        let mut compressor = ContextCompressor::new(CompressorConfig::default());
        match compressor.compress(&compressible, provider.as_ref(), &model, DEFAULT_CONTEXT_WINDOW as usize).await {
            Ok(compressed) => {
                all_messages = compressed.iter().map(|m| Message {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    tool_calls: m.tool_calls.as_ref().map(|tcs| tcs.iter().map(|tc| ToolCallRequest {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    }).collect()),
                    tool_call_id: m.tool_call_id.clone(),
                }).collect();
                let _ = state.app_handle.emit("agent-event", AgentEvent::CompactionTriggered {
                    session_id: session_id.clone(),
                });
                tracing::info!(
                    session_id = %session_id,
                    original = compressible.len(),
                    compressed = all_messages.len(),
                    "Context compressed via LLM summary"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    session_id = %session_id,
                    "Context compression failed, falling back to simple truncation"
                );
                let max_msgs = budget.max_messages_after_compact(200);
                if compact_history(&mut all_messages, max_msgs) {
                    let _ = state.app_handle.emit("agent-event", AgentEvent::CompactionTriggered {
                        session_id: session_id.clone(),
                    });
                }
            }
        }
    }

    let mut full_response = String::new();
    let mut last_reasoning = String::new();
    let mut total_input: u32 = 0;
    let mut total_output: u32 = 0;

    for _turn in 0..MAX_TURNS {
        if *session_refs.cancelled.read().await {
            tracing::warn!(session_id = %session_id, "Agent turn cancelled");
            break;
        }

        let messages_json = serde_json::to_string(&all_messages).unwrap_or_default();
        let tools_json = serde_json::to_string(&tools).unwrap_or_default();
        let llm_call_id = state.db.log_llm_call_start(&session_id, "chat", &model, "default", Some(&system_prompt), &messages_json, Some(&tools_json), None, None).await?;
        let llm_start = std::time::Instant::now();

        let stream = provider.stream(&model, &system_prompt, &all_messages, &tools).await?;

        let mut text_buf = String::new();
        // 当前 turn 的 reasoning 累积。turn 结束后写入 DB 并清空。
        let mut reasoning_buf = String::new();
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
                StreamEvent::ReasoningDelta { content } => {
                    reasoning_buf.push_str(&content);
                    let _ = state.app_handle.emit("agent-event", AgentEvent::ReasoningDelta {
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
                    // 转发增量 args 给前端，让 UI 可以流式渲染工具调用参数
                    let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallDelta {
                        session_id: session_id.clone(),
                        tool_call_id: id,
                        args_delta,
                    });
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
                        // 不在此处 emit ToolCallEnd —— 实际执行后的 emit (line ~320) 才携带真实 output。
                        // 此处双 emit 会让前端先看到"空成功"再被"真实失败/成功"覆盖，语义矛盾。
                        tracing::info!(tool = %name, args_len, "Tool call args collected");
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
            let reasoning_for_meta = if reasoning_buf.is_empty() { None } else { Some(reasoning_buf.as_str()) };
            let _ = state.db.create_message_with_meta(
                &session_id, "assistant", &text_buf, Some(&tc_json), None,
                Some(crate::infrastructure::db::MessageMeta {
                    thinking_content: reasoning_for_meta,
                    model: Some(&model),
                    provider: Some(provider.name()),
                    input_tokens: total_input,
                    output_tokens: total_output,
                    latency_ms: Some(llm_start.elapsed().as_millis() as u64),
                }),
            ).await;
            // 已写入 DB，下一轮重新累积
            reasoning_buf.clear();

            // ── 工具执行循环（接入 SafetyGate）─────────────────────────
            //
            // 流程：
            // 1. SafetyGate::evaluate_risk 评估每个工具调用的风险等级
            // 2. Safe：直接执行
            // 3. High/Moderate：检查 auto_approved_tools
            //    - 已在 auto_approved_tools：直接执行（首次确认后的自动模式）
            //    - 不在：emit ConfirmationRequired，等待 agent_respond_confirmation 响应
            //      - Approved/ApprovedAuto：执行（ApprovedAuto 同时加入 auto_approved_tools）
            //      - Rejected：把 [User rejected] 作为 tool 结果
            //      - Modified(args)：用修改后的 args 执行
            // 4. PVE 注入验证 + ToolRegistry.execute() 统一执行
            let pve_validator = crate::infrastructure::sandbox::security::InjectionValidator::new();
            let mut turn_step_id: u32 = 0;

            for (id, (name, args_str)) in &tool_calls {
                turn_step_id += 1;
                let args: serde_json::Value = serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);

                // ── SafetyGate 评估 + 确认流程 ──
                let risk = SafetyGate::evaluate_risk(name, &args);
                let args_to_run: serde_json::Value = match risk {
                    RiskLevel::Safe => args.clone(),
                    RiskLevel::High | RiskLevel::Moderate => {
                        // 检查是否已自动批准
                        let auto_approved = session_refs.auto_approved_tools.read().await;
                        if auto_approved.contains(name) {
                            tracing::info!(tool = %name, risk = ?risk, "Auto-approved (prior confirmation)");
                            args.clone()
                        } else {
                            drop(auto_approved);
                            // emit ConfirmationRequired 并等待响应
                            let request = SafetyGate::create_confirmation_request(turn_step_id, name, &args);
                            let _ = state.app_handle.emit("agent-event", AgentEvent::ConfirmationRequired {
                                session_id: session_id.clone(),
                                step_id: request.step_id,
                                tool_call_id: id.clone(),
                                tool: name.clone(),
                                description: request.description,
                                details: request.details,
                                risk_level: format!("{:?}", request.risk_level),
                            });

                            let response = {
                                let mut rx = session_refs.confirmation_rx.lock().await;
                                rx.recv().await.unwrap_or(ConfirmationResponse::Rejected)
                            };

                            match response {
                                ConfirmationResponse::Approved => args.clone(),
                                ConfirmationResponse::ApprovedAuto => {
                                    // 加入 auto_approved_tools，后续同名工具自动通过
                                    session_refs.auto_approved_tools.write().await.insert(name.clone());
                                    tracing::info!(tool = %name, "Added to auto-approved set");
                                    args.clone()
                                }
                                ConfirmationResponse::Rejected => {
                                    // 用户拒绝：把拒绝信息作为 tool 结果
                                    let result_str = "[User rejected this tool call]".to_string();
                                    let tool_exec_id = state.db.log_tool_execution_start(
                                        &session_id, Some(&llm_call_id), name, args_str,
                                    ).await?;
                                    let _ = state.db.log_tool_execution_complete(
                                        &tool_exec_id, Some(&result_str), true,
                                        Some(&result_str), 0, true, None, false,
                                    ).await;
                                    let _ = state.app_handle.emit("agent-event", AgentEvent::ToolCallEnd {
                                        session_id: session_id.clone(),
                                        tool_call_id: id.clone(),
                                        output: result_str.clone(),
                                        is_error: true,
                                    });
                                    all_messages.push(Message {
                                        role: "tool".to_string(),
                                        content: result_str,
                                        tool_calls: None,
                                        tool_call_id: Some(id.clone()),
                                    });
                                    continue;
                                }
                                ConfirmationResponse::Modified(new_args) => {
                                    serde_json::from_str(&new_args).unwrap_or(args.clone())
                                }
                            }
                        }
                    }
                };

                // ── 执行工具（PVE 验证 + ToolRegistry.execute）──
                let tool_exec_id = state.db.log_tool_execution_start(
                    &session_id, Some(&llm_call_id), name, args_str,
                ).await?;
                let tool_start = std::time::Instant::now();

                let result = match pve_validator.validate_tool_args(name, &args_to_run) {
                    Ok(()) => match tool_registry.execute(name, args_to_run).await {
                        Ok(tool_result) => Ok(tool_result.content),
                        Err(e) => Err(e),
                    },
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
        last_reasoning = reasoning_buf;
        break;
    }

    if !full_response.is_empty() {
        let reasoning_for_meta = if last_reasoning.is_empty() { None } else { Some(last_reasoning.as_str()) };
        let _ = state.db.create_message_with_meta(
            &session_id, "assistant", &full_response, None, None,
            Some(crate::infrastructure::db::MessageMeta {
                thinking_content: reasoning_for_meta,
                model: Some(&model),
                provider: Some(provider.name()),
                input_tokens: total_input,
                output_tokens: total_output,
                latency_ms: None,
            }),
        ).await;
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
    _tool_call_id: String,
    _approved: bool,
) -> Result<IpcResponse<()>, AppError> {
    // 已被 agent_respond_confirmation 取代，保留 stub 避免破坏旧前端调用。
    // 新前端应改用 agent_respond_confirmation（支持 auto_approve_similar + modified_args）。
    tracing::warn!("agent_approve_tool is deprecated, use agent_respond_confirmation");
    Ok(IpcResponse::ok(()))
}

/// 响应 SafetyGate 的确认请求。
///
/// 用户在前端看到 `AgentEvent::ConfirmationRequired` 事件后，通过本命令响应：
/// - `approved=true` + `auto_approve_similar=true`：批准 + 后续同名工具自动通过
/// - `approved=true` + `auto_approve_similar=false`：仅批准本次
/// - `modified_args=Some(...)`：用修改后的参数执行（approved 自动设为 true）
/// - 否则：拒绝
#[tauri::command]
pub async fn agent_respond_confirmation(
    state: State<'_, AppState>,
    session_id: String,
    _tool_call_id: String,
    approved: bool,
    auto_approve_similar: bool,
    modified_args: Option<String>,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;

    let response = if let Some(args) = modified_args {
        ConfirmationResponse::Modified(args)
    } else if approved && auto_approve_similar {
        ConfirmationResponse::ApprovedAuto
    } else if approved {
        ConfirmationResponse::Approved
    } else {
        ConfirmationResponse::Rejected
    };

    let session_refs = ensure_agent_session(&state.agent_states, &session_id).await;
    session_refs.confirmation_tx.send(response)
        .map_err(|_| AppError::internal("Agent not waiting for confirmation"))?;

    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn agent_cancel(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<IpcResponse<()>, AppError> {
    validate_id_component(&session_id, "session_id")?;
    tracing::warn!(session_id = %session_id, "Agent cancelled");
    let session_refs = ensure_agent_session(&state.agent_states, &session_id).await;
    *session_refs.cancelled.write().await = true;
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

/// 构造 chat agent 的完整 ToolRegistry。
///
/// 合并 main agent 能力后，chat agent 的工具集包含：
/// 1. 基础文件/记忆工具：read_file / write_file / list_files / bash / search_memory
/// 2. 子 Agent 协作：spawn_subagent（绑定当前 session_id 作为 parent_thread_id）
/// 3. 小说创作（仅当 workspace_id 存在）：create_novel / write_next_chapter / get_novel_status
///
/// 所有工具走同一条执行路径（ToolRegistry.execute），
/// SafetyGate 在执行前统一拦截高风险调用。
async fn build_chat_tool_registry(
    state: &AppState,
    workspace_id: Option<&str>,
    session_id: &str,
    provider: Arc<dyn crate::infrastructure::llm_client::Provider>,
    model: String,
) -> ToolRegistry {
    use crate::core::agent::tools::{ReadFileTool, WriteFileTool, ListFilesTool, BashTool, SearchMemoryTool};

    let work_dir = state.data_dir.root().join("workspace");
    let memory = Arc::new(tokio::sync::RwLock::new(
        crate::core::agent::MemorySystem::new(20)
    ));
    let mut tools = ToolRegistry::new();

    // 基础文件/记忆工具
    tools.register("read_file", Box::new(ReadFileTool::new(work_dir.clone())));
    tools.register("write_file", Box::new(WriteFileTool::new(work_dir.clone())));
    tools.register("list_files", Box::new(ListFilesTool::new(work_dir.clone())));
    tools.register("bash", Box::new(BashTool::new(work_dir.clone(), None)));
    tools.register("search_memory", Box::new(SearchMemoryTool::new(memory.clone())));

    // spawn_subagent：让 chat agent 能自主 spawn 子 Agent
    use crate::core::agent::sub_agent::{ParentAgentRefs, SpawnAgentTool};
    let parent_refs = ParentAgentRefs {
        parent_thread_id: session_id.to_string(),
        provider: provider.clone(),
        model: model.clone(),
        project_root: work_dir.clone(),
        book_id: None,
        memory: memory.clone(),
        skill_manager: None,
        user_profile: None,
    };
    tools.register("spawn_subagent", Box::new(SpawnAgentTool::new(
        state.sub_agent_control.clone(),
        parent_refs,
    )));

    // 小说创作工具：仅当 chat agent 绑定了 workspace 时注册
    // 让用户能通过对话自主创建小说、循环写章节、查询进度
    if let Some(ws_id) = workspace_id {
        use crate::core::agent::main_agent::NovelToolDeps;
        // S9: 从 registry 获取 per-agent 路由
        let (model_overrides, agent_providers) = {
            let registry = state.provider_registry.lock().await;
            registry.build_agent_routing()
        };
        let deps = NovelToolDeps {
            provider: provider.clone(),
            model: model.clone(),
            memory_store: state.memory_store.clone(),
            data_dir: state.data_dir.clone(),
            db: state.db.clone(),
            workspace_id: ws_id.to_string(),
            model_overrides,
            agent_providers,
        };
        tools.register("create_novel",
            Box::new(crate::core::agent::main_agent::NovelCreateTool::new(deps.clone())));
        tools.register("write_next_chapter",
            Box::new(crate::core::agent::main_agent::WriteNextChapterTool::new(deps.clone())));
        tools.register("get_novel_status",
            Box::new(crate::core::agent::main_agent::GetNovelStatusTool::new(deps)));
    }

    tools
}
