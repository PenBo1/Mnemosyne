//! ═══════════════════════════════════════════════════════════════════════════
//! 断路器配置 - 断路器参数配置
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 断路器配置参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// 失败阈值，达到此数值触发打开
    pub failure_threshold: usize,
    /// 成功阈值，半开状态下达到此数值触发关闭
    pub success_threshold: usize,
    /// 超时时间，打开状态持续时间
    pub timeout: Duration,
    /// 滑动窗口大小
    pub window_size: usize,
    /// 错误率阈值
    pub error_rate_threshold: f64,
    /// 最小请求数，低于此数值不触发打开
    pub min_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            window_size: 100,
            error_rate_threshold: 0.5,
            min_requests: 10,
        }
    }
}

impl CircuitBreakerConfig {
    /// 创建新的配置实例
    pub fn new(
        failure_threshold: usize,
        success_threshold: usize,
        timeout: Duration,
        window_size: usize,
    ) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            window_size,
            error_rate_threshold: 0.5,
            min_requests: 0,
        }
    }

    /// 设置错误率阈值
    pub fn with_error_rate_threshold(mut self, threshold: f64) -> Self {
        self.error_rate_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 设置最小请求数
    pub fn with_min_requests(mut self, min_requests: usize) -> Self {
        self.min_requests = min_requests;
        self
    }

    /// 判断是否应该打开断路器
    pub fn should_open(&self, failures: usize, total: usize) -> bool {
        if total < self.min_requests {
            return false;
        }
        if failures >= self.failure_threshold {
            return true;
        }
        let error_rate = failures as f64 / total as f64;
        error_rate >= self.error_rate_threshold
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.window_size, 100);
    }

    #[test]
    fn test_should_open_by_threshold() {
        let config = CircuitBreakerConfig::default();
        
        assert!(config.should_open(5, 10));
        assert!(!config.should_open(4, 10));
    }

    #[test]
    fn test_should_open_by_error_rate() {
        let config = CircuitBreakerConfig::default()
            .with_error_rate_threshold(0.6);
        
        assert!(config.should_open(60, 100));
        assert!(config.should_open(6, 10));
        assert!(!config.should_open(4, 10));
    }

    #[test]
    fn test_should_not_open_below_min_requests() {
        let config = CircuitBreakerConfig::default()
            .with_min_requests(20);
        
        assert!(!config.should_open(5, 10));
        assert!(config.should_open(10, 20));
    }
}