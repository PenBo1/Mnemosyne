//! ═══════════════════════════════════════════════════════════════════════════
//! 断路器状态 - 状态定义与原子操作
//! ═══════════════════════════════════════════════════════════════════════════

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};

// ── 断路器状态枚举 ───────────────────────────────────────────────────────────

/// 断路器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CircuitState {
    /// 关闭状态，正常处理请求
    #[default]
    Closed = 0,
    /// 打开状态，拒绝所有请求
    Open = 1,
    /// 半开状态，允许有限请求探测
    HalfOpen = 2,
}


impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

impl CircuitState {
    /// 检查是否允许请求
    pub fn is_request_allowed(&self) -> bool {
        matches!(self, CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// 检查是否处于打开状态
    pub fn is_open(&self) -> bool {
        matches!(self, CircuitState::Open)
    }
}

// ── 原子断路器状态 ───────────────────────────────────────────────────────────

/// 原子断路器状态，支持并发访问
#[derive(Debug)]
pub struct AtomicCircuitState {
    inner: AtomicU8,
}

impl Default for AtomicCircuitState {
    fn default() -> Self {
        Self::new(CircuitState::Closed)
    }
}

impl AtomicCircuitState {
    /// 创建新的原子状态实例
    pub fn new(state: CircuitState) -> Self {
        Self {
            inner: AtomicU8::new(state as u8),
        }
    }

    /// 加载当前状态
    pub fn load(&self, order: Ordering) -> CircuitState {
        match self.inner.load(order) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    /// 存储新状态
    pub fn store(&self, state: CircuitState, order: Ordering) {
        self.inner.store(state as u8, order);
    }

    /// 比较并交换状态
    pub fn compare_exchange(
        &self,
        current: CircuitState,
        new: CircuitState,
        success: Ordering,
        failure: Ordering,
    ) -> Result<CircuitState, CircuitState> {
        self.inner
            .compare_exchange(current as u8, new as u8, success, failure)
            .map(|v| match v {
                0 => CircuitState::Closed,
                1 => CircuitState::Open,
                2 => CircuitState::HalfOpen,
                _ => CircuitState::Closed,
            })
            .map_err(|v| match v {
                0 => CircuitState::Closed,
                1 => CircuitState::Open,
                2 => CircuitState::HalfOpen,
                _ => CircuitState::Closed,
            })
    }

    /// 检查是否处于打开状态
    pub fn is_open(&self, order: Ordering) -> bool {
        self.load(order).is_open()
    }

    /// 检查是否允许请求
    pub fn is_request_allowed(&self, order: Ordering) -> bool {
        self.load(order).is_request_allowed()
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let atomic_state = AtomicCircuitState::new(CircuitState::Closed);
        
        assert!(!atomic_state.is_open(Ordering::SeqCst));
        assert!(atomic_state.is_request_allowed(Ordering::SeqCst));
        
        atomic_state.store(CircuitState::Open, Ordering::SeqCst);
        assert!(atomic_state.is_open(Ordering::SeqCst));
        assert!(!atomic_state.is_request_allowed(Ordering::SeqCst));
        
        atomic_state.store(CircuitState::HalfOpen, Ordering::SeqCst);
        assert!(!atomic_state.is_open(Ordering::SeqCst));
        assert!(atomic_state.is_request_allowed(Ordering::SeqCst));
    }

    #[test]
    fn test_compare_exchange() {
        let atomic_state = AtomicCircuitState::new(CircuitState::Closed);
        
        let result = atomic_state.compare_exchange(
            CircuitState::Closed,
            CircuitState::Open,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert!(result.is_ok());
        assert_eq!(atomic_state.load(Ordering::SeqCst), CircuitState::Open);
        
        let result = atomic_state.compare_exchange(
            CircuitState::Closed,
            CircuitState::HalfOpen,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        assert!(result.is_err());
        assert_eq!(atomic_state.load(Ordering::SeqCst), CircuitState::Open);
    }
}