//! ═══════════════════════════════════════════════════════════════════════════
//! TokenBudgetReminderFragment - Token 预算提醒 fragment
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// 默认提醒阈值（tokens）：剩余 token 低于此值时注入提醒。
///
/// 参考值：~2K tokens（约等于 1-2 轮对话的余量）。
pub const DEFAULT_REMINDER_THRESHOLD_TOKENS: u64 = 2_000;

/// token budget 提醒 fragment。
///
/// `should_inject` 判断是否满足注入条件；`claim_token_budget_reminder` 标记
/// 已注入（去重）。注入后同一实例不再注入，需重新构造才会重置状态。
pub struct TokenBudgetReminderFragment {
    /// 剩余 token 数（距下次 compaction）
    tokens_until_compaction: u64,
    /// 提醒阈值
    reminder_threshold_tokens: u64,
    /// 是否已注入过（去重标记，原子操作保证线程安全）
    claimed: AtomicBool,
}

impl TokenBudgetReminderFragment {
    /// 用默认阈值构造。
    pub fn new(tokens_until_compaction: u64) -> Self {
        Self::with_threshold(
            tokens_until_compaction,
            DEFAULT_REMINDER_THRESHOLD_TOKENS,
        )
    }

    /// 用自定义阈值构造。
    pub fn with_threshold(
        tokens_until_compaction: u64,
        reminder_threshold_tokens: u64,
    ) -> Self {
        Self {
            tokens_until_compaction,
            reminder_threshold_tokens,
            claimed: AtomicBool::new(false),
        }
    }

    /// 是否应注入提醒（剩余 token ≤ 阈值 且 未注入过）。
    pub fn should_inject(&self) -> bool {
        self.tokens_until_compaction <= self.reminder_threshold_tokens
            && !self.claimed.load(Ordering::SeqCst)
    }

    /// 标记已注入（去重）。
    ///
    /// 返回是否成功 claim（true = 首次注入，false = 已注入过）。
    pub fn claim_token_budget_reminder(&self) -> bool {
        self.claimed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// 渲染提醒文本。
    fn render_reminder(&self) -> String {
        format!(
            "<token_budget_reminder>\n\
             Token budget is running low: {} tokens remaining until context compaction.\n\
             Consider wrapping up the current task or invoking /compact to free up context.\n\
             </token_budget_reminder>",
            self.tokens_until_compaction
        )
    }
}

impl ContextualUserFragment for TokenBudgetReminderFragment {
    fn name(&self) -> &str {
        "token_budget_reminder"
    }

    fn render_full(&self) -> Result<String, AppError> {
        if self.should_inject() {
            // 渲染时自动 claim（首次渲染后标记为已注入）
            self.claim_token_budget_reminder();
            Ok(self.render_reminder())
        } else {
            // 不满足条件或已注入：渲染为空字符串（registry 会跳过空内容）
            Ok(String::new())
        }
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(128)
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::Developer
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_inject_when_above_threshold() {
        let frag = TokenBudgetReminderFragment::new(10_000);
        assert!(!frag.should_inject());
        assert_eq!(frag.render_full().unwrap(), "");
    }

    #[test]
    fn injects_when_at_or_below_threshold() {
        let frag = TokenBudgetReminderFragment::new(2_000);
        assert!(frag.should_inject());

        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("<token_budget_reminder>"));
        assert!(rendered.contains("2000 tokens remaining"));
    }

    #[test]
    fn does_not_inject_twice_after_claim() {
        let frag = TokenBudgetReminderFragment::new(1_000);

        let first = frag.render_full().unwrap();
        assert!(!first.is_empty(), "首次应注入");

        let second = frag.render_full().unwrap();
        assert!(second.is_empty(), "已 claim 后不应再注入");
    }

    #[test]
    fn claim_returns_true_only_once() {
        let frag = TokenBudgetReminderFragment::new(1_000);
        assert!(frag.claim_token_budget_reminder(), "首次 claim 应成功");
        assert!(
            !frag.claim_token_budget_reminder(),
            "二次 claim 应失败（已注入）"
        );
    }

    #[test]
    fn custom_threshold_respected() {
        // 阈值 5K，剩余 4K → 应注入
        let frag = TokenBudgetReminderFragment::with_threshold(4_000, 5_000);
        assert!(frag.should_inject());

        // 阈值 5K，剩余 6K → 不应注入
        let frag2 = TokenBudgetReminderFragment::with_threshold(6_000, 5_000);
        assert!(!frag2.should_inject());
    }

    #[test]
    fn fragment_metadata() {
        let frag = TokenBudgetReminderFragment::new(1_000);
        assert_eq!(frag.name(), "token_budget_reminder");
        assert_eq!(frag.bounded_size(), Some(128));
        assert_eq!(frag.role(), FragmentRole::Developer);
    }

    #[test]
    fn zero_tokens_triggers_injection() {
        let frag = TokenBudgetReminderFragment::new(0);
        assert!(frag.should_inject());
        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("0 tokens remaining"));
    }
}
