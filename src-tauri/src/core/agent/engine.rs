use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use futures_util::StreamExt;
use rig::agent::{Agent, MultiTurnStreamItem, StreamingError};
use rig::client::CompletionClient;
use rig::completion::{GetTokenUsage, Prompt, Usage};
use rig::streaming::{StreamingPrompt, StreamedAssistantContent, StreamedUserContent};
use tokio::sync::mpsc;

use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::session::MessageMeta;
use crate::infrastructure::llm::registry::ProviderRegistry;
use crate::shared::error::AppError;

use super::approval::ApprovalManager;
use super::subagent::{SubAgentCache, SubAgentExecutor, SubAgentTool, TokenCounter};
use super::tools::todo_tools::TodoWriteTool;
use super::tools::{
    CreateDirectoryTool, EditTool, ListDirectoryTool, MultiEditTool, ReadFileTool, WriteFileTool,
};
use super::types::{ChatEvent, ChatRequest};

const MAX_TOOL_STEPS: usize = 20;
const MAX_CONCURRENT_SUBAGENTS: usize = 3;
const MAX_TOKENS: u64 = 8192;

#[derive(Clone)]
pub struct AgentEngine {
    registry: ProviderRegistry,
    db: Database,
    subagent_cache: Arc<SubAgentCache>,
    token_counter: Arc<TokenCounter>,
}

/// 一次 agent 流式调用的聚合结果,用于持久化到 messages 表。
struct StreamOutcome {
    text: String,
    usage: Usage,
    tool_calls: Vec<serde_json::Value>,
    tool_results: Vec<serde_json::Value>,
}

impl AgentEngine {
    pub fn new(registry: ProviderRegistry, db: Database, _workspace_root: PathBuf) -> Self {
        let cache = Arc::new(SubAgentCache::new(100, 3600));
        let token_counter = Arc::new(TokenCounter::new(1000));

        Self {
            registry,
            db,
            subagent_cache: cache,
            token_counter,
        }
    }

    pub fn subagent_executor(&self, workspace_root: PathBuf) -> SubAgentExecutor<'_> {
        SubAgentExecutor::new(
            &self.registry,
            workspace_root,
            Arc::clone(&self.subagent_cache),
            Arc::clone(&self.token_counter),
            MAX_CONCURRENT_SUBAGENTS,
        )
    }

    pub async fn send_message(
        &self,
        request: ChatRequest,
        workspace_root: PathBuf,
        approval: Arc<ApprovalManager>,
        tx: mpsc::Sender<ChatEvent>,
    ) -> Result<(), AppError> {
        let config = self
            .registry
            .active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();

        let system_prompt = build_system_prompt(&request);
        let user_message = build_user_message(&request);

        let started = Instant::now();
        let outcome = match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval);
                run_agent_stream(agent, &user_message, &tx).await?
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval);
                run_agent_stream(agent, &user_message, &tx).await?
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval);
                run_agent_stream(agent, &user_message, &tx).await?
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();
                let agent = build_agent(client.agent(&model), &system_prompt, workspace_root, approval);
                run_agent_stream(agent, &user_message, &tx).await?
            }
            _ => return Err(AppError::provider_not_found(&provider)),
        };

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let input_tokens = outcome.usage.input_tokens as u32;
        let output_tokens = outcome.usage.output_tokens as u32;

        // 通知前端流结束(携带真实 token 用量)
        let _ = tx
            .send(ChatEvent::Finish {
                input_tokens,
                output_tokens,
            })
            .await;

        // 持久化到 messages 表,供仪表盘聚合统计
        self.persist_messages(
            &request.session_id,
            &request.content,
            &outcome,
            &model,
            &provider,
            input_tokens,
            output_tokens,
            elapsed_ms,
        );

        Ok(())
    }

    /// 写入 user 消息与 assistant 消息(含 LLM 指标)。指标记录为 best-effort,
    /// 失败时记录警告但不阻断主流程——主流程的成败由 LLM 响应本身决定。
    fn persist_messages(
        &self,
        session_id: &str,
        user_content: &str,
        outcome: &StreamOutcome,
        model: &str,
        provider: &str,
        input_tokens: u32,
        output_tokens: u32,
        latency_ms: u64,
    ) {
        let tool_calls_json = if outcome.tool_calls.is_empty() {
            None
        } else {
            serde_json::to_string(&outcome.tool_calls).ok()
        };
        let tool_results_json = if outcome.tool_results.is_empty() {
            None
        } else {
            serde_json::to_string(&outcome.tool_results).ok()
        };

        if let Err(e) = self.db.create_message(session_id, "user", user_content, None, None) {
            tracing::warn!(error = %e, session_id, "Failed to persist user message");
        }

        let meta = MessageMeta {
            thinking_content: None,
            model: Some(model),
            provider: Some(provider),
            input_tokens,
            output_tokens,
            latency_ms: Some(latency_ms),
        };
        if let Err(e) = self.db.create_message_with_meta(
            session_id,
            "assistant",
            &outcome.text,
            tool_calls_json.as_deref(),
            tool_results_json.as_deref(),
            Some(meta),
        ) {
            tracing::warn!(error = %e, session_id, "Failed to persist assistant message");
        }
    }

    pub async fn stop(&self, _session_id: &str) -> Result<(), AppError> {
        Ok(())
    }

    /// 一次性 LLM 调用(非流式、无工具),返回纯文本。
    /// 供雷达扫描等不需要工具链的简单场景使用。
    pub async fn prompt_once(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AppError> {
        let config = self
            .registry
            .active_model_config()
            .ok_or_else(|| AppError::model_not_found("active"))?;

        let provider = config.provider.clone();
        let base_url = config.base_url.clone();
        let api_key = config.api_key.clone();
        let model = config.model.clone();

        match provider.to_lowercase().as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "anthropic" => {
                let client = rig::providers::anthropic::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?;
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            "deepseek" | "agnes" | "openrouter" => {
                let client = rig::providers::openai::Client::builder()
                    .api_key(api_key)
                    .base_url(&base_url)
                    .build()
                    .map_err(|e| AppError::stream_error(e.to_string()))?
                    .completions_api();
                run_simple_prompt(client.agent(&model), system_prompt, user_message).await
            }
            _ => Err(AppError::provider_not_found(&provider)),
        }
    }
}

/// 构造无工具的简单 agent 并执行一次性 prompt,返回纯文本。
async fn run_simple_prompt<M>(
    builder: rig::agent::AgentBuilder<M>,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, AppError>
where
    M: rig::completion::CompletionModel + 'static,
{
    let agent = builder
        .preamble(system_prompt)
        .max_tokens(MAX_TOKENS)
        .build();
    let response = agent
        .prompt(user_message)
        .await
        .map_err(|e| AppError::stream_error(e.to_string()))?;
    tracing::debug!(
        response_len = response.len(),
        response_preview = &response[..response.len().min(500)],
        "prompt_once response received"
    );
    Ok(response)
}

/// 统一构造 agent:注入系统提示、token 上限、工具集。
fn build_agent<M>(
    builder: rig::agent::AgentBuilder<M>,
    system_prompt: &str,
    workspace_root: PathBuf,
    approval: Arc<ApprovalManager>,
) -> Agent<M>
where
    M: rig::completion::CompletionModel + 'static,
{
    builder
        .preamble(system_prompt)
        .max_tokens(MAX_TOKENS)
        .default_max_turns(MAX_TOOL_STEPS)
        .tool(ReadFileTool {
            workspace_root: workspace_root.clone(),
        })
        .tool(ListDirectoryTool {
            workspace_root: workspace_root.clone(),
        })
        .tool(WriteFileTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(CreateDirectoryTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(EditTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(MultiEditTool {
            workspace_root: workspace_root.clone(),
            approval: approval.clone(),
        })
        .tool(TodoWriteTool)
        .tool(SubAgentTool { workspace_root })
        .build()
}

/// 驱动 agent 的流式多轮循环,聚合文本、token 用量与工具调用记录。
async fn run_agent_stream<M>(
    agent: Agent<M>,
    user_message: &str,
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<StreamOutcome, AppError>
where
    M: rig::completion::CompletionModel + 'static,
    <M as rig::completion::CompletionModel>::StreamingResponse: GetTokenUsage + Clone + Unpin,
{
    let stream = agent
        .stream_prompt(user_message.to_string())
        .multi_turn(MAX_TOOL_STEPS)
        .await;
    consume_stream(stream, tx).await
}

/// 消费 rig 的多轮流,把 text delta / 工具调用 / token 用量转发给前端并聚合。
async fn consume_stream<S, R>(
    mut stream: S,
    tx: &mpsc::Sender<ChatEvent>,
) -> Result<StreamOutcome, AppError>
where
    S: futures_util::stream::Stream<Item = Result<MultiTurnStreamItem<R>, StreamingError>> + Unpin,
    R: GetTokenUsage + Clone + Unpin,
{
    let mut text = String::new();
    let mut usage = Usage::new();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    let mut tool_results: Vec<serde_json::Value> = Vec::new();

    while let Some(item) = stream.next().await {
        match item.map_err(|e| AppError::stream_error(e.to_string()))? {
            MultiTurnStreamItem::StreamAssistantItem(content) => match content {
                StreamedAssistantContent::Text(t) => {
                    let delta = t.text.to_string();
                    let _ = tx
                        .send(ChatEvent::TextDelta {
                            content: delta.clone(),
                        })
                        .await;
                    text.push_str(&delta);
                }
                StreamedAssistantContent::ToolCall {
                    tool_call, ..
                } => {
                    let _ = tx
                        .send(ChatEvent::ToolCallStart {
                            id: tool_call.id.clone(),
                            name: tool_call.function.name.clone(),
                        })
                        .await;
                    let _ = tx
                        .send(ChatEvent::ToolCallEnd {
                            id: tool_call.id.clone(),
                        })
                        .await;
                    tool_calls.push(serde_json::json!({
                        "id": tool_call.id,
                        "name": tool_call.function.name,
                        "arguments": tool_call.function.arguments,
                    }));
                }
                StreamedAssistantContent::Final(r) => {
                    let u = r.token_usage();
                    if u.has_values() {
                        usage = u;
                    }
                }
                _ => {}
            },
            MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult {
                tool_result, ..
            }) => {
                tool_results.push(serde_json::json!({
                    "id": tool_result.id,
                    "content": format!("{:?}", tool_result.content),
                }));
            }
            MultiTurnStreamItem::CompletionCall(cc) => {
                if cc.usage.has_values() {
                    usage = cc.usage;
                }
            }
            MultiTurnStreamItem::FinalResponse(fr) => {
                let final_text = fr.response().to_string();
                if !final_text.is_empty() {
                    text = final_text;
                }
                let final_usage = fr.usage();
                if final_usage.has_values() {
                    usage = final_usage;
                }
            }
            // non-exhaustive 枚举:未知变体静默跳过,不阻断流
            _ => {}
        }
    }

    Ok(StreamOutcome {
        text,
        usage,
        tool_calls,
        tool_results,
    })
}

fn build_system_prompt(request: &ChatRequest) -> String {
    let mut prompt = String::from("You are a helpful AI assistant.\n\nYou have access to specialized sub-agents:\n- researcher: Analyze code, find patterns, gather information\n- outliner: Create structured outlines and plans\n- critic: Review quality and suggest improvements\n\nUse the 'subagent' tool to delegate tasks to these specialists.");
    if let Some(ref instructions) = request.custom_instructions {
        prompt.push_str("\n\n");
        prompt.push_str(instructions);
    }
    prompt
}

fn build_user_message(request: &ChatRequest) -> String {
    let mut message = String::new();
    if let Some(ref context) = request.context_text {
        message.push_str("Context:\n");
        message.push_str(context);
        message.push_str("\n\n");
    }
    message.push_str(&request.content);
    message
}
