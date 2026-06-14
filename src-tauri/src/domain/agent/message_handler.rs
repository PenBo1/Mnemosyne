use futures::StreamExt;

use super::loop_core::{AgentLoop, AgentResources};
use super::stream_handler::{StreamAction, StreamState};
use super::types::*;
use crate::errors::AppError;
use crate::infra::llm::{Message, ToolSpec};
use crate::domain::tools::{ToolCall, ToolContext};

pub async fn handle_user_input(
    resources: &AgentResources,
    tx_event: &tokio::sync::mpsc::Sender<AgentEvent>,
    _agent: &mut AgentLoop,
    session_id: &str,
    content: &str,
) {
    tracing::info!(session_id = %session_id, content_len = content.len(), "handle_user_input started");

    let _ = tx_event
        .send(AgentEvent::TurnStarted {
            session_id: session_id.to_string(),
        })
        .await;

    {
        let db = resources.db.lock().await;
        if let Err(e) = db.create_message(session_id, "user", content, None, None) {
            tracing::error!(error = %e, "Failed to save user message");
            let _ = tx_event
                .send(AgentEvent::Error {
                    session_id: session_id.to_string(),
                    error: format!("Failed to save message: {}", e),
                })
                .await;
            return;
        }
        tracing::info!("User message saved to DB");
    }

    let history: Vec<Message> = {
        let db = resources.db.lock().await;
        match db.list_messages(session_id) {
            Ok(msgs) => msgs
                .into_iter()
                .map(|m| Message {
                    role: m.role,
                    content: m.content,
                    tool_calls: m
                        .tool_calls
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    tool_call_id: None,
                })
                .collect(),
            Err(e) => {
                let _ = tx_event
                    .send(AgentEvent::Error {
                        session_id: session_id.to_string(),
                        error: format!("Failed to load history: {}", e),
                    })
                    .await;
                return;
            }
        }
    };

    let system = build_system_prompt(&resources.model);

    let novel_id = {
        let db = resources.db.lock().await;
        db.get_session(session_id)
            .ok()
            .flatten()
            .and_then(|s| s.novel_id)
    };

    let tool_specs: Vec<ToolSpec> = resources.tool_registry.tool_specs();

    let result = run_turn(resources, tx_event, session_id, &system, &history, &tool_specs, novel_id.as_deref()).await;

    match result {
        Ok(usage) => {
            let _ = tx_event
                .send(AgentEvent::TurnCompleted {
                    session_id: session_id.to_string(),
                    input_tokens: usage.input_tokens,
                    output_tokens: usage.output_tokens,
                })
                .await;
        }
        Err(e) => {
            let _ = tx_event
                .send(AgentEvent::Error {
                    session_id: session_id.to_string(),
                    error: e.to_string(),
                })
                .await;
        }
    }
}

fn build_system_prompt(_model: &str) -> String {
    let mut parts = Vec::new();

    parts.push(
        "你是 Mnemosyne，一个专业的 AI 创作助手。你帮助用户进行小说创作、 \
         角色设计、世界观构建、情节分析和趋势研究。\
         你使用提供的工具来读取和写入文件、搜索内容、管理小说数据。\
         请用中文回复。"
            .to_string(),
    );

    parts.push(
        "## 工具使用规则\n\
         - 使用 read_file 读取文件内容\n\
         - 使用 write_file 写入文件\n\
         - 使用 grep 搜索文件内容\n\
         - 使用 glob 查找文件\n\
         - 使用 list_dir 列出目录\n\
         - 使用 novel_info 获取小说信息\n\
         - 使用 chapter_list 获取章节列表"
            .to_string(),
    );

    parts.push(format!("当前时间: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")));

    parts.join("\n\n")
}

async fn run_turn(
    resources: &AgentResources,
    tx_event: &tokio::sync::mpsc::Sender<AgentEvent>,
    session_id: &str,
    system: &str,
    history: &[Message],
    tools: &[ToolSpec],
    novel_id: Option<&str>,
) -> Result<TokenUsage, AppError> {
    let mut current_messages = history.to_vec();
    tracing::info!(model = %resources.model, history_len = current_messages.len(), "run_turn started");

    loop {
        tracing::info!("Calling provider.stream()...");
        let mut stream = resources
            .provider
            .stream(&resources.model, system, &current_messages, tools)
            .await?;
        tracing::info!("Stream started, processing events...");

        let mut stream_state = StreamState::new();

        while let Some(event) = stream.next().await {
            match stream_state.process_event(event) {
                StreamAction::Delta(content) => {
                    if !content.is_empty() {
                        let _ = tx_event
                            .send(AgentEvent::StreamDelta {
                                session_id: session_id.to_string(),
                                content,
                            })
                            .await;
                    }
                }
                StreamAction::ToolCallBegin {
                    tool_call_id,
                    tool,
                    args,
                } => {
                    let _ = tx_event
                        .send(AgentEvent::ToolCallBegin {
                            session_id: session_id.to_string(),
                            tool_call_id,
                            tool,
                            args,
                        })
                        .await;
                }
                StreamAction::Finish { tool_calls_empty } => {
                    if tool_calls_empty {
                        let db = resources.db.lock().await;
                        let _ = db.create_message(
                            session_id,
                            "assistant",
                            &stream_state.assistant_content,
                            None,
                            None,
                        );

                        return Ok(TokenUsage {
                            input_tokens: stream_state.total_input,
                            output_tokens: stream_state.total_output,
                        });
                    }

                    let tc_json = stream_state.build_tool_call_json();
                    let db = resources.db.lock().await;
                    let _ = db.create_message(
                        session_id,
                        "assistant",
                        &stream_state.assistant_content,
                        Some(&tc_json),
                        None,
                    );

                    let mut tool_results = Vec::new();
                    for (tool_id, tool_name, tool_args_str) in &stream_state.tool_calls {
                        let args: serde_json::Value =
                            serde_json::from_str(tool_args_str).unwrap_or_default();

                        let tool_call = ToolCall {
                            id: tool_id.clone(),
                            name: tool_name.clone(),
                            args,
                            session_id: session_id.to_string(),
                        };

                        let ctx = ToolContext {
                            session_id: session_id.to_string(),
                            work_dir: resources.work_dir.clone(),
                            novel_id: novel_id.map(|s| s.to_string()),
                            sandbox: Some(resources.sandbox.clone()),
                        };

                        let result = resources.tool_registry.execute(&tool_call, &ctx);

                        let (output, is_error) = match result {
                            Ok(out) => (out.content, out.is_error),
                            Err(e) => (e.to_string(), true),
                        };

                        let _ = tx_event
                            .send(AgentEvent::ToolCallEnd {
                                session_id: session_id.to_string(),
                                tool_call_id: tool_id.clone(),
                                output: output.clone(),
                                is_error,
                            })
                            .await;

                        let db = resources.db.lock().await;
                        let _ = db.create_message(
                            session_id,
                            "tool",
                            &output,
                            None,
                            Some(tool_id),
                        );

                        tool_results.push(Message {
                            role: "tool".into(),
                            content: output,
                            tool_calls: None,
                            tool_call_id: Some(tool_id.clone()),
                        });
                    }

                    current_messages.push(Message {
                        role: "assistant".into(),
                        content: stream_state.assistant_content.clone(),
                        tool_calls: Some(
                            stream_state.tool_calls
                                .iter()
                                .map(|(id, name, args)| {
                                    crate::infra::llm::ToolCallRequest {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: args.clone(),
                                    }
                                })
                                .collect(),
                        ),
                        tool_call_id: None,
                    });
                    current_messages.extend(tool_results);

                    break;
                }
                StreamAction::Error(e) => {
                    return Err(AppError::internal(e));
                }
            }
        }
    }
}
