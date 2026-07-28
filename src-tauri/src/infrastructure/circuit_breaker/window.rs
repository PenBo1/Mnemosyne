//! ═══════════════════════════════════════════════════════════════════════════
//! 滑动窗口 - 请求结果统计窗口
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ── 请求结果类型 ────────────────────────────────────────────────────────────

/// 请求执行结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// 成功
    Success,
    /// 失败
    Failure,
}

/// 带时间戳的请求结果
#[derive(Debug, Clone)]
struct TimestampedOutcome {
    /// 请求结果
    outcome: RequestOutcome,
    /// 时间戳
    timestamp: Instant,
}

impl TimestampedOutcome {
    /// 创建新的带时间戳结果
    fn new(outcome: RequestOutcome) -> Self {
        Self {
            outcome,
            timestamp: Instant::now(),
        }
    }
}

// ── 滑动窗口快照 ────────────────────────────────────────────────────────────

/// 滑动窗口统计快照
#[derive(Debug, Clone)]
pub struct SlidingWindowSnapshot {
    /// 总请求数
    pub total: usize,
    /// 成功数
    pub successes: usize,
    /// 失败数
    pub failures: usize,
    /// 错误率
    pub error_rate: f64,
}

impl Default for SlidingWindowSnapshot {
    fn default() -> Self {
        Self {
            total: 0,
            successes: 0,
            failures: 0,
            error_rate: 0.0,
        }
    }
}

// ── 滑动窗口实现 ────────────────────────────────────────────────────────────

/// 滑动窗口，用于统计请求结果
#[derive(Debug)]
pub struct SlidingWindow {
    /// 窗口内部状态
    window: Mutex<SlidingWindowInner>,
    /// 窗口大小
    window_size: usize,
    /// 窗口持续时间
    window_duration: Duration,
}

/// 滑动窗口内部状态
#[derive(Debug)]
#[derive(Default)]
struct SlidingWindowInner {
    /// 带时间戳的结果队列
    outcomes: VecDeque<TimestampedOutcome>,
    /// 总请求数
    total: usize,
    /// 成功数
    successes: usize,
    /// 失败数
    failures: usize,
}


impl SlidingWindow {
    /// 创建新的滑动窗口
    pub fn new(window_size: usize, window_duration: Duration) -> Self {
        Self {
            window: Mutex::new(SlidingWindowInner::default()),
            window_size,
            window_duration,
        }
    }

    /// 记录请求结果
    pub fn record(&self, outcome: RequestOutcome) {
        let mut inner = self.window.lock().expect("window lock poisoned");
        
        let timestamped = TimestampedOutcome::new(outcome);
        inner.outcomes.push_back(timestamped);
        
        inner.total += 1;
        match outcome {
            RequestOutcome::Success => inner.successes += 1,
            RequestOutcome::Failure => inner.failures += 1,
        }
        
        self.evict_expired(&mut inner);
        
        while inner.outcomes.len() > self.window_size {
            if let Some(removed) = inner.outcomes.pop_front() {
                inner.total -= 1;
                match removed.outcome {
                    RequestOutcome::Success => inner.successes -= 1,
                    RequestOutcome::Failure => inner.failures -= 1,
                }
            }
        }
    }

    /// 获取窗口统计快照
    pub fn snapshot(&self) -> SlidingWindowSnapshot {
        let inner = self.window.lock().expect("window lock poisoned");
        
        let error_rate = if inner.total == 0 {
            0.0
        } else {
            inner.failures as f64 / inner.total as f64
        };
        
        SlidingWindowSnapshot {
            total: inner.total,
            successes: inner.successes,
            failures: inner.failures,
            error_rate,
        }
    }

    /// 重置窗口
    pub fn reset(&self) {
        let mut inner = self.window.lock().expect("window lock poisoned");
        inner.outcomes.clear();
        inner.total = 0;
        inner.successes = 0;
        inner.failures = 0;
    }

    /// 清除过期记录
    fn evict_expired(&self, inner: &mut SlidingWindowInner) {
        let cutoff = Instant::now() - self.window_duration;
        
        while let Some(front) = inner.outcomes.front() {
            if front.timestamp < cutoff {
                let removed = inner.outcomes.pop_front().expect("front exists");
                inner.total -= 1;
                match removed.outcome {
                    RequestOutcome::Success => inner.successes -= 1,
                    RequestOutcome::Failure => inner.failures -= 1,
                }
            } else {
                break;
            }
        }
    }
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self::new(100, Duration::from_secs(60))
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_success() {
        let window = SlidingWindow::new(10, Duration::from_secs(60));
        
        window.record(RequestOutcome::Success);
        
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total, 1);
        assert_eq!(snapshot.successes, 1);
        assert_eq!(snapshot.failures, 0);
        assert_eq!(snapshot.error_rate, 0.0);
    }

    #[test]
    fn test_record_failure() {
        let window = SlidingWindow::new(10, Duration::from_secs(60));
        
        window.record(RequestOutcome::Failure);
        
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total, 1);
        assert_eq!(snapshot.successes, 0);
        assert_eq!(snapshot.failures, 1);
        assert_eq!(snapshot.error_rate, 1.0);
    }

    #[test]
    fn test_error_rate_calculation() {
        let window = SlidingWindow::new(10, Duration::from_secs(60));
        
        for _ in 0..3 {
            window.record(RequestOutcome::Success);
        }
        for _ in 0..2 {
            window.record(RequestOutcome::Failure);
        }
        
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total, 5);
        assert_eq!(snapshot.successes, 3);
        assert_eq!(snapshot.failures, 2);
        assert!((snapshot.error_rate - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_window_size_limit() {
        let window = SlidingWindow::new(3, Duration::from_secs(60));
        
        window.record(RequestOutcome::Success);
        window.record(RequestOutcome::Success);
        window.record(RequestOutcome::Success);
        window.record(RequestOutcome::Failure);
        
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total, 3);
        assert_eq!(snapshot.failures, 1);
    }

    #[test]
    fn test_reset() {
        let window = SlidingWindow::new(10, Duration::from_secs(60));
        
        window.record(RequestOutcome::Success);
        window.record(RequestOutcome::Failure);
        
        window.reset();
        
        let snapshot = window.snapshot();
        assert_eq!(snapshot.total, 0);
    }
}