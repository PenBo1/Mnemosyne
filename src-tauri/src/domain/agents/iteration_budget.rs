//! 迭代预算控制 — 线程安全的计数器，防止 Agent 工具调用无限循环。
//!
//! 移植自 Hermes Agent 的 `agent/iteration_budget.py`。
//! 每个 Agent 实例（父 Agent 或子 Agent）持有独立的 `IterationBudget`；
//! 父 Agent 的上限来自 `max_iterations`，子 Agent 的上限来自 `delegation.max_iterations`。

use std::sync::atomic::{AtomicUsize, Ordering};
use serde::{Serialize, Deserialize};

/// 线程安全的迭代预算计数器。
///
/// 用于控制 Agent 循环中的工具调用次数，防止无限循环。
/// `execute_code` 等编程式工具调用的迭代可以通过 `refund()` 退还。
#[derive(Debug, Serialize, Deserialize)]
pub struct IterationBudget {
    /// 最大允许迭代次数
    max_total: usize,
    /// 已使用次数（原子操作，线程安全）
    #[serde(skip)]
    used: AtomicUsize,
}

impl IterationBudget {
    /// 创建新的迭代预算。
    ///
    /// # 参数
    /// - `max_total`: 最大允许的迭代次数
    pub fn new(max_total: usize) -> Self {
        Self {
            max_total,
            used: AtomicUsize::new(0),
        }
    }

    /// 尝试消耗一次迭代。
    ///
    /// # 返回值
    /// - `true`: 成功消耗，可以继续执行
    /// - `false`: 已达预算上限，应停止工具调用循环
    pub fn consume(&self) -> bool {
        let current = self.used.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_total {
            // 超出预算，回退计数
            self.used.fetch_sub(1, Ordering::SeqCst);
            false
        } else {
            true
        }
    }

    /// 退还一次迭代（例如 `execute_code` 的编程式调用不应消耗预算）。
    pub fn refund(&self) {
        let current = self.used.load(Ordering::SeqCst);
        if current > 0 {
            self.used.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// 获取已使用的迭代次数。
    pub fn used(&self) -> usize {
        self.used.load(Ordering::SeqCst)
    }

    /// 获取剩余可用的迭代次数（最小为 0）。
    pub fn remaining(&self) -> usize {
        let used = self.used.load(Ordering::SeqCst);
        self.max_total.saturating_sub(used)
    }

    /// 重置已使用计数为 0。
    pub fn reset(&self) {
        self.used.store(0, Ordering::SeqCst);
    }

    /// 获取最大迭代次数。
    pub fn max_total(&self) -> usize {
        self.max_total
    }

    /// 预算是否已耗尽。
    pub fn is_exhausted(&self) -> bool {
        self.used.load(Ordering::SeqCst) >= self.max_total
    }
}

impl Clone for IterationBudget {
    fn clone(&self) -> Self {
        Self {
            max_total: self.max_total,
            used: AtomicUsize::new(self.used.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_consume() {
        let budget = IterationBudget::new(3);
        assert!(budget.consume());
        assert!(budget.consume());
        assert!(budget.consume());
        assert!(!budget.consume());
        assert_eq!(budget.used(), 3);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn test_refund() {
        let budget = IterationBudget::new(2);
        assert!(budget.consume());
        assert!(budget.consume());
        assert!(!budget.consume());
        budget.refund();
        assert!(budget.consume());
        assert!(!budget.consume());
    }

    #[test]
    fn test_reset() {
        let budget = IterationBudget::new(2);
        budget.consume();
        budget.consume();
        assert!(!budget.consume());
        budget.reset();
        assert!(budget.consume());
    }

    #[test]
    fn test_is_exhausted() {
        let budget = IterationBudget::new(1);
        assert!(!budget.is_exhausted());
        budget.consume();
        assert!(budget.is_exhausted());
    }
}
