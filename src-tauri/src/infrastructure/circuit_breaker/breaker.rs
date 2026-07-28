//! ═══════════════════════════════════════════════════════════════════════════
//! 断路器 - 核心断路器实现
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::config::CircuitBreakerConfig;
use super::state::{AtomicCircuitState, CircuitState};
use super::window::{RequestOutcome, SlidingWindow, SlidingWindowSnapshot};

// ── 断路器统计信息 ───────────────────────────────────────────────────────────

/// 断路器运行统计数据
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// 当前状态
    pub state: CircuitState,
    /// 滑动窗口快照
    pub window: SlidingWindowSnapshot,
    /// 最后一次失败时间
    pub last_failure_time: Option<Instant>,
    /// 半开状态下的成功次数
    pub half_open_successes: usize,
    /// 总请求数
    pub total_requests: u64,
    /// 总失败数
    pub total_failures: u64,
    /// 总打开次数
    pub total_opens: u64,
}

/// 断路器操作结果类型
pub type CircuitResult<T> = Result<T, CircuitBreakerError>;

/// 断路器错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum CircuitBreakerError {
    /// 断路器处于打开状态
    #[error("Circuit breaker is open")]
    Open,
    /// 断路器处于半开状态，仅允许有限请求
    #[error("Circuit breaker is in half-open state, only limited requests allowed")]
    HalfOpen,
}

// ── 断路器内部状态 ───────────────────────────────────────────────────────────

/// 断路器内部可变状态
#[derive(Debug)]
struct CircuitBreakerInner {
    /// 滑动窗口
    window: SlidingWindow,
    /// 最后一次失败时间
    last_failure_time: Option<Instant>,
    /// 半开状态下的成功次数
    half_open_successes: usize,
    /// 打开时间
    open_time: Option<Instant>,
    /// 总请求数
    total_requests: u64,
    /// 总失败数
    total_failures: u64,
    /// 总打开次数
    total_opens: u64,
}

// ── 断路器 ───────────────────────────────────────────────────────────────────

/// 断路器实现，用于防止级联故障
#[derive(Debug)]
pub struct CircuitBreaker {
    /// 断路器名称
    name: String,
    /// 断路器配置
    config: CircuitBreakerConfig,
    /// 原子状态
    state: AtomicCircuitState,
    /// 内部可变状态
    inner: Mutex<CircuitBreakerInner>,
    /// 半开探测标志
    is_half_open_probe: AtomicBool,
}

impl CircuitBreaker {
    /// 创建新的断路器实例
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let window = SlidingWindow::new(config.window_size, config.timeout);
        
        Self {
            name: name.into(),
            config,
            state: AtomicCircuitState::new(CircuitState::Closed),
            inner: Mutex::new(CircuitBreakerInner {
                window,
                last_failure_time: None,
                half_open_successes: 0,
                open_time: None,
                total_requests: 0,
                total_failures: 0,
                total_opens: 0,
            }),
            is_half_open_probe: AtomicBool::new(false),
        }
    }

    /// 获取断路器名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取当前状态
    pub fn state(&self) -> CircuitState {
        self.state.load(Ordering::Acquire)
    }

    /// 检查请求是否被允许
    pub fn is_request_allowed(&self) -> bool {
        self.state.is_request_allowed(Ordering::Acquire)
    }

    /// 检查是否可以执行请求
    pub fn check(&self) -> CircuitResult<()> {
        let current_state = self.state.load(Ordering::Acquire);
        
        match current_state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                let inner = self.inner.lock().expect("circuit breaker lock poisoned");
                if let Some(open_time) = inner.open_time {
                    if Instant::now().duration_since(open_time) >= self.config.timeout {
                        drop(inner);
                        self.try_half_open();
                        return Ok(());
                    }
                }
                Err(CircuitBreakerError::Open)
            }
            CircuitState::HalfOpen => {
                if self.is_half_open_probe.compare_exchange(
                    false,
                    true,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ).is_ok() {
                    Ok(())
                } else {
                    Err(CircuitBreakerError::HalfOpen)
                }
            }
        }
    }

    /// 记录成功请求
    pub fn record_success(&self) {
        let current_state = self.state.load(Ordering::Acquire);
        
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        inner.window.record(RequestOutcome::Success);
        inner.total_requests += 1;
        
        match current_state {
            CircuitState::HalfOpen => {
                inner.half_open_successes += 1;
                self.is_half_open_probe.store(false, Ordering::Release);
                
                if inner.half_open_successes >= self.config.success_threshold {
                    self.transition_to_closed(&mut inner);
                }
            }
            CircuitState::Closed => {
                let snapshot = inner.window.snapshot();
                if self.config.should_open(snapshot.failures, snapshot.total) {
                    self.transition_to_open(&mut inner);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// 记录失败请求
    pub fn record_failure(&self) {
        let current_state = self.state.load(Ordering::Acquire);
        
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        inner.window.record(RequestOutcome::Failure);
        inner.total_requests += 1;
        inner.total_failures += 1;
        inner.last_failure_time = Some(Instant::now());
        
        match current_state {
            CircuitState::HalfOpen => {
                inner.half_open_successes = 0;
                self.is_half_open_probe.store(false, Ordering::Release);
                self.transition_to_open(&mut inner);
            }
            CircuitState::Closed => {
                let snapshot = inner.window.snapshot();
                if self.config.should_open(snapshot.failures, snapshot.total) {
                    self.transition_to_open(&mut inner);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// 获取断路器统计数据
    pub fn stats(&self) -> CircuitBreakerStats {
        let inner = self.inner.lock().expect("circuit breaker lock poisoned");
        
        CircuitBreakerStats {
            state: self.state.load(Ordering::Acquire),
            window: inner.window.snapshot(),
            last_failure_time: inner.last_failure_time,
            half_open_successes: inner.half_open_successes,
            total_requests: inner.total_requests,
            total_failures: inner.total_failures,
            total_opens: inner.total_opens,
        }
    }

    /// 重置断路器状态
    pub fn reset(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        self.state.store(CircuitState::Closed, Ordering::Release);
        inner.window.reset();
        inner.half_open_successes = 0;
        inner.open_time = None;
        inner.last_failure_time = None;
    }

    /// 强制打开断路器
    pub fn force_open(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        self.transition_to_open(&mut inner);
    }

    /// 强制关闭断路器
    pub fn force_closed(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        self.transition_to_closed(&mut inner);
    }

    /// 尝试切换到半开状态
    fn try_half_open(&self) {
        let result = self.state.compare_exchange(
            CircuitState::Open,
            CircuitState::HalfOpen,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
        
        if result.is_ok() {
            let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
            inner.half_open_successes = 0;
            self.is_half_open_probe.store(false, Ordering::Release);
        }
    }

    /// 切换到打开状态
    fn transition_to_open(&self, inner: &mut CircuitBreakerInner) {
        self.state.store(CircuitState::Open, Ordering::Release);
        inner.open_time = Some(Instant::now());
        inner.total_opens += 1;
        self.is_half_open_probe.store(false, Ordering::Release);
    }

    /// 切换到关闭状态
    fn transition_to_closed(&self, inner: &mut CircuitBreakerInner) {
        self.state.store(CircuitState::Closed, Ordering::Release);
        inner.half_open_successes = 0;
        inner.open_time = None;
        self.is_half_open_probe.store(false, Ordering::Release);
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new("default", CircuitBreakerConfig::default())
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_closed_state_allows_requests() {
        let breaker = CircuitBreaker::default();
        
        assert!(breaker.check().is_ok());
        assert!(breaker.is_request_allowed());
    }

    #[test]
    fn test_opens_after_threshold_failures() {
        let config = CircuitBreakerConfig::new(3, 2, Duration::from_secs(30), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        for _ in 0..3 {
            breaker.record_failure();
        }
        
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(breaker.check().is_err());
    }

    #[test]
    fn test_half_open_after_timeout() {
        let config = CircuitBreakerConfig::new(1, 1, Duration::from_millis(50), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        thread::sleep(Duration::from_millis(100));
        
        assert!(breaker.check().is_ok());
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_to_closed_on_success() {
        let config = CircuitBreakerConfig::new(1, 2, Duration::from_millis(50), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        thread::sleep(Duration::from_millis(100));
        
        breaker.check().expect("half-open should allow");
        breaker.record_success();
        
        breaker.check().expect("half-open should allow");
        breaker.record_success();
        
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_to_open_on_failure() {
        let config = CircuitBreakerConfig::new(1, 1, Duration::from_millis(50), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        thread::sleep(Duration::from_millis(100));
        
        breaker.check().expect("half-open should allow");
        breaker.record_failure();
        
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_stats() {
        let breaker = CircuitBreaker::default();
        
        breaker.record_success();
        breaker.record_success();
        breaker.record_failure();
        
        let stats = breaker.stats();
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.total_failures, 1);
        assert_eq!(stats.window.total, 3);
        assert_eq!(stats.window.failures, 1);
    }

    #[test]
    fn test_reset() {
        let config = CircuitBreakerConfig::new(1, 1, Duration::from_secs(30), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        breaker.reset();
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        let stats = breaker.stats();
        assert_eq!(stats.window.total, 0);
    }

    #[test]
    fn test_force_open() {
        let breaker = CircuitBreaker::default();
        
        breaker.force_open();
        
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(breaker.check().is_err());
    }

    #[test]
    fn test_force_closed() {
        let config = CircuitBreakerConfig::new(1, 1, Duration::from_secs(30), 10);
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        breaker.force_closed();
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.check().is_ok());
    }
}