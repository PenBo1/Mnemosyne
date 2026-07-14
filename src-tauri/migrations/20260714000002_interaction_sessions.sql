-- Interaction Runtime：交互会话持久化表
--
-- 与 sessions 表（chat 会话）不同：本表存储 InteractionSession 完整状态
-- （含 messages/events/pendingDecision/automationMode 等），以 JSON blob 形式持久化。
-- 交互运行时是面向 pipeline + 编辑事务 + 自动化模式的更上层抽象，
-- 介于 session（轻量对话）与 pipeline（重型创作流程）之间。
--
-- 设计要点：
-- - session_kind: 10 种会话类型（chat/book/book_create/short/script/storyboard/...）
-- - active_book_id: 当前绑定的书籍 ID（可空，未绑定书籍时为 NULL）
-- - automation_mode: 自动化模式（auto/semi/manual），决定是否等待用户决策
-- - payload_json: InteractionSession 完整 JSON（messages + events + pendingDecision 等）
-- - updated_at 触发自动按时间倒序排列

CREATE TABLE interaction_sessions (
    id TEXT PRIMARY KEY,
    session_kind TEXT NOT NULL CHECK(length(session_kind) > 0 AND length(session_kind) <= 64),
    automation_mode TEXT NOT NULL DEFAULT 'semi'
        CHECK(automation_mode IN ('auto', 'semi', 'manual')),
    active_book_id TEXT,
    title TEXT NOT NULL DEFAULT '',
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(length(id) > 0 AND length(id) <= 200)
);

CREATE INDEX idx_interaction_sessions_active_book ON interaction_sessions(active_book_id);
CREATE INDEX idx_interaction_sessions_updated_at ON interaction_sessions(updated_at DESC);
CREATE INDEX idx_interaction_sessions_session_kind ON interaction_sessions(session_kind);
