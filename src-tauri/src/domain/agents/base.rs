use async_trait::async_trait;
use crate::errors::AppError;
use crate::infra::llm::{Provider, types::Message};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use super::types::{AgentRole, LlmResponse};

// ── Tool System (P14.06 Tool Use) ──────────────────────────────────────────

/// A tool definition that an agent can invoke
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool execution (P14.01 Observation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

/// Tool registry for an agent (P14.06)
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, executor: Box<dyn ToolExecutor>) {
        self.tools.insert(name.into(), executor);
    }

    pub async fn execute(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<ToolResult, AppError> {
        let executor = self.tools.get(name).ok_or_else(|| {
            AppError::bad_request(format!("Unknown tool: {}", name))
        })?;
        executor.execute(args).await
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .map(|(name, exec)| exec.definition(name))
            .collect()
    }
}

/// Trait for executable tools
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn definition(&self, name: &str) -> ToolDefinition;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, AppError>;
}

// ── Memory System (P14.07 MemGPT) ──────────────────────────────────────────

/// A memory entry stored in the archival store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub entry_type: MemoryType,
    pub chapter: Option<u32>,
    pub timestamp: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MemoryType {
    Character,
    Plot,
    Setting,
    Dialogue,
    Fact,
    Style,
}

/// Two-layer memory system (P14.07 MemGPT)
pub struct MemorySystem {
    /// Main context: fixed-size, always visible to the agent
    main_context: Vec<MemoryEntry>,
    main_context_budget: usize,

    /// Archival store: unbounded, searchable via tools
    archival_store: Vec<MemoryEntry>,
}

impl MemorySystem {
    pub fn new(main_budget: usize) -> Self {
        Self {
            main_context: Vec::new(),
            main_context_budget: main_budget,
            archival_store: Vec::new(),
        }
    }

    /// Page in: move from archival to main context (P14.07 interrupt pattern)
    pub fn page_in(&mut self, entry_id: &str) -> Result<(), AppError> {
        let pos = self.archival_store.iter().position(|e| e.id == entry_id)
            .ok_or_else(|| AppError::not_found(format!("Memory entry {} not found", entry_id)))?;
        let entry = self.archival_store.remove(pos);
        self.main_context.push(entry);

        // Evict oldest if over budget
        while self.main_context.len() > self.main_context_budget {
            self.main_context.remove(0);
        }
        Ok(())
    }

    /// Page out: move from main context to archival store
    pub fn page_out(&mut self, entry_id: &str) -> Result<(), AppError> {
        let pos = self.main_context.iter().position(|e| e.id == entry_id)
            .ok_or_else(|| AppError::not_found(format!("Memory entry {} not found", entry_id)))?;
        let entry = self.main_context.remove(pos);
        self.archival_store.push(entry);
        Ok(())
    }

    /// Search archival store by query (BM25-style)
    pub fn search_memory(&self, query: &str, top_k: usize) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(&MemoryEntry, f64)> = self.archival_store
            .iter()
            .map(|entry| {
                let content_lower = entry.content.to_lowercase();
                let score: f64 = query_terms
                    .iter()
                    .map(|term| {
                        content_lower.matches(term).count() as f64
                            / (content_lower.len() as f64 + 1.0)
                    })
                    .sum();
                (entry, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(e, _)| e).collect()
    }

    /// Archive a new memory entry
    pub fn archive(&mut self, entry: MemoryEntry) {
        self.archival_store.push(entry);
    }

    /// Get all entries (main context + archival) for persistence
    pub fn get_all_entries(&self) -> Vec<&MemoryEntry> {
        self.main_context.iter().chain(self.archival_store.iter()).collect()
    }

    /// Get the count of entries in main context
    pub fn main_context_len(&self) -> usize {
        self.main_context.len()
    }

    /// Get the count of entries in archival store
    pub fn archival_store_len(&self) -> usize {
        self.archival_store.len()
    }

    /// Get main context as formatted string for prompt injection
    pub fn format_main_context(&self) -> String {
        self.main_context
            .iter()
            .map(|e| format!("[{}] {}", e.entry_type as u8, e.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ── Context Passed to Every Agent ───────────────────────────────────────────

use super::iteration_budget::IterationBudget;
use super::tool_guardrails::ToolCallGuardrailController;
use super::context_compressor::ContextCompressor;

/// Context passed to every agent execution
#[derive(Clone)]
pub struct AgentContext {
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: std::path::PathBuf,
    pub book_id: Option<String>,
    /// Tool registry for this agent
    pub tools: Arc<ToolRegistry>,
    /// Memory system shared across agents
    pub memory: Arc<tokio::sync::RwLock<MemorySystem>>,
    /// 迭代预算 — 防止 Agent 无限循环
    pub iteration_budget: Arc<IterationBudget>,
    /// 工具调用守卫 — 检测工具调用循环
    pub tool_guardrails: Arc<tokio::sync::Mutex<ToolCallGuardrailController>>,
    /// 上下文压缩器 — 自动压缩长对话
    pub context_compressor: Arc<tokio::sync::Mutex<ContextCompressor>>,
}

// ── Base Agent Trait ────────────────────────────────────────────────────────

/// Base trait for all pipeline agents.
///
/// Agents follow the ReAct loop (P14.01): Observe → Think → Act → Observe
/// They have access to tools (P14.06) and memory (P14.07).
#[async_trait]
pub trait BaseAgent: Send + Sync {
    /// The role identifier
    fn role(&self) -> AgentRole;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Call the LLM with system + user messages (带重试的版本)
    async fn chat(
        &self,
        ctx: &AgentContext,
        system: &str,
        user: &str,
    ) -> Result<LlmResponse, AppError> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: user.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];
        self.chat_with_retry(ctx, system, &messages).await
    }

    /// Call the LLM with a full message history (带重试和消息清洗的版本)
    async fn chat_with_retry(
        &self,
        ctx: &AgentContext,
        system: &str,
        messages: &[Message],
    ) -> Result<LlmResponse, AppError> {
        use super::retry_utils::RetryState;
        use super::retry_utils::RetryConfig;

        let mut retry_state = RetryState::new(RetryConfig::default());
        let start = std::time::Instant::now();

        loop {
            // 消息清洗 — 确保角色交替正确
            let mut sanitized = messages.to_vec();
            super::message_sanitization::sanitize_message_sequence(&mut sanitized);

            match ctx.provider.complete(&ctx.model, system, &sanitized).await {
                Ok(content) => {
                    let elapsed = start.elapsed().as_millis();
                    tracing::debug!(
                        agent = self.name(),
                        response_len = content.len(),
                        elapsed_ms = elapsed,
                        retries = retry_state.attempt(),
                        "LLM call completed"
                    );
                    return Ok(LlmResponse {
                        content,
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    // 使用错误分类器判断是否可重试
                    let classified = super::error_classifier::classify_api_error(
                        &error_msg, None, "", "",
                    );

                    if !classified.retryable || !retry_state.can_retry() {
                        tracing::error!(
                            agent = self.name(),
                            error = %e,
                            reason = ?classified.reason,
                            retryable = classified.retryable,
                            attempts = retry_state.attempt(),
                            "LLM call failed (no more retries)"
                        );
                        return Err(e);
                    }

                    let backoff = retry_state.next_backoff();
                    tracing::warn!(
                        agent = self.name(),
                        error = %e,
                        reason = ?classified.reason,
                        attempt = retry_state.attempt(),
                        max_retries = retry_state.max_retries(),
                        backoff_secs = backoff,
                        "LLM call failed, retrying after backoff"
                    );

                    tokio::time::sleep(std::time::Duration::from_secs_f64(backoff)).await;
                }
            }
        }
    }

    /// Stream LLM response (P11 Streaming Support)
    /// NOTE: Requires Provider to implement complete_stream method
    /// When implemented, this returns a channel that yields response chunks
    async fn chat_stream(
        &self,
        ctx: &AgentContext,
        system: &str,
        user: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, AppError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let messages = vec![Message {
            role: "user".to_string(),
            content: user.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let provider = ctx.provider.clone();
        let model = ctx.model.clone();
        let system_clone = system.to_string();
        let agent_name = self.name().to_string();

        // TODO: Implement when Provider supports streaming
        // For now, fall back to non-streaming with retry
        tokio::spawn(async move {
            use super::retry_utils::{RetryState, RetryConfig};

            let mut retry_state = RetryState::new(RetryConfig::default());

            loop {
                // 消息清洗
                let mut sanitized = messages.clone();
                super::message_sanitization::sanitize_message_sequence(&mut sanitized);

                match provider.complete(&model, &system_clone, &sanitized).await {
                    Ok(content) => {
                        let _ = tx.send(content).await;
                        break;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let classified = super::error_classifier::classify_api_error(
                            &error_msg, None, "", "",
                        );

                        if !classified.retryable || !retry_state.can_retry() {
                            tracing::error!(
                                agent = %agent_name,
                                error = %e,
                                "Stream fallback error (no more retries)"
                            );
                            break;
                        }

                        let backoff = retry_state.next_backoff();
                        tracing::warn!(
                            agent = %agent_name,
                            error = %e,
                            attempt = retry_state.attempt(),
                            backoff_secs = backoff,
                            "Stream fallback retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs_f64(backoff)).await;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Execute a tool call (P14.06 Tool Use + P14.01 Observation)
    async fn use_tool(
        &self,
        ctx: &AgentContext,
        tool_call: &ToolCall,
    ) -> Result<ToolResult, AppError> {
        let start = std::time::Instant::now();
        let result = ctx.tools.execute(&tool_call.name, tool_call.arguments.clone()).await?;
        let elapsed = start.elapsed().as_millis();
        tracing::debug!(
            agent = self.name(),
            tool = %tool_call.name,
            is_error = result.is_error,
            elapsed_ms = elapsed,
            "Tool execution completed"
        );
        Ok(result)
    }

    /// Search memory for relevant context (P14.07)
    async fn search_memory(
        &self,
        ctx: &AgentContext,
        query: &str,
        top_k: usize,
    ) -> Vec<MemoryEntry> {
        let memory = ctx.memory.read().await;
        memory.search_memory(query, top_k)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Archive a memory entry (P14.07)
    async fn archive_memory(
        &self,
        ctx: &AgentContext,
        entry: MemoryEntry,
    ) {
        let mut memory = ctx.memory.write().await;
        memory.archive(entry);
    }

    /// Run the ReAct loop with tool support (P14.01)
    ///
    /// 集成:
    /// - IterationBudget: 每次工具调用前检查预算
    /// - ToolGuardrails: 工具调用前后检查循环模式
    /// - MessageSanitization: LLM 调用前清洗消息序列
    /// - RetryWithBackoff: LLM 调用失败时自动重试
    async fn react_loop(
        &self,
        ctx: &AgentContext,
        system: &str,
        initial_user: &str,
        max_steps: usize,
    ) -> Result<String, AppError> {
        let mut messages: Vec<Message> = vec![Message {
            role: "user".to_string(),
            content: initial_user.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        // 重置工具守卫
        ctx.tool_guardrails.lock().await.reset_for_turn();

        // 重置迭代预算（如果需要）
        ctx.iteration_budget.reset();

        for step in 0..max_steps {
            // 检查迭代预算
            if !ctx.iteration_budget.consume() {
                tracing::warn!(
                    step,
                    budget_used = ctx.iteration_budget.used(),
                    budget_max = ctx.iteration_budget.max_total(),
                    "Iteration budget exhausted in react_loop"
                );
                return Err(AppError::internal(format!(
                    "迭代预算已耗尽 ({}-{}/{} 已用)",
                    self.name(),
                    step,
                    ctx.iteration_budget.max_total()
                )));
            }

            // 消息清洗
            let mut sanitized = messages.clone();
            let sanitize_result = super::message_sanitization::sanitize_message_sequence(&mut sanitized);
            if sanitize_result.repaired {
                tracing::debug!(
                    agent = self.name(),
                    repairs = sanitize_result.repair_count,
                    "{}", sanitize_result.description
                );
            }

            // LLM 调用（带重试）
            let response = self.chat_with_retry(ctx, system, &sanitized).await?;
            let content = response.content;

            // 解析工具调用
            if let Some(tool_calls) = parse_tool_calls(&content) {
                // 添加 assistant 消息（带工具调用）
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: content.clone(),
                    tool_calls: Some(tool_calls.iter().map(|tc| {
                        crate::infra::llm::types::ToolCallRequest {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            arguments: tc.arguments.to_string(),
                        }
                    }).collect()),
                    tool_call_id: None,
                });

                // 执行每个工具调用
                for tc in &tool_calls {
                    // 工具守卫：调用前检查
                    let guard_decision = {
                        let mut guard = ctx.tool_guardrails.lock().await;
                        guard.before_call(&tc.name, &tc.arguments)
                    };

                    if guard_decision.should_halt() {
                        tracing::warn!(
                            tool = %tc.name,
                            code = %guard_decision.code,
                            message = %guard_decision.message,
                            "Tool guardrail halted tool call"
                        );
                        // 返回合成错误结果
                        let synthetic = super::tool_guardrails::toolguard_synthetic_result(&guard_decision);
                        messages.push(Message {
                            role: "tool".to_string(),
                            content: synthetic,
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                        });
                        // 返回守卫停止响应
                        return Ok(format!(
                            "工具调用被守卫阻止: {} — {}",
                            tc.name, guard_decision.message
                        ));
                    }

                    // 执行工具
                    let result = self.use_tool(ctx, tc).await?;
                    let is_error = result.is_error;

                    // 工具守卫：调用后检查
                    let result_content = {
                        let mut guard = ctx.tool_guardrails.lock().await;
                        let decision = guard.after_call(
                            &tc.name,
                            &tc.arguments,
                            &result.content,
                            Some(is_error),
                        );

                        if decision.action == "warn" || decision.action == "halt" {
                            super::tool_guardrails::append_toolguard_guidance(&result.content, &decision)
                        } else {
                            result.content
                        }
                    };

                    messages.push(Message {
                        role: "tool".to_string(),
                        content: result_content,
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });

                    // 检查守卫是否要求停止
                    let halt_decision = ctx.tool_guardrails.lock().await.halt_decision().cloned();
                    if let Some(decision) = halt_decision {
                        if decision.should_halt() {
                            let halt_response = format!(
                                "工具循环被守卫停止: {} 已失败 {} 次。请改变策略，不要重复相同调用。",
                                decision.tool_name, decision.count
                            );
                            return Ok(halt_response);
                        }
                    }
                }
                continue;
            }

            // 无工具调用 — 最终回答
            return Ok(content);
        }

        Err(AppError::internal(format!(
            "Agent 循环超过最大步数 ({}-{}步)",
            self.name(),
            max_steps
        )))
    }
}

/// Parse tool calls from LLM response.
/// Handles JSON tool calls (OpenAI/Anthropic format) and markdown patterns.
fn parse_tool_calls(content: &str) -> Option<Vec<ToolCall>> {
    // Try JSON parsing first (most reliable)
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(calls) = json.get("tool_calls").and_then(|v| v.as_array()) {
            let parsed: Vec<ToolCall> = calls.iter().filter_map(|tc| {
                let id = tc.get("id")?.as_str()?.to_string();
                let name = tc.get("function")?.get("name")?.as_str()?.to_string();
                let args = tc.get("function")?.get("arguments")
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::Value::Null);
                Some(ToolCall { id, name, arguments: args })
            }).collect();
            if !parsed.is_empty() { return Some(parsed); }
        }
    }

    // Try markdown tool call blocks (```tool_call ... ```)
    if content.contains("```tool_call") || content.contains("```json") {
        let calls: Vec<ToolCall> = content
            .lines()
            .filter(|line| line.starts_with("tool_call:") || line.starts_with("\"name\":"))
            .enumerate()
            .filter_map(|(i, line)| {
                let name = if line.starts_with("tool_call:") {
                    line.splitn(2, ':').nth(1)?.trim().to_string()
                } else {
                    line.splitn(2, ':').nth(1)?.trim().trim_matches(|c: char| c == '"' || c == ' ').to_string()
                };
                if name.is_empty() { return None; }
                Some(ToolCall {
                    id: format!("call_{}", i),
                    name,
                    arguments: serde_json::Value::Null,
                })
            })
            .collect();
        if !calls.is_empty() { return Some(calls); }
    }

    None
}
