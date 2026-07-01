//! 子 Agent 控制器。
//!
//! 学习 codex 的 `AgentControl`：单实例 + `Arc<ThreadRegistry>` + 完成消息异步推送。
//! `SubAgentControl` 是子 Agent 系统的入口，负责 spawn、取消、状态查询。
//!
//! 与 codex 的关键差异：
//! - `spawn` 接收 `ParentAgentRefs`（而非 `&AgentContext`），避免引用环
//!   （ToolRegistry → SpawnAgentTool → AgentContext → ToolRegistry）
//! - 子 Agent 工具集由角色白名单决定（codex 没有）
//! - 结果通过 `oneshot::channel` 同步回传给调用方（工具等待结果）

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};

use crate::core::agent::prompts::shared_sections::REACT_DISCIPLINE_ZH;
use crate::core::agent::{
    AgentContext, ContextCompressor, CompressorConfig, IterationBudget,
    MemorySystem, ToolCallGuardrailController, ToolGuardrailConfig,
};
use crate::features::skill_manager::SkillManager;
use crate::features::user_profile::UserProfileStore;
use crate::infrastructure::llm_client::Provider;
use crate::shared::errors::AppError;

use futures::StreamExt;
use crate::infrastructure::llm_client::types::{FinishReason, Message, StreamEvent, ToolCallRequest, ToolSpec};

use super::registry::{ThreadRegistry, MAX_CONCURRENT, MAX_DEPTH};
use super::role::{self, RoleConfig};
use super::types::{SubAgentResult, SubAgentRole, SubAgentSpawnRequest, SubAgentStatus};

/// 子 Agent ReAct 循环最大轮次（比主 Agent 的 30 更保守）。
const MAX_SUB_AGENT_TURNS: u32 = 15;

/// 父 Agent 的引用集合 — 从 `AgentContext` 中提取的非 ToolRegistry 字段。
///
/// 设计理由：`SpawnAgentTool` 注册在父 Agent 的 `ToolRegistry` 中，
/// 如果工具持有完整的 `Arc<AgentContext>` 会形成引用环
/// （ToolRegistry → SpawnAgentTool → AgentContext → ToolRegistry）导致内存泄漏。
/// 因此只提取 spawn 子 Agent 所需的字段（均为 Arc/Clone 廉价复制）。
///
/// 与 spec 中 `spawn(req, parent_ctx: &AgentContext)` 的偏差已记录于此。
pub struct ParentAgentRefs {
    /// 父线程 ID（主 Agent 的 session_id 或上级子 Agent 的 task_id）。
    pub parent_thread_id: String,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub project_root: PathBuf,
    pub book_id: Option<String>,
    pub memory: Arc<RwLock<MemorySystem>>,
    pub skill_manager: Option<Arc<SkillManager>>,
    pub user_profile: Option<Arc<Mutex<UserProfileStore>>>,
}

/// 子 Agent 控制器 — 单实例，管理所有子 Agent 的生命周期。
///
/// 持有：
/// - `Arc<ThreadRegistry>`：子 Agent 元信息注册表（查询用）
/// - `Mutex<HashMap<task_id, CancelFlag>>`：取消标志（运行时控制用）
pub struct SubAgentControl {
    registry: Arc<ThreadRegistry>,
    cancel_flags: Mutex<HashMap<String, Arc<RwLock<bool>>>>,
}

impl SubAgentControl {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ThreadRegistry::new()),
            cancel_flags: Mutex::new(HashMap::new()),
        }
    }

    pub fn registry(&self) -> &Arc<ThreadRegistry> {
        &self.registry
    }

    /// Spawn 一个子 Agent。
    ///
    /// 流程：
    /// 1. 检查深度和并发上限
    /// 2. 在 registry 注册
    /// 3. 构建子 Agent 的 AgentContext（角色过滤后的工具集）
    /// 4. tokio::spawn 一个独立 task 运行子 agent loop
    /// 5. 返回 (task_id, result_receiver)
    ///
    /// 调用方可选择 await receiver 等待结果（SpawnAgentTool 会等待），
    /// 或丢弃 receiver 实现 fire-and-forget。
    pub async fn spawn(
        &self,
        req: SubAgentSpawnRequest,
        refs: &ParentAgentRefs,
    ) -> Result<(String, oneshot::Receiver<SubAgentResult>), AppError> {
        // 1. 检查深度上限
        let parent_depth = self.registry.parent_depth(&req.parent_thread_id).await.unwrap_or(0);
        let depth = parent_depth + 1;
        if depth > MAX_DEPTH {
            return Err(AppError::bad_request(format!(
                "Sub-agent depth {} exceeds MAX_DEPTH {} (recursive spawn limit)",
                depth, MAX_DEPTH
            )));
        }

        // 2. 检查并发上限
        let active = self.registry.count_active().await;
        if active >= MAX_CONCURRENT {
            return Err(AppError::bad_request(format!(
                "Concurrent sub-agent limit reached ({}/{}). Wait for active sub-agents to complete.",
                active, MAX_CONCURRENT
            )));
        }

        // 3. 注册
        let task_id = self.registry.register(req.clone(), depth).await;

        // 4. 创建取消标志
        let cancel_flag = Arc::new(RwLock::new(false));
        self.cancel_flags.lock().await.insert(task_id.clone(), cancel_flag.clone());

        // 5. 构建子 Agent 的 AgentContext
        let sub_tools = role::build_role_tool_registry(req.role, refs.project_root.clone(), refs.memory.clone());
        let sub_ctx = AgentContext {
            provider: refs.provider.clone(),
            model: refs.model.clone(),
            project_root: refs.project_root.clone(),
            book_id: refs.book_id.clone(),
            tools: Arc::new(sub_tools),
            memory: refs.memory.clone(),
            iteration_budget: Arc::new(IterationBudget::new(30)),
            tool_guardrails: Arc::new(tokio::sync::Mutex::new(
                ToolCallGuardrailController::new(ToolGuardrailConfig::default())
            )),
            context_compressor: Arc::new(tokio::sync::Mutex::new(
                ContextCompressor::new(CompressorConfig::default())
            )),
            skill_manager: refs.skill_manager.clone(),
            user_profile: refs.user_profile.clone(),
        };

        // 6. 创建结果 channel
        let (result_tx, result_rx) = oneshot::channel::<SubAgentResult>();

        // 7. 更新状态为 Running 并 spawn 异步任务
        self.registry.update_status(&task_id, SubAgentStatus::Running).await;

        let registry = self.registry.clone();
        let task_id_clone = task_id.clone();
        let role = req.role;
        let task = req.task;
        let context = req.context;

        tokio::spawn(async move {
            let result = run_sub_agent(
                &task_id_clone,
                sub_ctx,
                role,
                task,
                context,
                &registry,
                &cancel_flag,
            ).await;

            // 更新注册表状态
            registry.update_status(&task_id_clone, result.status).await;

            // 推送结果（如果接收方已丢弃，忽略错误）
            let _ = result_tx.send(result);
        });

        Ok((task_id, result_rx))
    }

    /// 查询子 Agent 状态。
    pub async fn get_status(&self, task_id: &str) -> Option<SubAgentStatus> {
        self.registry.get(task_id).await.map(|info| info.status)
    }

    /// 取消子 Agent。
    ///
    /// 设置取消标志，子 Agent 在下一轮 ReAct 循环开始时检查并退出。
    /// 如果子 Agent 已完成，此操作无效。
    pub async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        let flags = self.cancel_flags.lock().await;
        let flag = flags.get(task_id).cloned();
        drop(flags);

        match flag {
            Some(f) => {
                *f.write().await = true;
                self.registry.update_status(task_id, SubAgentStatus::Cancelled).await;
                tracing::info!(task_id = %task_id, "Sub-agent cancel signal sent");
                Ok(())
            }
            None => {
                // 可能已清理或不存在 — 幂等返回成功
                tracing::warn!(task_id = %task_id, "Cancel called on unknown sub-agent (no-op)");
                Ok(())
            }
        }
    }

    /// 列出指定父线程的所有子 Agent。
    pub async fn list_children(&self, parent_thread_id: &str) -> Vec<super::types::SubAgentInfo> {
        self.registry.list_by_parent(parent_thread_id).await
    }
}

impl Default for SubAgentControl {
    fn default() -> Self {
        Self::new()
    }
}

/// 子 Agent 的简化 ReAct 循环。
///
/// 参考 `main_agent/agent_loop.rs` 的 ReAct 模式，但：
/// - max_turns = 15（更保守）
/// - 无 confirmation 流程（角色工具白名单是安全边界）
/// - 追踪 write_file 调用作为 artifacts
/// - 每轮检查取消标志
///
/// 完成后返回 `SubAgentResult`。
async fn run_sub_agent(
    task_id: &str,
    ctx: AgentContext,
    role: SubAgentRole,
    task: String,
    context: Option<String>,
    registry: &Arc<ThreadRegistry>,
    cancel_flag: &Arc<RwLock<bool>>,
) -> SubAgentResult {
    let _registry = registry;  // 用于未来状态更新
    let start = std::time::Instant::now();
    let role_config = RoleConfig::for_role(role);

    // 构建 tool_specs
    let tool_specs: Vec<ToolSpec> = ctx.tools.definitions().into_iter()
        .map(|d| ToolSpec {
            name: d.name,
            description: d.description,
            parameters: d.parameters,
        })
        .collect();

    // 构建系统提示词：角色片段 + ReAct 纪律 + 子 Agent 约束
    let system_prompt = format!(
        "{role_fragment}\n\n{react_discipline}\n\n## 子 Agent 工作约束\n\
         - 你是被父 Agent 委托的子 Agent，聚焦完成指定任务。\n\
         - 完成任务后给出明确的最终输出（不调用工具的纯文本回复即视为完成）。\n\
         - 最大轮次：{max_turns} 轮，合理规划推进节奏。\n\
         - 你不能 spawn 子 Agent。",
        role_fragment = role_config.system_prompt_fragment,
        react_discipline = REACT_DISCIPLINE_ZH,
        max_turns = MAX_SUB_AGENT_TURNS,
    );

    // 构建初始消息
    let task_message = match &context {
        Some(ctx_text) => format!("Task: {}\n\nAdditional context:\n{}", task, ctx_text),
        None => format!("Task: {}", task),
    };

    let mut messages: Vec<Message> = vec![Message {
        role: "user".to_string(),
        content: task_message,
        tool_calls: None,
        tool_call_id: None,
    }];

    let mut final_output = String::new();
    let mut last_assistant_text = String::new();
    let mut artifacts: Vec<String> = Vec::new();
    let mut error: Option<String> = None;
    let mut cancelled = false;

    for turn in 1..=MAX_SUB_AGENT_TURNS {
        // 检查取消标志
        if *cancel_flag.read().await {
            cancelled = true;
            break;
        }

        tracing::debug!(task_id = %task_id, turn, role = %role, "Sub-agent ReAct turn");

        // 流式调用 LLM
        let mut stream = match ctx.provider.stream(
            &ctx.model,
            &system_prompt,
            &messages,
            &tool_specs,
        ).await {
            Ok(s) => s,
            Err(e) => {
                error = Some(format!("LLM stream error: {}", e));
                break;
            }
        };

        let mut text_buf = String::new();
        let mut reasoning_buf = String::new();
        let mut tool_calls: HashMap<String, (String, String)> = HashMap::new();
        let mut finish_reason = FinishReason::Stop;

        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::TextDelta { content } => text_buf.push_str(&content),
                StreamEvent::ReasoningDelta { content } => reasoning_buf.push_str(&content),
                StreamEvent::ToolCallStart { id, name } => {
                    tool_calls.entry(id).or_insert((name, String::new()));
                }
                StreamEvent::ToolCallDelta { id, args_delta } => {
                    if let Some(entry) = tool_calls.get_mut(&id) {
                        entry.1.push_str(&args_delta);
                    }
                }
                StreamEvent::ToolCallEnd { id: _ } => {}
                StreamEvent::Finish { reason, usage: _ } => {
                    finish_reason = reason;
                }
                StreamEvent::Error(e) => {
                    error = Some(format!("Stream error: {}", e));
                    break;
                }
            }
        }

        if error.is_some() {
            break;
        }

        if !reasoning_buf.is_empty() {
            tracing::debug!(
                task_id = %task_id, turn,
                reasoning_len = reasoning_buf.len(),
                "Sub-agent produced reasoning"
            );
        }

        // 空回复：避免死循环
        if text_buf.is_empty() && tool_calls.is_empty() {
            tracing::warn!(task_id = %task_id, turn, "Empty sub-agent turn, stopping");
            break;
        }

        // 把 assistant 输出加入对话
        let tool_call_requests: Vec<ToolCallRequest> = tool_calls.iter()
            .map(|(id, (name, args))| ToolCallRequest {
                id: id.clone(),
                name: name.clone(),
                arguments: args.clone(),
            })
            .collect();

        let tc_opt = if tool_call_requests.is_empty() {
            None
        } else {
            Some(tool_call_requests.clone())
        };

        messages.push(Message {
            role: "assistant".to_string(),
            content: text_buf.clone(),
            tool_calls: tc_opt,
            tool_call_id: None,
        });

        last_assistant_text = text_buf.clone();

        // 判断是否最终答案
        if !matches!(finish_reason, FinishReason::ToolCalls) || tool_calls.is_empty() {
            final_output = text_buf;
            break;
        }

        // 执行 tool_calls
        for (id, (name, args_str)) in &tool_calls {
            let args: serde_json::Value = serde_json::from_str(args_str)
                .unwrap_or(serde_json::Value::Null);

            // 追踪 write_file 产出的文件路径
            if name == "write_file" {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    if !artifacts.contains(&path.to_string()) {
                        artifacts.push(path.to_string());
                    }
                }
            }

            let tool_result = match ctx.tools.execute(name, args).await {
                Ok(r) => r,
                Err(e) => {
                    // 工具执行错误：把错误信息作为 tool 结果反馈给 LLM，让它自行处理
                    tracing::warn!(
                        task_id = %task_id, turn,
                        tool = %name, error = %e,
                        "Sub-agent tool execution error"
                    );
                    crate::core::agent::base::ToolResult {
                        tool_call_id: id.clone(),
                        content: format!("Error: {}", e),
                        is_error: true,
                    }
                }
            };

            let content = if tool_result.is_error {
                format!("Error: {}", tool_result.content)
            } else {
                tool_result.content
            };

            messages.push(Message {
                role: "tool".to_string(),
                content,
                tool_calls: None,
                tool_call_id: Some(id.clone()),
            });
        }
    }

    // 收尾：如果没拿到 final answer，用最后的 assistant text
    if final_output.is_empty() {
        final_output = if last_assistant_text.is_empty() {
            format!("Sub-agent reached max turns ({}) without final answer", MAX_SUB_AGENT_TURNS)
        } else {
            last_assistant_text
        };
    }

    let status = if cancelled {
        SubAgentStatus::Cancelled
    } else if error.is_some() {
        SubAgentStatus::Errored
    } else {
        SubAgentStatus::Completed
    };

    SubAgentResult {
        task_id: task_id.to_string(),
        role,
        status,
        output: final_output,
        artifacts,
        error,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_constants_are_reasonable() {
        assert_eq!(MAX_DEPTH, 3);
        assert_eq!(MAX_CONCURRENT, 4);
        assert!(MAX_SUB_AGENT_TURNS >= 5 && MAX_SUB_AGENT_TURNS <= 30);
    }

    #[tokio::test]
    async fn cancel_unknown_task_is_noop() {
        let control = SubAgentControl::new();
        // 取消不存在的 task_id 应该幂等返回 Ok
        assert!(control.cancel("nonexistent").await.is_ok());
    }

    #[tokio::test]
    async fn get_status_for_unknown_returns_none() {
        let control = SubAgentControl::new();
        assert!(control.get_status("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn list_children_empty() {
        let control = SubAgentControl::new();
        let children = control.list_children("session-1").await;
        assert!(children.is_empty());
    }
}
