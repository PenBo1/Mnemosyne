//! ═══════════════════════════════════════════════════════════════════════════
//! Compressor Hooks - 压缩 Hook 系统
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! Hook 系统: CompactHook trait、pre/post-compact hook 运行、initial context 插入。
//! 允许上层 (如 security_kernel 审批、curator) 介入 compaction 流程。

use std::sync::Arc;

use crate::infrastructure::llm::types::Message;

use super::types::{CompactionTrigger, HaltReason};

// ── CompactHook Trait ───────────────────────────────────────────────────────

/// Hook 回调 trait
/// 
/// 允许上层介入 compaction 流程:
/// - pre-compact hook 在 compaction 开始前调用, 返回 false 中止整个 compaction
/// - post-compact hook 在 compaction 完成后调用, 返回 false 中止后续 turn
pub trait CompactHook: Send + Sync {
    /// pre-compact 检查
    /// 
    /// 返回 false 中止 compaction。
    fn pre_compact(&self, trigger: CompactionTrigger) -> bool {
        let _ = trigger;
        true
    }

    /// post-compact 检查
    /// 
    /// 返回 false 中止后续 turn。
    fn post_compact(&self, trigger: CompactionTrigger) -> bool {
        let _ = trigger;
        true
    }
}

// ── Hook 运行函数 ────────────────────────────────────────────────────────────

/// 运行所有 pre-compact hooks
/// 
/// 任一返回 false 即返回 Err(HaltReason::PreCompactHalted)。
pub async fn run_pre_compact_hooks(
    hooks: &[Arc<dyn CompactHook>],
    trigger: CompactionTrigger,
) -> Result<(), HaltReason> {
    for hook in hooks {
        if !hook.pre_compact(trigger) {
            return Err(HaltReason::PreCompactHalted);
        }
    }
    Ok(())
}

/// 运行所有 post-compact hooks
/// 
/// 任一返回 false 即返回 Err(HaltReason::PostCompactHalted)。
pub async fn run_post_compact_hooks(
    hooks: &[Arc<dyn CompactHook>],
    trigger: CompactionTrigger,
) -> Result<(), HaltReason> {
    for hook in hooks {
        if !hook.post_compact(trigger) {
            return Err(HaltReason::PostCompactHalted);
        }
    }
    Ok(())
}

// ── Initial Context 插入 ────────────────────────────────────────────────────

/// 在最后一条真实用户消息前插入 initial context
/// 
/// MidTurn compaction 的 replacement history 必须保留用户最后一条原始消息作为末尾项,
/// 因此 initial context 需要插入到最后一条真实 user 消息之前。
/// 
/// # 参数
/// - `history`: 历史消息列表
/// - `initial_context`: 要插入的初始上下文
/// 
/// # 返回值
/// 返回插入后的历史消息列表
pub fn insert_initial_context_before_last_user_message(
    mut history: Vec<Message>,
    initial_context: Vec<Message>,
) -> Vec<Message> {
    // 从尾部扫描, 找最后一条真实 user 消息
    let last_real_user_idx = history.iter().enumerate().rev().find_map(|(i, m)| {
        if m.role == "user" && !m.content.starts_with(super::SUMMARY_MARKER) {
            Some(i)
        } else {
            None
        }
    });

    let insertion_idx = match last_real_user_idx {
        Some(i) => Some(i),
        None => {
            // 回退: 找最后一条 summary 消息, 插到其前
            history.iter().enumerate().rev().find_map(|(i, m)| {
                if m.role == "user" && m.content.starts_with(super::SUMMARY_MARKER) {
                    Some(i)
                } else {
                    None
                }
            })
        }
    };

    match insertion_idx {
        Some(i) => {
            history.splice(i..i, initial_context);
        }
        None => {
            // 无 user/summary 消息, 直接 append
            history.extend(initial_context);
        }
    }

    history
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_support::*;

    #[test]
    fn insert_initial_context_before_last_real_user_message() {
        let summary = format!(
            "{}\n{}\n{{}}",
            super::super::SUMMARY_MARKER,
            super::super::SUMMARY_HIJACK_GUARD
        );
        let history = vec![
            Message {
                role: "user".to_string(),
                content: summary,
                tool_calls: None,
                tool_call_id: None,
            },
            user_msg("final real user message"),
        ];
        let initial = vec![Message {
            role: "system".to_string(),
            content: "[INITIAL_CONTEXT]".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];
        let result = insert_initial_context_before_last_user_message(history, initial);
        assert_eq!(result.len(), 3);
        assert_eq!(result[2].content, "final real user message");
        assert_eq!(result[1].content, "[INITIAL_CONTEXT]");
        assert!(result[0].content.starts_with(super::super::SUMMARY_MARKER));
    }

    #[test]
    fn insert_initial_context_appends_when_no_user_or_summary() {
        let history = vec![assistant_msg("only assistant")];
        let initial = vec![Message {
            role: "system".to_string(),
            content: "[INITIAL_CONTEXT]".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];
        let result = insert_initial_context_before_last_user_message(history, initial);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "only assistant");
        assert_eq!(result[1].content, "[INITIAL_CONTEXT]");
    }

    #[test]
    fn insert_initial_context_empty_initial_is_noop() {
        let history = vec![user_msg("hi"), assistant_msg("hello")];
        let result = insert_initial_context_before_last_user_message(history, vec![]);
        assert_eq!(result.len(), 2);
    }

    struct AlwaysContinueHook;
    impl CompactHook for AlwaysContinueHook {}

    struct AlwaysHaltHook;
    impl CompactHook for AlwaysHaltHook {
        fn pre_compact(&self, _trigger: CompactionTrigger) -> bool {
            false
        }
        fn post_compact(&self, _trigger: CompactionTrigger) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn run_pre_compact_hooks_continue_when_empty() {
        let result = run_pre_compact_hooks(&[], CompactionTrigger::Auto).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_pre_compact_hooks_continue_with_always_continue_hook() {
        let hook: Arc<dyn CompactHook> = Arc::new(AlwaysContinueHook);
        let result = run_pre_compact_hooks(&[hook], CompactionTrigger::Manual).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_pre_compact_hooks_halt_when_hook_returns_false() {
        let hook: Arc<dyn CompactHook> = Arc::new(AlwaysHaltHook);
        let result = run_pre_compact_hooks(&[hook], CompactionTrigger::Auto).await;
        assert_eq!(result, Err(HaltReason::PreCompactHalted));
    }
}