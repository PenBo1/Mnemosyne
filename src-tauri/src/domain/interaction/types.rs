//! ═══════════════════════════════════════════════════════════════════════════
//! 交互类型 - 交互运行时核心类型定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ── 会话类型 ────────────────────────────────────────────────────────────────

/// 交互会话类型。覆盖创作流水线的所有入口场景。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    /// 普通对话（无书籍绑定）
    Chat,
    /// 长篇书籍创作
    Book,
    /// 书籍创建向导（收集创作草案）
    BookCreate,
    /// 短篇 pipeline
    Short,
    /// 剧本创作
    Script,
    /// 分镜创作
    Storyboard,
    /// 互动影游创作
    InteractiveFilm,
    /// 互动影游作者模式
    InteractiveFilmAuthoring,
    /// 互动影游游玩模式
    Play,
    /// 编辑事务（truth/chapter/rename）
    Edit,
}

impl SessionKind {
    /// 序列化为字符串（用于 SQLite session_kind 列）
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionKind::Chat => "chat",
            SessionKind::Book => "book",
            SessionKind::BookCreate => "book-create",
            SessionKind::Short => "short",
            SessionKind::Script => "script",
            SessionKind::Storyboard => "storyboard",
            SessionKind::InteractiveFilm => "interactive-film",
            SessionKind::InteractiveFilmAuthoring => "interactive-film-authoring",
            SessionKind::Play => "play",
            SessionKind::Edit => "edit",
        }
    }

    /// 从字符串解析（容错：未知值回退为 Chat）
    pub fn from_str(s: &str) -> Self {
        match s {
            "chat" => SessionKind::Chat,
            "book" => SessionKind::Book,
            "book-create" => SessionKind::BookCreate,
            "short" => SessionKind::Short,
            "script" => SessionKind::Script,
            "storyboard" => SessionKind::Storyboard,
            "interactive-film" => SessionKind::InteractiveFilm,
            "interactive-film-authoring" => SessionKind::InteractiveFilmAuthoring,
            "play" => SessionKind::Play,
            "edit" => SessionKind::Edit,
            _ => SessionKind::Chat,
        }
    }
}

// ── 自动化模式 ───────────────────────────────────────────────

/// 自动化模式。决定执行后是否等待用户决策。
/// - Auto: 完全自动，不等待用户
/// - Semi: 半自动，内容生成类操作（写下一章/修订/修补/替换）等待用户
/// - Manual: 手动，内容生成 + 编辑类操作都等待用户
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AutomationMode {
    Auto,
    #[default]
    Semi,
    Manual,
}

impl AutomationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AutomationMode::Auto => "auto",
            AutomationMode::Semi => "semi",
            AutomationMode::Manual => "manual",
        }
    }

    /// 从字符串解析（容错：未知值回退为 Semi）
    pub fn from_str(s: &str) -> Self {
        match s {
            "auto" => AutomationMode::Auto,
            "manual" => AutomationMode::Manual,
            _ => AutomationMode::Semi,
        }
    }
}


// ── 交互意图（23 种） ────────────────────────────────────────

/// 交互意图。每个意图对应 runtime 中的一条分发路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionIntent {
    DevelopBook,
    ShowBookDraft,
    CreateBook,
    DiscardBookDraft,
    ListBooks,
    SelectBook,
    ContinueBook,
    WriteNext,
    PauseBook,
    ResumeBook,
    ReviseChapter,
    RewriteChapter,
    PatchChapterText,
    ReplaceChapterText,
    EditTruth,
    RenameEntity,
    UpdateFocus,
    UpdateAuthorIntent,
    Chat,
    ExplainStatus,
    ExplainFailure,
    ExportBook,
}

impl InteractionIntent {
    /// 序列化为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            InteractionIntent::DevelopBook => "develop_book",
            InteractionIntent::ShowBookDraft => "show_book_draft",
            InteractionIntent::CreateBook => "create_book",
            InteractionIntent::DiscardBookDraft => "discard_book_draft",
            InteractionIntent::ListBooks => "list_books",
            InteractionIntent::SelectBook => "select_book",
            InteractionIntent::ContinueBook => "continue_book",
            InteractionIntent::WriteNext => "write_next",
            InteractionIntent::PauseBook => "pause_book",
            InteractionIntent::ResumeBook => "resume_book",
            InteractionIntent::ReviseChapter => "revise_chapter",
            InteractionIntent::RewriteChapter => "rewrite_chapter",
            InteractionIntent::PatchChapterText => "patch_chapter_text",
            InteractionIntent::ReplaceChapterText => "replace_chapter_text",
            InteractionIntent::EditTruth => "edit_truth",
            InteractionIntent::RenameEntity => "rename_entity",
            InteractionIntent::UpdateFocus => "update_focus",
            InteractionIntent::UpdateAuthorIntent => "update_author_intent",
            InteractionIntent::Chat => "chat",
            InteractionIntent::ExplainStatus => "explain_status",
            InteractionIntent::ExplainFailure => "explain_failure",
            InteractionIntent::ExportBook => "export_book",
        }
    }

    /// 从字符串解析（容错：未知值返回 None）
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "develop_book" => InteractionIntent::DevelopBook,
            "show_book_draft" => InteractionIntent::ShowBookDraft,
            "create_book" => InteractionIntent::CreateBook,
            "discard_book_draft" => InteractionIntent::DiscardBookDraft,
            "list_books" => InteractionIntent::ListBooks,
            "select_book" => InteractionIntent::SelectBook,
            "continue_book" => InteractionIntent::ContinueBook,
            "write_next" => InteractionIntent::WriteNext,
            "pause_book" => InteractionIntent::PauseBook,
            "resume_book" => InteractionIntent::ResumeBook,
            "revise_chapter" => InteractionIntent::ReviseChapter,
            "rewrite_chapter" => InteractionIntent::RewriteChapter,
            "patch_chapter_text" => InteractionIntent::PatchChapterText,
            "replace_chapter_text" => InteractionIntent::ReplaceChapterText,
            "edit_truth" => InteractionIntent::EditTruth,
            "rename_entity" => InteractionIntent::RenameEntity,
            "update_focus" => InteractionIntent::UpdateFocus,
            "update_author_intent" => InteractionIntent::UpdateAuthorIntent,
            "chat" => InteractionIntent::Chat,
            "explain_status" => InteractionIntent::ExplainStatus,
            "explain_failure" => InteractionIntent::ExplainFailure,
            "export_book" => InteractionIntent::ExportBook,
            _ => return None,
        })
    }
}

// ── 执行状态（11 种） ────────────────────────────────────────

/// 执行状态。描述当前任务的生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Idle,
    Planning,
    Composing,
    Writing,
    Assessing,
    Repairing,
    Persisting,
    WaitingHuman,
    Blocked,
    Completed,
    Failed,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Idle => "idle",
            ExecutionStatus::Planning => "planning",
            ExecutionStatus::Composing => "composing",
            ExecutionStatus::Writing => "writing",
            ExecutionStatus::Assessing => "assessing",
            ExecutionStatus::Repairing => "repairing",
            ExecutionStatus::Persisting => "persisting",
            ExecutionStatus::WaitingHuman => "waiting_human",
            ExecutionStatus::Blocked => "blocked",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
        }
    }
}

/// 检查是否终态（completed/failed/blocked）。
pub fn is_terminal_execution_status(status: &ExecutionStatus) -> bool {
    matches!(
        status,
        ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Blocked
    )
}

// ── 执行状态（含 stageLabel） ────────────────────────────────

/// 当前执行状态快照。运行时在每次状态切换时更新。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    /// 可读的阶段标签（中英文皆可），用于前端展示
    pub stage_label: String,
    /// 进度（0.0 ~ 1.0，可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
}

impl ExecutionState {
    pub fn new(status: ExecutionStatus, stage_label: impl Into<String>) -> Self {
        Self {
            status,
            stage_label: stage_label.into(),
            progress: None,
        }
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }
}

// ── 交互事件 ─────────────────────────────────────────────────

/// 交互事件。记录运行时每个关键节点的状态变迁。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionEvent {
    /// 事件类型（task.started / task.completed / task.failed / ...）
    pub event_type: String,
    /// ISO 时间戳
    pub timestamp: String,
    /// 事件载荷（结构因事件类型而异）
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl InteractionEvent {
    pub fn new(event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
        }
    }
}

// ── 交互消息 ─────────────────────────────────────────────────

/// 交互消息。运行时维护的对话历史（独立于 sessions 表）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionMessage {
    /// 角色（user / assistant / system）
    pub role: String,
    pub content: String,
    /// ISO 时间戳
    pub timestamp: String,
    /// 元数据（toolExecutions / thinking / ...）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ── 待决策 ───────────────────────────────────────────────────

/// 待决策。当 automation_mode 决定等待用户时填充。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingDecision {
    /// 决策类型（review-next-step / confirm-edit / ...）
    pub decision_type: String,
    /// 可读描述
    pub description: String,
    /// 可选选项列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

// ── 交互会话 ─────────────────────────────────────────────────

/// 交互会话。运行时的核心状态容器。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionSession {
    pub session_id: String,
    pub session_kind: SessionKind,
    #[serde(default)]
    pub automation_mode: AutomationMode,
    /// 当前绑定的书籍 ID（可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_book_id: Option<String>,
    /// 对话消息历史
    #[serde(default)]
    pub messages: Vec<InteractionMessage>,
    /// 事件流
    #[serde(default)]
    pub events: Vec<InteractionEvent>,
    /// 待决策（可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_decision: Option<PendingDecision>,
    /// 当前执行状态（可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_execution: Option<ExecutionState>,
    /// 创建时间（ISO）
    pub created_at: String,
    /// 更新时间（ISO）
    pub updated_at: String,
}

impl InteractionSession {
    /// 创建新会话
    pub fn new(session_id: impl Into<String>, session_kind: SessionKind) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            session_id: session_id.into(),
            session_kind,
            automation_mode: AutomationMode::default(),
            active_book_id: None,
            messages: Vec::new(),
            events: Vec::new(),
            pending_decision: None,
            current_execution: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 追加事件并刷新 updated_at
    pub fn append_event(&mut self, event: InteractionEvent) {
        self.events.push(event);
        self.touch();
    }

    /// 追加消息并刷新 updated_at
    pub fn append_message(&mut self, message: InteractionMessage) {
        self.messages.push(message);
        self.touch();
    }

    /// 绑定激活书籍
    pub fn bind_active_book(&mut self, book_id: impl Into<String>) {
        self.active_book_id = Some(book_id.into());
        self.touch();
    }

    /// 清空待决策
    pub fn clear_pending_decision(&mut self) {
        if self.pending_decision.is_some() {
            self.pending_decision = None;
            self.touch();
        }
    }

    /// 更新自动化模式
    pub fn update_automation_mode(&mut self, mode: AutomationMode) {
        self.automation_mode = mode;
        self.touch();
    }

    /// 刷新 updated_at
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

// ── 真相权威（5 级） ─────────────────────────────────────────

/// 真相文件权威级别。从高到低：Direction > Foundation > Rules > RuntimeTruth > Memory。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TruthAuthority {
    /// 方向（author_intent / current_focus）
    Direction,
    /// 基础（story_bible / volume_outline）
    Foundation,
    /// 规则（book_rules）
    Rules,
    /// 运行时真相（current_state / pending_hooks）
    RuntimeTruth,
    /// 记忆（chapter_summaries）
    Memory,
}

impl TruthAuthority {
    pub fn as_str(&self) -> &'static str {
        match self {
            TruthAuthority::Direction => "direction",
            TruthAuthority::Foundation => "foundation",
            TruthAuthority::Rules => "rules",
            TruthAuthority::RuntimeTruth => "runtime-truth",
            TruthAuthority::Memory => "memory",
        }
    }
}

// ── 动作来源 ─────────────────────────────────────────────────

/// 动作来源。描述用户触发交互的入口。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionSource {
    /// 按钮点击
    Button,
    /// 斜杠命令
    Slash,
    /// 聊天输入
    Chat,
}

// ── 动作封装 ─────────────────────────────────────────────────

/// 动作封装。统一描述前端发起的任何交互动作。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionEnvelope {
    pub intent: InteractionIntent,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub source: ActionSource,
}

// ── 交互请求 ─────────────────────────────────────────────────

/// 交互请求。前端发起 run_interaction_request 的入参。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionRequest {
    pub intent: InteractionIntent,
    /// 用户输入文本（chat 内容 / edit instruction / ...）
    #[serde(default)]
    pub input: String,
    /// 目标书籍 ID（可空，未指定时使用 session.active_book_id）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_id: Option<String>,
    /// 章节号（chapter 相关意图必填）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<u32>,
    /// 目标 session ID（可空，未指定时创建新会话）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// 覆盖 automation_mode（可空）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<AutomationMode>,
    /// 额外参数（fileName / oldValue / newValue / targetText / replacementText / fullText / format / ...）
    #[serde(default)]
    pub params: serde_json::Value,
}

// ── 交互运行时结果 ───────────────────────────────────────────

/// 交互运行时结果。run_interaction_request 的返回值。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionRuntimeResult {
    /// 最终会话状态（已应用所有变迁）
    pub session: InteractionSession,
    /// 可读的响应文本（前端展示）
    pub response_text: String,
    /// 本次运行产生的新事件（已并入 session.events）
    #[serde(default)]
    pub events: Vec<InteractionEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_kind_roundtrip() {
        for kind in [
            SessionKind::Chat,
            SessionKind::Book,
            SessionKind::BookCreate,
            SessionKind::Short,
            SessionKind::Script,
            SessionKind::Storyboard,
            SessionKind::InteractiveFilm,
            SessionKind::InteractiveFilmAuthoring,
            SessionKind::Play,
            SessionKind::Edit,
        ] {
            assert_eq!(SessionKind::from_str(kind.as_str()), kind);
        }
    }

    #[test]
    fn automation_mode_default_is_semi() {
        assert_eq!(AutomationMode::default(), AutomationMode::Semi);
    }

    #[test]
    fn intent_roundtrip() {
        for s in [
            "develop_book",
            "show_book_draft",
            "create_book",
            "discard_book_draft",
            "list_books",
            "select_book",
            "continue_book",
            "write_next",
            "pause_book",
            "resume_book",
            "revise_chapter",
            "rewrite_chapter",
            "patch_chapter_text",
            "replace_chapter_text",
            "edit_truth",
            "rename_entity",
            "update_focus",
            "update_author_intent",
            "chat",
            "explain_status",
            "explain_failure",
            "export_book",
        ] {
            let intent = InteractionIntent::from_str(s).expect(s);
            assert_eq!(intent.as_str(), s);
        }
    }

    #[test]
    fn terminal_status_check() {
        assert!(is_terminal_execution_status(&ExecutionStatus::Completed));
        assert!(is_terminal_execution_status(&ExecutionStatus::Failed));
        assert!(is_terminal_execution_status(&ExecutionStatus::Blocked));
        assert!(!is_terminal_execution_status(&ExecutionStatus::Idle));
        assert!(!is_terminal_execution_status(&ExecutionStatus::Writing));
    }

    #[test]
    fn session_new_initializes_fields() {
        let session = InteractionSession::new("test-1", SessionKind::Book);
        assert_eq!(session.session_id, "test-1");
        assert_eq!(session.session_kind, SessionKind::Book);
        assert_eq!(session.automation_mode, AutomationMode::Semi);
        assert!(session.messages.is_empty());
        assert!(session.events.is_empty());
        assert!(session.pending_decision.is_none());
        assert!(session.current_execution.is_none());
    }

    #[test]
    fn session_append_event_touches_timestamp() {
        let mut session = InteractionSession::new("test-2", SessionKind::Chat);
        let original = session.updated_at.clone();
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.append_event(InteractionEvent::new("task.started", serde_json::Value::Null));
        assert_eq!(session.events.len(), 1);
        assert_ne!(session.updated_at, original);
    }
}
