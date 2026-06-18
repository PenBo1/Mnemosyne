//! 重试工具 — 抖动退避（jittered backoff）。
//!
//! 移植自 Hermes Agent 的 `agent/retry_utils.py`。
//! 使用抖动退避代替固定指数退避，防止多个会话同时重试时的惊群效应。

use std::sync::atomic::{AtomicUsize, Ordering};

/// 全局抖动计数器（进程内唯一，防止并发重试路径的种子冲突）
static JITTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 计算抖动退避延迟。
///
/// # 参数
/// - `attempt`: 1-based 重试次数
/// - `base_delay`: 基础延迟（秒），默认 2.0
/// - `max_delay`: 最大延迟上限（秒），默认 60.0
/// - `jitter_ratio`: 抖动比例，默认 0.5（延迟的 0~50% 随机抖动）
///
/// # 返回值
/// 延迟秒数: min(base * 2^(attempt-1), max_delay) + jitter
///
/// 抖动使并发重试去相关化，避免多个会话同时重试同一限流提供者。
pub fn jittered_backoff(
    attempt: u32,
    base_delay: f64,
    max_delay: f64,
    jitter_ratio: f64,
) -> f64 {
    let tick = JITTER_COUNTER.fetch_add(1, Ordering::SeqCst);

    let exponent = attempt.saturating_sub(1);
    let delay = if exponent >= 63 || base_delay <= 0.0 {
        max_delay
    } else {
        let raw = base_delay * (2.0_f64.powi(exponent as i32));
        raw.min(max_delay)
    };

    // 使用时间戳 + 计数器作为种子，即使时钟粗糙也能去相关
    let seed = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64)
        ^ (tick as u64).wrapping_mul(0x9E3779B9);
    let jitter = pseudo_random_uniform(seed, 0.0, jitter_ratio * delay);

    delay + jitter
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础延迟（秒）
    pub base_delay: f64,
    /// 最大延迟上限（秒）
    pub max_delay: f64,
    /// 抖动比例
    pub jitter_ratio: f64,
    /// 可重试的 HTTP 状态码
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: 2.0,
            max_delay: 60.0,
            jitter_ratio: 0.5,
            retryable_status_codes: vec![429, 500, 502, 503, 529],
        }
    }
}

/// 重试状态追踪器
pub struct RetryState {
    /// 当前重试次数
    pub count: u32,
    /// 配置
    config: RetryConfig,
}

impl RetryState {
    pub fn new(config: RetryConfig) -> Self {
        Self { count: 0, config }
    }

    /// 重置重试计数
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// 是否还可以重试
    pub fn can_retry(&self) -> bool {
        self.count < self.config.max_retries
    }

    /// 增加重试计数并返回退避延迟
    pub fn next_backoff(&mut self) -> f64 {
        self.count += 1;
        jittered_backoff(
            self.count,
            self.config.base_delay,
            self.config.max_delay,
            self.config.jitter_ratio,
        )
    }

    /// 获取当前重试次数
    pub fn attempt(&self) -> u32 {
        self.count
    }

    /// 最大重试次数
    pub fn max_retries(&self) -> u32 {
        self.config.max_retries
    }
}

/// 简单的伪随机均匀分布（无外部依赖）
fn pseudo_random_uniform(seed: u64, min: f64, max: f64) -> f64 {
    // xorshift64
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let normalized = (x as f64) / (u64::MAX as f64);
    min + normalized * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let d1 = jittered_backoff(1, 2.0, 60.0, 0.5);
        let d2 = jittered_backoff(2, 2.0, 60.0, 0.5);
        let d3 = jittered_backoff(3, 2.0, 60.0, 0.5);
        // 基础延迟递增: 2, 4, 8
        assert!(d1 >= 2.0);
        assert!(d2 >= 4.0);
        assert!(d3 >= 8.0);
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let d = jittered_backoff(100, 2.0, 60.0, 0.5);
        assert!(d <= 60.0 + 30.0); // max_delay + max_jitter
    }

    #[test]
    fn test_retry_state() {
        let config = RetryConfig { max_retries: 3, ..Default::default() };
        let mut state = RetryState::new(config);
        assert!(state.can_retry());
        state.next_backoff();
        state.next_backoff();
        state.next_backoff();
        assert!(!state.can_retry());
        state.reset();
        assert!(state.can_retry());
    }
}
