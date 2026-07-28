//! ═══════════════════════════════════════════════════════════════════════════
//! types - Hook 类型定义模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};

// ── Hook 事件类型 ────────────────────────────────────────────────────────────────

/// Hook 事件类型（12 个事件）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookEvent {
    /// 工具调用前（kernel.execute 入口）。
    PreToolUse,
    /// 工具调用后（executor 完成）。
    PostToolUse,
    /// 权限审批请求时（PolicyDecision::RequireApproval）。
    PermissionRequest,
    /// 权限被拒绝时。
    PermissionDenied,
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
    /// 通知事件。
    Notification,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PermissionRequest => "permission_request",
            Self::PermissionDenied => "permission_denied",
            Self::PreCompact => "pre_compact",
            Self::PostCompact => "post_compact",
            Self::SessionStart => "session_start",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::SubagentStart => "subagent_start",
            Self::SubagentStop => "subagent_stop",
            Self::Stop => "stop",
            Self::Notification => "notification",
        }
    }
}

// ── Hook 结果 ────────────────────────────────────────────────────────────────

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

// ── Hook Payload ────────────────────────────────────────────────────────────────

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

// ── Hook 匹配模式 ────────────────────────────────────────────────────────────────

/// Hook 匹配模式类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatcherPattern {
    /// 精确匹配（字符串相等）。
    Exact(String),
    /// Glob 模式匹配（如 `fs_*`、`git_*`）。
    Glob(String),
    /// 正则表达式匹配。
    Regex(String),
}

impl MatcherPattern {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Exact(pattern) => value == pattern,
            Self::Glob(pattern) => {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(value))
                    .unwrap_or_else(|_| value == pattern)
            }
            Self::Regex(pattern) => {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(value))
                    .unwrap_or_else(|_| value == pattern)
            }
        }
    }
}

// ── Hook Matcher ────────────────────────────────────────────────────────────────

/// Hook 匹配器 —— 决定 hook 是否对某个 payload 生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookMatcher {
    /// 工具名匹配模式（精确/glob/正则）。
    pub tool_name_pattern: Option<MatcherPattern>,
    /// workspace_id 精确匹配；为 None 表示对所有 workspace 生效。
    pub workspace_id: Option<String>,
}

impl HookMatcher {
    /// 创建精确匹配器。
    pub fn exact(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name_pattern: Some(MatcherPattern::Exact(tool_name.into())),
            workspace_id: None,
        }
    }

    /// 创建 glob 匹配器。
    pub fn glob(pattern: impl Into<String>) -> Self {
        Self {
            tool_name_pattern: Some(MatcherPattern::Glob(pattern.into())),
            workspace_id: None,
        }
    }

    /// 创建正则匹配器。
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self {
            tool_name_pattern: Some(MatcherPattern::Regex(pattern.into())),
            workspace_id: None,
        }
    }

    /// 添加 workspace 约束。
    pub fn with_workspace(mut self, ws: impl Into<String>) -> Self {
        self.workspace_id = Some(ws.into());
        self
    }

    /// 判断 matcher 是否匹配 payload。
    pub fn matches(&self, payload: &HookPayload) -> bool {
        if let Some(pattern) = &self.tool_name_pattern {
            let name = match &payload.tool_name {
                Some(n) => n,
                None => return false,
            };
            if !pattern.matches(name) {
                return false;
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

// ── Hook Action ────────────────────────────────────────────────────────────────

/// 配置型 hook 的动作 —— IPC 无法注入函数，只能选择内置 action。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookAction {
    /// 记录 tracing 日志（info 级别）。
    Log,
    /// 发送 SecurityEvent 到 AuditEventBus。
    Audit,
    /// 直接返回 FailedAbort —— 用于配置"拦截"规则。
    Block,
    /// 自定义描述（仅用于配置展示，无实际逻辑）。
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

// ── Hook 函数类型 ────────────────────────────────────────────────────────────────

/// Hook 处理函数类型 —— 异步、线程安全、可克隆（Arc）。
pub type HookFn = Arc<dyn Fn(&HookPayload) -> BoxFuture<'static, HookResult> + Send + Sync>;

// ── ConfiguredHook ────────────────────────────────────────────────────────────────

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

// ── IPC DTO ────────────────────────────────────────────────────────────────

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

// ── HookHandler（扩展：Command / Http） ────────────────────────────────

/// Hook 处理器 —— 定义 hook 触发时的执行方式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HookHandler {
    /// 执行 shell 命令。
    Command {
        /// 命令字符串（支持环境变量替换）。
        cmd: String,
    },
    /// 发送 HTTP 请求。
    Http {
        /// 目标 URL（仅允许 HTTPS，禁止私有 IP）。
        url: String,
        /// HTTP 方法（默认 POST）。
        #[serde(default = "default_http_method")]
        method: String,
        /// 请求头（可选）。
        #[serde(default)]
        headers: HashMap<String, String>,
        /// 请求超时（毫秒，默认 30000）。
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    },
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    30000
}

impl HookHandler {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command { .. } => "command",
            Self::Http { .. } => "http",
        }
    }
}

// ── HookSpec ────────────────────────────────────────────────────────────────

/// Hook 规格 —— 包含完整的 hook 配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpec {
    /// Hook 名称（用于日志和调试）。
    pub name: String,
    /// 触发事件。
    pub event: HookEvent,
    /// 处理器（Command / Http）。
    pub handler: HookHandler,
    /// 匹配器（可选，用于过滤 tool_name）。
    pub matcher: Option<HookMatcher>,
    /// 执行超时（毫秒，默认 30000）。
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 是否启用。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl HookSpec {
    pub fn new(name: impl Into<String>, event: HookEvent, handler: HookHandler) -> Self {
        Self {
            name: name.into(),
            event,
            handler,
            matcher: None,
            timeout: default_timeout(),
            enabled: true,
        }
    }

    pub fn with_matcher(mut self, matcher: HookMatcher) -> Self {
        self.matcher = Some(matcher);
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout = timeout_ms;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}