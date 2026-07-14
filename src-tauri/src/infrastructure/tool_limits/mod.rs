// Tool Limits —— 每对话工具执行上限配置。
//
// 与 EffortLevel 的关系:
// - EffortLevel.max_tool_steps 是"每次 send_message 调用中工具最多执行多少轮"
//   (low=5/medium=20/high=50/ultra=100)
// - ToolLimits 是更细粒度的覆盖,允许用户在不改 Effort 的情况下单独调整工具上限
//
// 当前实现:
// - 配置持久化到 <data_dir>/tool_limits.json
// - 提供 get/update IPC 命令
// - 配置项:
//   * max_tool_calls_per_dialog: 单次对话(send_message)中工具调用总次数上限
//   * max_consecutive_failures: 连续失败次数上限(达到则停止)
//   * timeout_per_call_ms: 单次工具调用超时(ms,0=不限制)
//
// 注:当前未在 engine 中强制执行(未来扩展点),仅作为用户可配置的"安全阀"展示。

pub mod config;
pub mod commands;
pub mod state;

pub use config::ToolLimitsConfig;
pub use state::ToolLimitsState;
