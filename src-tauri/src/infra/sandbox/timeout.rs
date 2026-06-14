use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 超时强制执行器
pub struct TimeoutEnforcer {
    timeout: Duration,
    cancelled: Arc<AtomicBool>,
}

impl TimeoutEnforcer {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 获取取消信号
    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    /// 检查是否已超时
    pub fn is_expired(&self, start: Instant) -> bool {
        start.elapsed() >= self.timeout
    }

    /// 等待超时或取消
    pub fn wait(&self) -> TimeoutResult {
        let start = Instant::now();
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                return TimeoutResult::Cancelled;
            }
            if start.elapsed() >= self.timeout {
                return TimeoutResult::TimedOut;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// 启动超时监控线程
    pub fn spawn_monitor(&self) -> std::thread::JoinHandle<()> {
        let timeout = self.timeout;
        let cancelled = self.cancelled.clone();
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            if !cancelled.load(Ordering::Relaxed) {
                cancelled.store(true, Ordering::Relaxed);
            }
        })
    }

    /// 取消操作
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// 重置计时器
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutResult {
    Completed,
    TimedOut,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_enforcer() {
        let enforcer = TimeoutEnforcer::new(1);
        assert!(!enforcer.is_expired(Instant::now()));
        assert!(enforcer.is_expired(Instant::now() - Duration::from_secs(2)));
    }

    #[test]
    fn test_cancellation() {
        let enforcer = TimeoutEnforcer::new(60);
        let token = enforcer.cancel_token();
        assert!(!token.load(Ordering::Relaxed));
        enforcer.cancel();
        assert!(token.load(Ordering::Relaxed));
    }
}
