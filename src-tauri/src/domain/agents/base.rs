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

    /// Call the LLM with system + user messages
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
        let start = std::time::Instant::now();
        let content = ctx
            .provider
            .complete(&ctx.model, system, &messages)
            .await?;
        let elapsed = start.elapsed().as_millis();
        tracing::debug!(
            agent = self.name(),
            response_len = content.len(),
            elapsed_ms = elapsed,
            "LLM call completed"
        );
        Ok(LlmResponse {
            content,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        })
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

        // TODO: Implement when Provider supports streaming
        // For now, fall back to non-streaming
        tokio::spawn(async move {
            match provider.complete(&model, &system_clone, &messages).await {
                Ok(content) => {
                    let _ = tx.send(content).await;
                }
                Err(e) => {
                    tracing::error!("Stream error: {:?}", e);
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

        for _step in 0..max_steps {
            let content = ctx.provider.complete(&ctx.model, system, &messages).await?;

            // Parse tool calls from response
            if let Some(tool_calls) = parse_tool_calls(&content) {
                // Add assistant message with tool calls
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

                // Execute each tool call and add results as "tool" role
                for tc in &tool_calls {
                    let result = self.use_tool(ctx, tc).await?;
                    messages.push(Message {
                        role: "tool".to_string(),
                        content: result.content,
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
                continue;
            }

            // No tool calls - we have a final answer
            return Ok(content);
        }

        Err(AppError::internal("Agent loop exceeded max steps"))
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
