//! ═══════════════════════════════════════════════════════════════════════════
//! Orchestrator Agent - Pipeline 中央调度代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：
//! - 只读：不直接写入文件，所有写操作委托给子 Agent
//! - 规划：决定 Pipeline 的执行顺序和并发策略
//! - 协调：在子 Agent 之间传递上下文
//! - 审查：验证子 Agent 输出质量
//!
//! 委托规则：
//! - 所有写操作必须委托给 writer/reviser
//! - 所有审计必须委托给 auditor
//! - 所有状态结算必须委托给 settler

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::super::types::BookConfig;

/// Orchestrator 执行阶段
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrchestratorPhase {
    Plan,
    Compose,
    Write,
    Audit,
    ReviseIfNeeded,
    Settle,
}

/// Orchestrator 执行计划
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OrchestratorPlan {
    pub phases: Vec<PhaseSpec>,
    pub parallel: Vec<Vec<String>>,
}

/// 阶段规格
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PhaseSpec {
    pub phase: String,
    pub subagent: String,
    pub task: String,
}

/// Orchestrator 审查结果
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorReview {
    pub action: ReviewAction,
    pub subagent: String,
    pub feedback: String,
    pub issues: Vec<String>,
}

/// 审查动作
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewAction {
    Accept,
    Revise,
    Reject,
}

/// Orchestrator 上下文
pub struct OrchestratorContext {
    pub book: BookConfig,
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
}

/// 生成执行计划
pub async fn generate_plan(
    engine: &AgentEngine,
    ctx: &OrchestratorContext,
) -> Result<OrchestratorPlan, AppError> {
    let system_prompt = build_orchestrator_system_prompt(&ctx.book);
    let user_message = build_plan_user_message(ctx);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    parse_plan(&response)
}

/// 审查子 Agent 输出
pub async fn review_subagent_output(
    engine: &AgentEngine,
    book: &BookConfig,
    subagent: &str,
    output: &str,
) -> Result<OrchestratorReview, AppError> {
    let system_prompt = build_orchestrator_system_prompt(book);
    let user_message = build_review_user_message(subagent, output);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    parse_review(&response)
}

fn build_orchestrator_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"<identity>
你是 Orchestrator —— Pipeline 的中央调度器。

你的职责不是写内容，而是：
1. READ：读取当前书籍状态（章节、真相文件、规则）
2. PLAN：决定子 Agent 调用顺序和并发策略
3. DELEGATE：将所有写操作委托给专业子 Agent
4. REVIEW：验证子 Agent 输出质量
5. DECIDE：决定是否接受、修订或拒绝子 Agent 工作
</identity>

<responsibilities>
## 只读操作（Orchestrator 可直接执行）
- 读取章节草稿、真相文件、书籍规则
- 查询 Pipeline 当前状态
- 分析哪些章节需要写作/修订

## 规划操作（Orchestrator 可直接执行）
- 决定子 Agent 执行顺序
- 为每个子 Agent 分配 token 预算
- 决定何时开始新章节 vs. 修订现有章节

## 委托操作（必须委托给子 Agent）
- **writer**：所有正文生成（章节内容）
- **auditor**：所有质量审计（37 维度检查）
- **reviser**：所有基于审计反馈的修订
- **settler**：所有状态结算（真相文件更新）
- **observer**：所有从章节提取事实
- **composer**：所有为 writer 组装上下文

## 审查操作（Orchestrator 可直接执行）
- 验证子 Agent 输出是否符合书籍规则
- 检查章节之间的连续性
- 批准或拒绝子 Agent 工作
- 请求重新工作当质量不足时
</responsibilities>

<delegation_rules>
## 规则 1：绝不直接写入
Orchestrator 绝不能使用 write_file、edit、multi_edit 工具。
所有内容修改必须通过 writer/reviser 子 Agent。

## 规则 2：委托所有审计
Orchestrator 绝不能直接执行审计。
所有质量检查必须通过 auditor 子 Agent。

## 规则 3：批量相关操作
当多个章节需要类似工作时，在同一个委托中批量处理。
示例："审计第 5-10 章"（一次调用）而非"审计第 5 章"× 6 次。

## 规则 4：提供完整上下文
委托时，提供完整上下文：
- 相关真相文件
- 书籍规则和风格指南
- 上一章摘要
- 任何特定约束

## 规则 5：接受前审查
子 Agent 完成工作后：
1. 读取输出
2. 检查是否符合书籍规则
3. 验证连续性
4. 接受、修订或拒绝
</delegation_rules>

## Book Information
- 书名：{title}
- 目标章数：{target_chapters} 章

<output_format>
## 规划时输出
输出 JSON 计划：
{{
  "phases": [
    {{
      "phase": "plan",
      "subagent": "planner",
      "task": "为第 N 章生成章节备忘录"
    }},
    {{
      "phase": "compose",
      "subagent": "composer",
      "task": "为第 N 章组装上下文"
    }},
    {{
      "phase": "write",
      "subagent": "writer",
      "task": "写作第 N 章"
    }},
    {{
      "phase": "audit",
      "subagent": "auditor",
      "task": "审计第 N 章"
    }},
    {{
      "phase": "revise_if_needed",
      "subagent": "reviser",
      "task": "根据审计反馈修订第 N 章"
    }}
  ],
  "parallel": [
    ["plan", "compose"],
    ["write"],
    ["audit"],
    ["revise_if_needed"]
  ]
}}

## 审查时输出
输出审查结果：
{{
  "action": "accept|revise|reject",
  "subagent": "writer|auditor|...",
  "feedback": "给子 Agent 的具体反馈",
  "issues": ["问题 1", "问题 2"]
}}
</output_format>"#,
        title = book.title,
        target_chapters = book.target_chapters,
    )
}

fn build_plan_user_message(ctx: &OrchestratorContext) -> String {
    format!(
        r#"请为下一章生成执行计划。

## Current State Card
{current_state}

## Hook Pool
{pending_hooks}

## Chapter Summaries
{chapter_summaries}

基于以上信息，输出执行计划（JSON 格式）。"#,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
    )
}

fn build_review_user_message(subagent: &str, output: &str) -> String {
    format!(
        r#"请审查 {subagent} 的输出。

## Subagent Output
{output}

输出审查结果（JSON 格式）：accept、revise 或 reject。"#,
        subagent = subagent,
        output = output,
    )
}

fn parse_plan(response: &str) -> Result<OrchestratorPlan, AppError> {
    let json_str = extract_json_block(response)
        .or_else(|| {
            if response.trim().starts_with('{') {
                Some(response.trim())
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::invalid_format("orchestrator plan JSON missing"))?;

    serde_json::from_str::<OrchestratorPlan>(json_str)
        .map_err(|e| AppError::invalid_format(format!("orchestrator plan JSON invalid: {}", e)))
}

fn parse_review(response: &str) -> Result<OrchestratorReview, AppError> {
    let json_str = extract_json_block(response)
        .or_else(|| {
            if response.trim().starts_with('{') {
                Some(response.trim())
            } else {
                None
            }
        })
        .ok_or_else(|| AppError::invalid_format("orchestrator review JSON missing"))?;

    serde_json::from_str::<OrchestratorReview>(json_str)
        .map_err(|e| AppError::invalid_format(format!("orchestrator review JSON invalid: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plan() {
        let response = r#"{
      "phases": [
        {"phase": "plan", "subagent": "planner", "task": "Generate memo for chapter 1"}
      ],
      "parallel": [["plan"]]
    }"#;
        let plan = parse_plan(response).unwrap();
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].subagent, "planner");
    }

    #[test]
    fn parses_review() {
        let response = r#"{
      "action": "accept",
      "subagent": "writer",
      "feedback": "Good quality",
      "issues": []
    }"#;
        let review = parse_review(response).unwrap();
        assert_eq!(review.action, ReviewAction::Accept);
        assert_eq!(review.subagent, "writer");
    }
}