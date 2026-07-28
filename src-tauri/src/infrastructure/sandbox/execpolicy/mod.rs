//! ═══════════════════════════════════════════════════════════════════════════
//! 执行策略模块 - 精细化沙箱策略系统
//! ═══════════════════════════════════════════════════════════════════════════

pub mod builtin;
pub mod evaluator;
pub mod parser;
pub mod types;

pub use builtin::default_policy;
pub use evaluator::{extract_host_and_protocol, Evaluation};
pub use parser::{parse, serialize};
pub use types::{
    ExecPolicy, NetworkProtocol, NetworkRule, PolicyDecision, PrefixRule, RuleKind,
};
