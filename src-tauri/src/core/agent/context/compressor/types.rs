//! ═══════════════════════════════════════════════════════════════════════════
//! Compressor Types - 压缩类型定义
//! ═══════════════════════════════════════════════════════════════════════════

/// Compaction 触发源
/// 
/// 不同触发源决定 InitialContextInjection 策略与 reference_context_item 是否保留。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionTrigger {
    /// token 接近上限自动触发
    Auto,
    /// 用户显式 `/compact` 命令触发
    Manual,
    /// 轮次内 token 超限触发
    MidTurn,
}

/// Initial Context 注入策略
/// 
/// 决定 replace_compacted_history 后 replacement history 的组装方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialContextInjection {
    /// 不注入, 清空 reference_context_item
    DoNotInject,
    /// 在最后真实用户消息前注入 initial context
    BeforeLastUserMessage,
}

impl CompactionTrigger {
    /// 根据触发源返回默认的 injection 策略
    pub fn default_injection(self) -> InitialContextInjection {
        match self {
            CompactionTrigger::Auto | CompactionTrigger::Manual => {
                InitialContextInjection::DoNotInject
            }
            CompactionTrigger::MidTurn => InitialContextInjection::BeforeLastUserMessage,
        }
    }

    /// 是否应在 compaction 后保留 reference_context_item
    pub fn preserves_reference_context_item(self) -> bool {
        matches!(self, CompactionTrigger::MidTurn)
    }
}

impl InitialContextInjection {
    /// 是否在 replacement history 中注入 initial context
    pub fn injects(self) -> bool {
        matches!(self, InitialContextInjection::BeforeLastUserMessage)
    }
}

/// Hook 中断原因
/// 
/// run_pre/post_compact_hooks 返回 Err(HaltReason) 时调用方应中止 compaction 流程。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaltReason {
    /// pre-compact hook 显式请求停止 compaction
    PreCompactHalted,
    /// post-compact hook 显式请求停止后续流程
    PostCompactHalted,
}

// ── 测试用例 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_default_injection_for_auto_is_do_not_inject() {
        assert_eq!(
            CompactionTrigger::Auto.default_injection(),
            InitialContextInjection::DoNotInject
        );
        assert!(!CompactionTrigger::Auto.preserves_reference_context_item());
    }

    #[test]
    fn trigger_default_injection_for_manual_is_do_not_inject() {
        assert_eq!(
            CompactionTrigger::Manual.default_injection(),
            InitialContextInjection::DoNotInject
        );
        assert!(!CompactionTrigger::Manual.preserves_reference_context_item());
    }

    #[test]
    fn trigger_default_injection_for_midturn_is_before_last_user_message() {
        assert_eq!(
            CompactionTrigger::MidTurn.default_injection(),
            InitialContextInjection::BeforeLastUserMessage
        );
        assert!(CompactionTrigger::MidTurn.preserves_reference_context_item());
    }

    #[test]
    fn injection_strategy_injects_flag() {
        assert!(!InitialContextInjection::DoNotInject.injects());
        assert!(InitialContextInjection::BeforeLastUserMessage.injects());
    }
}