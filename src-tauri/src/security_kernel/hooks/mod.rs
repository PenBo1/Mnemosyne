// Hook 系统。
//
// 模块布局：
// - types.rs: 类型定义（HookEvent、HookPayload、HookResult、HookMatcher、HookAction、ConfiguredHook、IPC DTO）
// - registry.rs: HookRegistry —— 存储 + 派发（按 priority 降序，matcher 过滤，FailedAbort 中止）
// - engine.rs: HookEngine —— 包装 registry + audit_bus，提供带审计的 dispatch
// - dispatcher.rs: HookDispatcher trait —— AgentEngine / SubAgentExecutor 的 hook 派发抽象 + payload 构造辅助
// - commands.rs: IPC 命令（hook_list/hook_register/hook_unregister/hook_test_dispatch）+ HookEngineState
//
// 集成路径：
// - SecurityKernel 持有 `hook_engine: Arc<HookEngine>`，在 execute_internal 的关键节点派发 hook
// - AgentEngine 持有 `Option<Arc<HookEngine>>`，在 send_message 前后派发 SessionStart/UserPromptSubmit/Stop
// - SubAgentExecutor 借用 AgentEngine 的 dispatcher，在 execute 前后派发 SubagentStart/SubagentStop
// - IPC 通过 HookEngineState 暴露 hook 管理 API

pub mod commands;
pub mod dispatcher;
pub mod engine;
pub mod registry;
pub mod types;

pub use commands::{hook_list, hook_register, hook_test_dispatch, hook_unregister, HookEngineState};
pub use dispatcher::{
    try_dispatch, HookDispatcher, OptionalHookDispatcher,
    post_compact_payload, pre_compact_payload, session_start_payload,
    stop_payload, subagent_start_payload, subagent_stop_payload, user_prompt_submit_payload,
};
pub use engine::HookEngine;
pub use registry::{handler_for_action, HookDispatchOutcome, HookRegistry};
pub use types::{
    ConfiguredHook, HookAction, HookConfig, HookEvent, HookFn, HookInfo, HookMatcher, HookPayload,
    HookResult, HookTestRequest, HookTestResult,
};
