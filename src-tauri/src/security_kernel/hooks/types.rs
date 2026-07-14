// Hook 系统类型定义。
//
// 设计要点：
// - `HookEvent` 枚举覆盖完整生命周期（Pre/PostToolUse、Permission、Compact、Session、
//   Subagent、UserPromptSubmit、Stop）。
// - `HookFn` 为 `Arc<dyn Fn(&HookPayload) -> BoxFuture<'static, HookResult> + Send + Sync>`，
//   支持 sync 与 async handler（async handler 直接 `Box::pin(async move {...})`）。
// - `HookAction` 用于配置型 hook：IPC 无法注入函数，只能选择内置 action（Log / Audit / Block），
//   引擎按 action 派发到内置 handler。
// - `HookMatcher` 用 glob 模式匹配 tool_name（如 `fs_*` 匹配所有 fs 工具）。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};

// ── 事件 ──

/// Hook 事件类型（10 个事件）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookEvent {
    /// 工具调用前（kernel.execute 入口）。
    PreToolUse,
    /// 工具调用后（executor 完成）。
    PostToolUse,
    /// 权限审批请求时（PolicyDecision::RequireApproval）。
    PermissionRequest,
    /// 上下文压缩前。
    PreCompact,
    /// 上下文压缩后。
    PostCompact,
    /// 会话开始时。
    SessionStart,
    /// 用户提交 prompt 时。
    UserPromptSubmit,
    /// 子 agent 启动时。
    SubagentStart,
    /// 子 agent 停止时。
    SubagentStop,
    /// 主 agent 停止时。
    Stop,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PermissionRequest => "permission_request",
            Self::PreCompact => "pre_compact",
            Self::PostCompact => "post_compact",
            Self::SessionStart => "session_start",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::SubagentStart => "subagent_start",
            Self::SubagentStop => "subagent_stop",
            Self::Stop => "stop",
        }
    }
}

// ── 结果 ──

/// Hook handler 返回值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookResult {
    /// 成功，继续后续 hook。
    Success,
    /// 失败但继续（仅记录）。
    FailedContinue,
    /// 失败并中止整个操作链 —— 立即返回错误。
    FailedAbort,
}

// ── Payload ──

/// Hook 派发的上下文负载。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPayload {
    pub event: HookEvent,
    /// 触发 hook 的工具/操作名（如 "fs_write_file"、"git_commit"）。
    pub tool_name: Option<String>,
    /// 工具入参（JSON 序列化）。
    pub tool_args: Option<serde_json::Value>,
    pub session_id: Option<String>,
    pub workspace_id: Option<String>,
    pub agent_role: Option<String>,
    pub timestamp: DateTime<Utc>,
    /// 任意附加元数据（key → JSON value）。
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookPayload {
    pub fn new(event: HookEvent) -> Self {
        Self {
            event,
            tool_name: None,
            tool_args: None,
            session_id: None,
            workspace_id: None,
            agent_role: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = Some(name.into());
        self
    }

    pub fn with_workspace(mut self, ws: impl Into<String>) -> Self {
        self.workspace_id = Some(ws.into());
        self
    }

    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session_id = Some(session.into());
        self
    }

    pub fn with_agent_role(mut self, role: impl Into<String>) -> Self {
        self.agent_role = Some(role.into());
        self
    }

    pub fn with_tool_args(mut self, args: serde_json::Value) -> Self {
        self.tool_args = Some(args);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

// ── Matcher ──

/// Hook 匹配器 —— 决定 hook 是否对某个 payload 生效。
///
/// `tool_name_pattern` 使用 glob 语法（如 `fs_*`、`git_*`、`edit`）。
/// `workspace_id` 精确匹配；为 None 表示对所有 workspace 生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookMatcher {
    pub tool_name_pattern: Option<String>,
    pub workspace_id: Option<String>,
}

impl HookMatcher {
    /// 判断 matcher 是否匹配 payload。
    ///
    /// - 任何字段为 None 表示"不约束"。
    /// - tool_name_pattern 用 glob 匹配；payload.tool_name 为 None 时，pattern 必须为 None 才匹配。
    /// - workspace_id 精确字符串匹配。
    pub fn matches(&self, payload: &HookPayload) -> bool {
        if let Some(pattern) = &self.tool_name_pattern {
            let name = match &payload.tool_name {
                Some(n) => n,
                None => return false,
            };
            if let Ok(p) = glob::Pattern::new(pattern) {
                if !p.matches(name) {
                    return false;
                }
            } else {
                // 模式非法时退化为字面匹配；不静默放过（避免误触发）
                if pattern != name {
                    return false;
                }
            }
        }

        if let Some(ws) = &self.workspace_id {
            if payload.workspace_id.as_deref() != Some(ws.as_str()) {
                return false;
            }
        }

        true
    }
}

// ── Action（配置型 hook 的内置动作） ──

/// 配置型 hook 的动作 —— IPC 无法注入函数，只能选择内置 action。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookAction {
    /// 记录 tracing 日志（info 级别）。
    Log,
    /// 发送 SecurityEvent 到 AuditEventBus（HookDispatched）。
    Audit,
    /// 直接返回 FailedAbort —— 用于配置"拦截"规则。
    Block,
    /// 自定义描述（仅用于配置展示，无实际逻辑 —— 等同 Log）。
    Custom(String),
}

impl HookAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Audit => "audit",
            Self::Block => "block",
            Self::Custom(_) => "custom",
        }
    }
}

// ── HookFn ──

/// Hook 处理函数类型 —— 异步、线程安全、可克隆（Arc）。
///
/// handler 接收 `&HookPayload`，返回 `'static` future —— handler 内部需克隆所需数据。
pub type HookFn = Arc<dyn Fn(&HookPayload) -> BoxFuture<'static, HookResult> + Send + Sync>;

// ── ConfiguredHook（内部完整结构） ──

/// 已注册的 hook —— 内部使用，包含 handler 函数。
#[derive(Clone)]
pub struct ConfiguredHook {
    pub id: String,
    pub event: HookEvent,
    pub matcher: Option<HookMatcher>,
    pub handler: HookFn,
    pub priority: i32,
    /// 配置型 hook 的 action（用于 IPC 展示）；内部注册的 hook 为 None。
    pub action: Option<HookAction>,
}

// ── IPC DTO ──

/// IPC 注册 hook 的输入 —— 不含 handler 函数，仅描述配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookConfig {
    /// 可选 id；未提供时由 registry 生成（uuid v4）。
    pub id: Option<String>,
    pub event: HookEvent,
    pub matcher: Option<HookMatcher>,
    pub action: HookAction,
    /// 优先级，数字越大越先执行；默认 0。
    #[serde(default)]
    pub priority: i32,
}

/// IPC 输出 —— 已注册 hook 的描述信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookInfo {
    pub id: String,
    pub event: HookEvent,
    pub matcher: Option<HookMatcher>,
    pub action: Option<HookAction>,
    pub priority: i32,
}

impl From<&ConfiguredHook> for HookInfo {
    fn from(h: &ConfiguredHook) -> Self {
        Self {
            id: h.id.clone(),
            event: h.event,
            matcher: h.matcher.clone(),
            action: h.action.clone(),
            priority: h.priority,
        }
    }
}

/// Hook 测试派发的输入 —— 由 IPC 调用，模拟一次 hook 触发。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookTestRequest {
    pub event: HookEvent,
    pub tool_name: Option<String>,
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_role: Option<String>,
    pub tool_args: Option<serde_json::Value>,
}

/// Hook 测试派发的输出 —— 每个被触发 hook 的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookTestResult {
    pub dispatched_count: usize,
    /// 任意 hook 返回 FailedAbort 时为 true。
    pub aborted: bool,
    /// 触发到的 hook id 列表（按执行顺序）。
    pub triggered_ids: Vec<String>,
}
