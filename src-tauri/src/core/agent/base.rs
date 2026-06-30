use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::llm_client::{Provider, types::Message};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::features::skill_manager::discovery::SkillManager;
use super::types::{AgentRole, LlmResponse};
use crate::features::user_profile::UserProfileStore;

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
//
// MemoryEntry 与 MemoryType 已下沉到 crate::shared::memory（修复 infra → core/agent
// 反向依赖）。这里通过 re-export 保持 `crate::core::agent::MemoryEntry` 路径兼容。
pub use crate::shared::memory::{MemoryEntry, MemoryType};

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
        scored.into_iter().filter(|(_, score)| *score > 0.0).take(top_k).map(|(e, _)| e).collect()
    }

    /// Archive a new memory entry
    pub fn archive(&mut self, entry: MemoryEntry) {
        self.archival_store.push(entry);
    }

    /// Delete an entry by id from both main context and archival store
    pub fn delete_entry(&mut self, entry_id: &str) -> bool {
        let before = self.main_context.len() + self.archival_store.len();
        self.main_context.retain(|e| e.id != entry_id);
        self.archival_store.retain(|e| e.id != entry_id);
        let after = self.main_context.len() + self.archival_store.len();
        before != after
    }

    /// Update an entry's content and tags by id (searches both stores)
    pub fn update_entry(
        &mut self,
        entry_id: &str,
        content: String,
        tags: Vec<String>,
    ) -> Option<MemoryEntry> {
        for entry in self.main_context.iter_mut().chain(self.archival_store.iter_mut()) {
            if entry.id == entry_id {
                entry.content = content.clone();
                entry.tags = tags.clone();
                return Some(entry.clone());
            }
        }
        None
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
    /// Skill manager for dynamic skill injection
    pub skill_manager: Option<Arc<SkillManager>>,
    /// User profile for tailoring output
    pub user_profile: Option<std::sync::Arc<tokio::sync::Mutex<UserProfileStore>>>,
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

    // 原 react_loop 实现已废弃（被 main_agent/agent_loop.rs 的 execute_step + main_loop 取代）。
    // 如需恢复，参考 git history 中本文件的 react_loop 实现。
}
