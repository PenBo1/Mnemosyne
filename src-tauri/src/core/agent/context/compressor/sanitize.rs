//! ═══════════════════════════════════════════════════════════════════════════
//! Compressor Sanitize - 工具对完整性修复
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 压缩后可能出现孤儿 tool_call / tool_result 对, 本模块负责修复这些不一致。

use crate::infrastructure::llm::types::{Message, ToolCallRequest};

/// 修复孤儿 tool_call / tool_result 对
/// 
/// 压缩后可能出现:
/// - tool 角色消息但其前驱 assistant 已被丢弃 → 删除该 tool 消息
/// - assistant 消息带 tool_calls 但部分/全部 id 无后续 tool 结果 → 移除无结果的 tool_call 条目
/// 
/// # 参数
/// - `messages`: 消息列表
/// 
/// # 返回值
/// 返回修复后的消息列表
pub fn sanitize_tool_pairs(messages: Vec<Message>) -> Vec<Message> {
    let n = messages.len();

    // Pass 1: 标记需要删除的孤儿 tool 消息
    let mut keep = vec![true; n];
    for (i, m) in messages.iter().enumerate() {
        if m.role != "tool" {
            continue;
        }
        let has_preceding = match &m.tool_call_id {
            Some(tcid) => messages[..i].iter().any(|prev| {
                prev.role == "assistant"
                    && prev
                        .tool_calls
                        .as_ref()
                        .map(|tcs| tcs.iter().any(|tc| &tc.id == tcid))
                        .unwrap_or(false)
            }),
            None => false,
        };
        if !has_preceding {
            keep[i] = false;
        }
    }

    // Pass 2: 计算每条 assistant 需保留的 tool_call 条目
    let mut filtered_tool_calls: Vec<Option<Vec<ToolCallRequest>>> = vec![None; n];
    for (i, m) in messages.iter().enumerate() {
        if m.role != "assistant" {
            continue;
        }
        if let Some(tcs) = m.tool_calls.as_ref() {
            let kept: Vec<ToolCallRequest> = tcs
                .iter()
                .filter(|tc| {
                    messages[i + 1..].iter().enumerate().any(|(j, prev)| {
                        let real_idx = i + 1 + j;
                        keep[real_idx]
                            && prev.role == "tool"
                            && prev.tool_call_id.as_ref() == Some(&tc.id)
                    })
                })
                .cloned()
                .collect();
            if kept.len() != tcs.len() {
                filtered_tool_calls[i] = Some(kept);
            }
        }
    }

    // Pass 3: 应用 keep + filtered_tool_calls
    let mut result = Vec::with_capacity(n);
    for (i, mut m) in messages.into_iter().enumerate() {
        if !keep[i] {
            continue;
        }
        if let Some(kept_tcs) = filtered_tool_calls[i].take() {
            if kept_tcs.is_empty() {
                m.tool_calls = None;
                if m.content.trim().is_empty() {
                    m.content = "[tool calls compressed away]".to_string();
                }
            } else {
                m.tool_calls = Some(kept_tcs);
            }
        }
        result.push(m);
    }
    result
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_support::*;

    #[test]
    fn sanitize_removes_orphan_tool_message() {
        let messages = vec![user_msg("hello"), tool_msg("orphan_id", "result")];
        let result = sanitize_tool_pairs(messages);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
    }

    #[test]
    fn sanitize_strips_orphan_tool_calls() {
        let messages = vec![
            user_msg("run tool"),
            assistant_with_tool_call("tc1", "search"),
            user_msg("next"),
        ];
        let result = sanitize_tool_pairs(messages);
        let assistant = result.iter().find(|m| m.role == "assistant").unwrap();
        assert!(assistant.tool_calls.is_none());
        assert!(!assistant.content.is_empty());
    }

    #[test]
    fn sanitize_preserves_matched_pairs() {
        let messages = vec![
            user_msg("run tool"),
            assistant_with_tool_call("tc1", "search"),
            tool_msg("tc1", "result"),
            assistant_msg("done"),
        ];
        let result = sanitize_tool_pairs(messages);
        assert_eq!(result.len(), 4);
        let assistant = result
            .iter()
            .find(|m| m.role == "assistant" && m.tool_calls.is_some());
        assert!(assistant.is_some());
    }

    #[test]
    fn sanitize_empty_input_returns_empty() {
        let result = sanitize_tool_pairs(Vec::new());
        assert!(result.is_empty());
    }
}