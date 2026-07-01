//! 子 Agent 完成消息格式化。
//!
//! 学习 codex 的 `InterAgentCompletionMessage`：子 Agent 完成后，结果以结构化文本
//! 消息回传给父 Agent。该消息作为 `spawn_subagent` 工具的返回值，让父 Agent
//! 在下一轮 ReAct 中看到子 Agent 的产出。

use super::types::{SubAgentResult, SubAgentRole};

/// 格式化子 Agent 完成消息。
///
/// 该消息作为 `spawn_subagent` 工具的 tool result 返回给父 Agent。
/// 使用 `=== SUB_AGENT_COMPLETED ===` 标记便于父 Agent 识别和解析。
pub fn format_completion_message(
    task_name: &str,
    role: &SubAgentRole,
    result: &SubAgentResult,
) -> String {
    let artifacts_str = if result.artifacts.is_empty() {
        "(none)".to_string()
    } else {
        result.artifacts.join(", ")
    };

    let error_str = result.error.as_deref().unwrap_or("(none)");

    format!(
        r#"=== SUB_AGENT_COMPLETED ===
Task: {task_name}
Role: {role}
Status: {status}
Output:
{output}
Artifacts: {artifacts}
Error: {error}
=== END ==="#,
        task_name = task_name,
        role = role,
        status = format!("{:?}", result.status),
        output = result.output,
        artifacts = artifacts_str,
        error = error_str,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::sub_agent::types::SubAgentStatus;

    #[test]
    fn format_message_completed_no_artifacts() {
        let result = SubAgentResult {
            task_id: "task-123".to_string(),
            role: SubAgentRole::Researcher,
            status: SubAgentStatus::Completed,
            output: "Found 3 relevant files".to_string(),
            artifacts: vec![],
            error: None,
            duration_ms: 1500,
        };

        let msg = format_completion_message("task-123", &SubAgentRole::Researcher, &result);
        assert!(msg.contains("=== SUB_AGENT_COMPLETED ==="));
        assert!(msg.contains("Task: task-123"));
        assert!(msg.contains("Role: researcher"));
        assert!(msg.contains("Status: Completed"));
        assert!(msg.contains("Found 3 relevant files"));
        assert!(msg.contains("Artifacts: (none)"));
        assert!(msg.contains("Error: (none)"));
        assert!(msg.contains("=== END ==="));
    }

    #[test]
    fn format_message_errored_with_artifacts() {
        let result = SubAgentResult {
            task_id: "task-456".to_string(),
            role: SubAgentRole::Outliner,
            status: SubAgentStatus::Errored,
            output: "Partial outline written".to_string(),
            artifacts: vec!["outline/ch1.md".to_string(), "outline/ch2.md".to_string()],
            error: Some("Token limit exceeded".to_string()),
            duration_ms: 30000,
        };

        let msg = format_completion_message("task-456", &SubAgentRole::Outliner, &result);
        assert!(msg.contains("Artifacts: outline/ch1.md, outline/ch2.md"));
        assert!(msg.contains("Error: Token limit exceeded"));
        assert!(msg.contains("Status: Errored"));
    }
}
