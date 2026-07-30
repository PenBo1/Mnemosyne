//! ═══════════════════════════════════════════════════════════════════════════
//! 沙箱模块 - 执行安全策略
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! ## 分层架构
//!
//! - `commands.rs`: IPC 命令入口，仅参数验证和委托
//! - `validator.rs`: 验证业务逻辑，路径/命令/URL 验证
//! - `constants.rs`: 常量定义
//! - `state.rs`: 状态管理
//! - `policy.rs`: 沙箱策略配置
//! - `heuristics.rs`: 危险模式检测
//! - `execpolicy/`: 执行策略子系统

pub mod commands;
pub mod constants;
pub mod validator;
pub mod execpolicy;
pub mod policy;
pub mod types;
pub mod state;
pub mod heuristics;

// 公开类型导出
pub use types::SandboxStatus;
pub use policy::SandboxPolicy;
pub use constants::{MAX_PATH_LEN, MAX_COMMAND_LEN, MAX_URL_LEN};