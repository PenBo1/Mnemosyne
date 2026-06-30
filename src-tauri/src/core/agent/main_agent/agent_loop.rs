use crate::shared::errors::AppError;
use crate::core::agent::base::AgentContext;
use crate::core::agent::prompts::shared_sections::REACT_DISCIPLINE_ZH;
use super::types::*;
use super::safety_gate::SafetyGate;
use std::sync::Arc;
use tokio::sync::mpsc;

use futures::StreamExt;
use crate::infrastructure::llm_client::types::{Message, StreamEvent, FinishReason, ToolSpec, ToolCallRequest};

/// Main Agent 身份与任务推进段。
///
/// 此段定义 main_agent 的角色（自主任务执行器）、能力边界、任务推进原则与
/// SafetyGate 确认流程约定。与 `REACT_DISCIPLINE_ZH` 组装为完整系统提示词。
///
/// 设计参考：
/// - codex `gpt_5_1_prompt.md` 的 "Autonomy and Persistence" 段
/// - hermes-agent `TASK_COMPLETION_GUIDANCE` 的"持续到完成、诚实报告 blocker"
const MAIN_AGENT_HEADER: &str = r#"你是 Mnemosyne 主 Agent，一个自主任务执行器。用户会给你一个目标，你需要通过工具调用逐步推进直到完成，或在确实无法完成时明确说明原因。

## 你的能力

- 通过工具调用执行实际操作（读/写文件、运行命令、搜索记忆等）。
- 在 ReAct 循环中自主决定下一步动作；高风险操作会触发用户确认流程。
- 单次任务最多 30 轮工具调用预算，请合理规划推进节奏。

## 任务推进原则

- **持续到完成**：不要做了一半就报告"已完成"，必须真正交付用户要的结果。
- **遇到失败不放弃**：工具调用失败时，先分析错误信息、尝试替代方案，而不是立即报告失败。
- **诚实报告 blocker**：确实无法完成时，明确说明阻塞点、已尝试的方案、需要的帮助。
- **避免无效循环**：如果同一工具调用连续失败 2 次以上，重新审视策略而不是机械重试。
- **粒度合适**：每轮聚焦一个明确的子任务，不要在单轮里塞过多工具调用导致难以追踪。
- **高风险确认**：当 SafetyGate 触发用户确认时，你的回复要清晰说明该动作的目的、影响范围、可逆性，便于用户做出决策。用户拒绝后不要重复发起相同调用，应改换策略或请求澄清。
"#;

/// 构造 main_agent 的完整系统提示词。
///
/// 组装顺序（参考 hermes-agent 3 层架构，但简化为 2 段）：
/// 1. `MAIN_AGENT_HEADER`：身份 + 任务推进原则 + SafetyGate 约定（场景特定）
/// 2. `REACT_DISCIPLINE_ZH`：ReAct 工作模式 + 强制规则 + 安全约束（跨 agent 共享）
fn build_react_system_prompt() -> String {
    format!("{}\n{}", MAIN_AGENT_HEADER, REACT_DISCIPLINE_ZH)
}

/// 单轮 ReAct 的最大循环次数（防止无限循环）
const MAX_REACT_TURNS: u32 = 30;

/// The core autonomous agent loop.
///
/// 实现 ReAct 范式（参考 codex turn-based 主循环 + hermes-agent 工具强制规则）：
/// 每轮通过 `provider.stream()` 让 LLM 自主决定是否调用工具。
/// 工具结果作为 tool message 反馈给 LLM，由 LLM 决定下一步动作。
pub struct AgentLoop {
    ctx: AgentContext,
    progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    confirmation_tx: mpsc::UnboundedSender<ConfirmationRequest>,
    confirmation_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ConfirmationResponse>>>,
}

impl AgentLoop {
    pub fn new(
        ctx: AgentContext,
        progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
        confirmation_tx: mpsc::UnboundedSender<ConfirmationRequest>,
        confirmation_rx: mpsc::UnboundedReceiver<ConfirmationResponse>,
    ) -> Self {
        Self {
            ctx,
            progress_tx,
            confirmation_tx,
            confirmation_rx: Arc::new(tokio::sync::Mutex::new(confirmation_rx)),
        }
    }

    /// Execute a goal autonomously via ReAct loop.
    ///
    /// 流程：
    /// 1. 把 goal 作为 user message 加进对话
    /// 2. 调用 LLM stream，收集 text + reasoning + tool_calls
    /// 3. 如果 LLM 给出 Stop：拿到最终答案，结束
    /// 4. 如果 LLM 给出 ToolCalls：执行工具（高风险走 confirmation），把结果加回对话，回到 2
    /// 5. 达到 MAX_REACT_TURNS 仍未完成：用最后的 assistant text 作为 fallback
    pub async fn execute(&self, goal: &str) -> Result<String, AppError> {
        // 把 ToolRegistry 转成 LLM 可识别的 ToolSpec 列表
        let tool_specs: Vec<ToolSpec> = self.ctx.tools.definitions().into_iter()
            .map(|d| ToolSpec {
                name: d.name,
                description: d.description,
                parameters: d.parameters,
            })
            .collect();

        // 系统提示词在循环外构造一次（身份段 + ReAct 强制规则段）
        let system_prompt = build_react_system_prompt();

        let mut messages: Vec<Message> = vec![Message {
            role: "user".to_string(),
            content: format!("Goal: {}", goal),
            tool_calls: None,
            tool_call_id: None,
        }];

        self.send_progress(
            AgentStatus::Planning,
            None,
            None,
            "Starting ReAct loop".to_string(),
        ).await;

        let mut final_answer = String::new();
        let mut last_assistant_text = String::new();

        for turn in 1..=MAX_REACT_TURNS {
            self.send_progress(
                AgentStatus::Executing,
                Some(turn),
                Some(MAX_REACT_TURNS),
                format!("Turn {}", turn),
            ).await;

            // ── 流式调用 LLM ───────────────────────────────────────
            let mut stream = self.ctx.provider.stream(
                &self.ctx.model,
                &system_prompt,
                &messages,
                &tool_specs,
            ).await?;

            let mut text_buf = String::new();
            let mut reasoning_buf = String::new();
            // tool_call_id → (name, args_str)
            let mut tool_calls: std::collections::HashMap<String, (String, String)> =
                std::collections::HashMap::new();
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
                    StreamEvent::ToolCallEnd { id: _ } => {
                        // args 已通过 Delta 累积完成；End 仅作信号
                    }
                    StreamEvent::Finish { reason, usage: _ } => {
                        finish_reason = reason;
                    }
                    StreamEvent::Error(e) => {
                        self.send_progress(
                            AgentStatus::Failed,
                            Some(turn),
                            None,
                            format!("Stream error: {}", e),
                        ).await;
                        return Err(AppError::internal(format!("Stream error: {}", e)));
                    }
                }
            }

            if !reasoning_buf.is_empty() {
                tracing::info!(
                    turn,
                    reasoning_len = reasoning_buf.len(),
                    "LLM produced reasoning"
                );
            }

            // ── 把 assistant 的本轮输出加入对话 ─────────────────────
            if text_buf.is_empty() && tool_calls.is_empty() {
                // 空回复：模型未输出任何内容，避免死循环直接 break
                tracing::warn!(turn, "Empty assistant turn, stopping");
                break;
            }

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

            // ── 判断是否最终答案 ───────────────────────────────────
            if !matches!(finish_reason, FinishReason::ToolCalls) || tool_calls.is_empty() {
                final_answer = text_buf;
                break;
            }

            // ── 执行 tool_calls（高风险走 confirmation）──────────────
            for (id, (name, args_str)) in &tool_calls {
                let args: serde_json::Value = serde_json::from_str(args_str)
                    .unwrap_or(serde_json::Value::Null);

                let risk = SafetyGate::evaluate_risk(name, &args);
                let args_after_confirmation = match risk {
                    RiskLevel::High | RiskLevel::Moderate => {
                        // 请求用户确认
                        let request = SafetyGate::create_confirmation_request(
                            turn,
                            name,
                            &args,
                        );
                        self.send_progress(
                            AgentStatus::WaitingForConfirmation,
                            Some(turn),
                            Some(MAX_REACT_TURNS),
                            format!("Confirm: {}", request.description),
                        ).await;
                        let _ = self.confirmation_tx.send(request);

                        let response = {
                            let mut rx = self.confirmation_rx.lock().await;
                            rx.recv().await.unwrap_or(ConfirmationResponse::Rejected)
                        };

                        match response {
                            ConfirmationResponse::Approved => Some(args.clone()),
                            ConfirmationResponse::Rejected => {
                                // 用户拒绝：把拒绝信息作为 tool 结果反馈给 LLM
                                messages.push(Message {
                                    role: "tool".to_string(),
                                    content: "[User rejected this tool call]".to_string(),
                                    tool_calls: None,
                                    tool_call_id: Some(id.clone()),
                                });
                                continue;
                            }
                            ConfirmationResponse::Modified(new_args) => {
                                serde_json::from_str::<serde_json::Value>(&new_args).ok()
                            }
                        }
                    }
                    RiskLevel::Safe => Some(args.clone()),
                };

                let args_to_run = match args_after_confirmation {
                    Some(a) => a,
                    None => {
                        // 修改后的参数解析失败，用原始 args
                        args.clone()
                    }
                };

                let tool_result = self.ctx.tools.execute(name, args_to_run).await?;
                let content = if tool_result.is_error {
                    format!("Error: {}", tool_result.content)
                } else {
                    tool_result.content
                };

                self.send_progress(
                    AgentStatus::Executing,
                    Some(turn),
                    Some(MAX_REACT_TURNS),
                    format!("Tool {} completed ({} chars)", name, content.len()),
                ).await;

                messages.push(Message {
                    role: "tool".to_string(),
                    content,
                    tool_calls: None,
                    tool_call_id: Some(id.clone()),
                });
            }
            // 继续 next turn：LLM 看到 tool 结果后再次推理
        }

        // ── 收尾：如果到 max_turns 仍没拿到 final answer，用最后 assistant text ──
        if final_answer.is_empty() {
            final_answer = if last_assistant_text.is_empty() {
                format!("Reached max turns ({}) without final answer", MAX_REACT_TURNS)
            } else {
                last_assistant_text
            };
        }

        self.send_progress(
            AgentStatus::Completed,
            None,
            None,
            "Execution complete".to_string(),
        ).await;

        Ok(final_answer)
    }

    async fn send_progress(&self, status: AgentStatus, current: Option<u32>, total: Option<u32>, message: String) {
        let _ = self.progress_tx.send(ProgressUpdate {
            status,
            current_step: current,
            total_steps: total,
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_react_system_prompt_includes_tool_enforcement() {
        let prompt = build_react_system_prompt();
        // 跨 agent 共享段：ReAct 强制规则
        assert!(prompt.contains("tool_call"));
        assert!(prompt.contains("禁止\"光说不做\""));
        assert!(prompt.contains("禁止停在 stub"));
        assert!(prompt.contains("禁止编造"));
        // main_agent 专属：任务推进原则
        assert!(prompt.contains("持续到完成"));
        assert!(prompt.contains("诚实报告 blocker"));
        assert!(prompt.contains("SafetyGate"));
    }

    #[test]
    fn test_max_react_turns_is_reasonable() {
        // 30 轮足够大多数任务，又不会让死循环烧光预算
        assert!(MAX_REACT_TURNS >= 10);
        assert!(MAX_REACT_TURNS <= 50);
    }
}
