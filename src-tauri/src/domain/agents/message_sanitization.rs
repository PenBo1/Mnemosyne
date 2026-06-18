//! 消息清洗 — 角色交替校验与修复。
//!
//! 移植自 Hermes Agent 的 `agent/agent_runtime_helpers.py` 的 `repair_message_sequence`。
//! 确保消息列表遵循严格的角色交替：system → user/tool ↔ assistant。
//! 违反此规则会导致 API 返回空响应或报错。
//!
//! 修复规则：
//! 1. 删除孤立的 tool 消息（其 tool_call_id 不匹配任何前驱 assistant 的 tool_call）
//! 2. 合并连续的 user 消息（保留所有用户输入）
//! 3. 在 assistant 消息后如果没有 tool 结果，插入空 tool 结果

use crate::infra::llm::types::Message;

/// 消息清洗结果
#[derive(Debug, Clone)]
pub struct SanitizeResult {
    /// 是否进行了修复
    pub repaired: bool,
    /// 修复数量
    pub repair_count: usize,
    /// 描述
    pub description: String,
}

/// 清洗消息列表，确保角色交替正确。
///
/// # 规则
/// - 删除孤立的 tool 消息（tool_call_id 无匹配）
/// - 合并连续的 user 消息
/// - 确保 assistant(tool_calls) 后跟 tool 消息
pub fn sanitize_message_sequence(messages: &mut Vec<Message>) -> SanitizeResult {
    if messages.is_empty() {
        return SanitizeResult {
            repaired: false,
            repair_count: 0,
            description: "空消息列表".to_string(),
        };
    }

    let original_len = messages.len();
    let mut repairs = 0;

    // Pass 1: 删除孤立的 tool 消息
    let mut known_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut filtered: Vec<Message> = Vec::new();

    for msg in messages.drain(..) {
        match msg.role.as_str() {
            "assistant" => {
                // 收集此 assistant 消息的 tool_call_id
                known_tool_ids.clear();
                if let Some(ref tool_calls) = msg.tool_calls {
                    for tc in tool_calls {
                        known_tool_ids.insert(tc.id.clone());
                    }
                }
                filtered.push(msg);
            }
            "tool" => {
                // 检查 tool_call_id 是否匹配
                if let Some(ref tc_id) = msg.tool_call_id {
                    if known_tool_ids.contains(tc_id) {
                        filtered.push(msg);
                    } else {
                        repairs += 1;
                        // 孤立的 tool 消息 — 删除
                    }
                } else {
                    // 没有 tool_call_id — 保留（可能是旧格式）
                    filtered.push(msg);
                }
            }
            "user" => {
                // user 消息关闭 tool 结果序列
                known_tool_ids.clear();
                filtered.push(msg);
            }
            _ => {
                // system 或其他 — 保留
                filtered.push(msg);
            }
        }
    }

    // Pass 2: 合并连续的 user 消息
    let mut merged: Vec<Message> = Vec::new();
    for msg in filtered {
        if let Some(last) = merged.last_mut() {
            if last.role == "user" && msg.role == "user" {
                // 合并连续 user 消息
                last.content.push_str("\n\n");
                last.content.push_str(&msg.content);
                repairs += 1;
                continue;
            }
        }
        merged.push(msg);
    }

    *messages = merged;

    let repaired = repairs > 0;
    let description = if repaired {
        format!("修复了 {} 个问题（原始 {} 条消息 → {} 条）", repairs, original_len, messages.len())
    } else {
        format!("消息序列正常（{} 条消息）", messages.len())
    };

    SanitizeResult {
        repaired,
        repair_count: repairs,
        description,
    }
}

/// 验证消息序列是否有效（不修改，仅检查）。
///
/// # 返回值
/// Ok(()) 表示有效，Err(String) 描述问题。
pub fn validate_message_sequence(messages: &[Message]) -> Result<(), String> {
    if messages.is_empty() {
        return Ok(());
    }

    let mut known_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut last_role: Option<&str> = None;

    for (i, msg) in messages.iter().enumerate() {
        match msg.role.as_str() {
            "system" => {
                // system 只能在开头
                if last_role.is_some() {
                    return Err(format!("消息 {}: system 消息只能出现在开头", i));
                }
            }
            "user" => {
                // user 不能跟在 user 后面（除非中间有 assistant）
                if last_role == Some("user") {
                    return Err(format!("消息 {}: 连续的 user 消息", i));
                }
                known_tool_ids.clear();
            }
            "assistant" => {
                // assistant 后应该跟 tool 或 user
                known_tool_ids.clear();
                if let Some(ref tool_calls) = msg.tool_calls {
                    for tc in tool_calls {
                        known_tool_ids.insert(tc.id.clone());
                    }
                }
            }
            "tool" => {
                // tool 必须有 tool_call_id 且匹配
                match &msg.tool_call_id {
                    Some(tc_id) => {
                        if !known_tool_ids.contains(tc_id) {
                            return Err(format!(
                                "消息 {}: 孤立的 tool 消息 (tool_call_id={} 不匹配)",
                                i, tc_id
                            ));
                        }
                    }
                    None => {
                        return Err(format!("消息 {}: tool 消息缺少 tool_call_id", i));
                    }
                }
            }
            other => {
                return Err(format!("消息 {}: 未知角色 '{}'", i, other));
            }
        }
        last_role = Some(&msg.role);
    }

    Ok(())
}

/// 为缺少 tool 结果的 assistant(tool_calls) 补充空 tool 结果。
///
/// 这是 Hermes 的 defensive repair：如果 assistant 返回了 tool_calls 但
/// 某些 tool 结果丢失（例如子 Agent 崩溃），补充空结果防止 API 报错。
pub fn fill_missing_tool_results(messages: &mut Vec<Message>) -> usize {
    let mut filled = 0;
    let mut i = 0;

    while i < messages.len() {
        if messages[i].role == "assistant" {
            if let Some(ref tool_calls) = messages[i].tool_calls.clone() {
                if !tool_calls.is_empty() {
                    // 收集此 assistant 的所有 tool_call_id
                    let expected_ids: std::collections::HashSet<String> =
                        tool_calls.iter().map(|tc| tc.id.clone()).collect();

                    // 收集已有的 tool 结果 ID
                    let mut found_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
                    for j in (i + 1)..messages.len() {
                        if messages[j].role == "tool" {
                            if let Some(ref tc_id) = messages[j].tool_call_id {
                                if expected_ids.contains(tc_id) {
                                    found_ids.insert(tc_id.clone());
                                }
                            }
                        } else {
                            break; // 遇到非 tool 消息，停止
                        }
                    }

                    // 补充缺失的 tool 结果
                    for tc in tool_calls {
                        if !found_ids.contains(&tc.id) {
                            let placeholder = Message {
                                role: "tool".to_string(),
                                content: "[工具结果丢失 — 子进程可能已崩溃]".to_string(),
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                            };
                            // 在 assistant 消息后插入
                            let insert_pos = i + 1 + found_ids.len();
                            messages.insert(insert_pos, placeholder);
                            filled += 1;
                        }
                    }
                }
            }
        }
        i += 1;
    }

    filled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::llm::types::ToolCallRequest;

    fn user_msg(content: &str) -> Message {
        Message {
            role: "user".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn assistant_msg(content: &str) -> Message {
        Message {
            role: "assistant".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    fn tool_msg(content: &str, tc_id: &str) -> Message {
        Message {
            role: "tool".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(tc_id.to_string()),
        }
    }

    #[test]
    fn test_sanitize_removes_orphan_tool() {
        let mut messages = vec![
            user_msg("hello"),
            tool_msg("orphan result", "nonexistent_id"),
            assistant_msg("response"),
        ];
        let result = sanitize_message_sequence(&mut messages);
        assert!(result.repaired);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn test_sanitize_merges_consecutive_user() {
        let mut messages = vec![
            user_msg("hello"),
            user_msg("world"),
            assistant_msg("response"),
        ];
        let result = sanitize_message_sequence(&mut messages);
        assert!(result.repaired);
        assert_eq!(messages.len(), 2);
        assert!(messages[0].content.contains("hello"));
        assert!(messages[0].content.contains("world"));
    }

    #[test]
    fn test_validate_valid_sequence() {
        let messages = vec![
            user_msg("hello"),
            assistant_msg("response"),
            user_msg("follow up"),
        ];
        assert!(validate_message_sequence(&messages).is_ok());
    }

    #[test]
    fn test_validate_consecutive_user() {
        let messages = vec![user_msg("a"), user_msg("b")];
        assert!(validate_message_sequence(&messages).is_err());
    }

    #[test]
    fn test_fill_missing_tool_results() {
        let mut messages = vec![
            user_msg("do something"),
            Message {
                role: "assistant".into(),
                content: "".into(),
                tool_calls: Some(vec![
                    ToolCallRequest { id: "call_1".into(), name: "tool_a".into(), arguments: "{}".into() },
                    ToolCallRequest { id: "call_2".into(), name: "tool_b".into(), arguments: "{}".into() },
                ]),
                tool_call_id: None,
            },
            tool_msg("result 1", "call_1"),
            // call_2 缺失
        ];
        let filled = fill_missing_tool_results(&mut messages);
        assert_eq!(filled, 1);
        // 应该有 4 条消息：user + assistant + tool(call_1) + tool(call_2)
        assert_eq!(messages.len(), 4);
    }
}
