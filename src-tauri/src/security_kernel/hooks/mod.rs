//! ═══════════════════════════════════════════════════════════════════════════
//! hooks - 钩子系统模块
//! ═══════════════════════════════════════════════════════════════════════════

pub mod commands;
pub mod discovery;
pub mod dispatcher;
pub mod engine;
pub mod registry;
pub mod runner;
pub mod types;

pub use commands::{hook_list, hook_register, hook_test_dispatch, hook_unregister, HookEngineState};
pub use discovery::{
    HookDiscovery, HookLoader, DEFAULT_HOOK_CONFIG_FILE, DEFAULT_HOOK_DIR,
    default_hook_config_path, default_hook_dir_path,
};
pub use dispatcher::{
    try_dispatch, HookDispatcher, OptionalHookDispatcher,
    post_compact_payload, pre_compact_payload, session_start_payload,
    stop_payload, subagent_start_payload, subagent_stop_payload, user_prompt_submit_payload,
};
pub use engine::HookEngine;
pub use registry::{handler_for_action, HookDispatchOutcome, HookRegistry};
pub use runner::{CommandRunner, HookRunner, HttpRunner, validate_url_for_ssrf};
pub use types::{
    ConfiguredHook, HookAction, HookConfig, HookEvent, HookFn, HookHandler, HookInfo,
    HookMatcher, HookPayload, HookResult, HookSpec, HookTestRequest, HookTestResult,
    MatcherPattern,
};