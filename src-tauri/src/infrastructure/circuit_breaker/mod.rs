//! ═══════════════════════════════════════════════════════════════════════════
//! 断路器模块 - 服务熔断保护机制
//! ═══════════════════════════════════════════════════════════════════════════

mod breaker;
mod config;
mod state;
mod window;

pub use breaker::{CircuitBreaker, CircuitBreakerError, CircuitBreakerStats, CircuitResult};
pub use config::CircuitBreakerConfig;
pub use state::{AtomicCircuitState, CircuitState};
pub use window::{RequestOutcome, SlidingWindow, SlidingWindowSnapshot};