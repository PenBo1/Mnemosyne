// ExecPolicy DSL 模块 —— 精细化沙箱策略系统。
//
// 将粗暴的 allow/deny 列表升级为支持
// 前缀匹配 + 网络协议规则的精细化策略。
//
// 模块布局：
// - types.rs: 类型定义（PolicyDecision / RuleKind / PrefixRule / NetworkRule / NetworkProtocol / ExecPolicy）
// - parser.rs: DSL 解析器（# 注释 + allow/deny/ask + command/path/network 规则）
// - evaluator.rs: 规则求值器（按 priority 降序遍历，无匹配返回 default_decision）
// - builtin.rs: 内置默认策略（白名单 + 黑名单 + 路径保护 + 云元数据拒绝）
//
// 持久化路径：<data_dir>/exec_policy.conf（DSL 文本格式，人类可读可编辑）

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
