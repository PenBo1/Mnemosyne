-- Mnemosyne 初始 schema（state.sqlite）
-- 业务核心 + AI 日志 + 章节版本 + 看板 + 循环工程
-- 清库重建版本：合并原 state/logs 两库

-- ═══════════════════════════════════════════════════════════
-- PRAGMA（WAL 模式下推荐配置）
-- ═══════════════════════════════════════════════════════════
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;       -- WAL 模式推荐
PRAGMA busy_timeout = 5000;        -- 5s 避免并发死锁
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -20000;        -- 20MB 缓存

-- ═══════════════════════════════════════════════════════════
-- Workspaces
-- ═══════════════════════════════════════════════════════════
CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
    path TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- Novels
-- ═══════════════════════════════════════════════════════════
CREATE TABLE novels (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL CHECK(length(title) > 0 AND length(title) <= 500),
    genre TEXT NOT NULL DEFAULT 'general' CHECK(length(genre) <= 100),
    platform TEXT NOT NULL DEFAULT 'local' CHECK(length(platform) <= 100),
    status TEXT NOT NULL DEFAULT 'drafting' CHECK(status IN ('drafting', 'paused', 'completed', 'archived')),
    language TEXT NOT NULL DEFAULT 'zh' CHECK(language IN ('zh', 'en')),
    word_count INTEGER NOT NULL DEFAULT 0 CHECK(word_count >= 0),
    chapter_count INTEGER NOT NULL DEFAULT 0 CHECK(chapter_count >= 0),
    target_chapters INTEGER NOT NULL DEFAULT 100 CHECK(target_chapters > 0),
    chapter_words INTEGER NOT NULL DEFAULT 3000 CHECK(chapter_words > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_novels_workspace ON novels(workspace_id);

-- ═══════════════════════════════════════════════════════════
-- Chapters
-- ═══════════════════════════════════════════════════════════
CREATE TABLE chapters (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    number INTEGER NOT NULL CHECK(number > 0),
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'drafting' CHECK(status IN ('drafting', 'review', 'approved', 'rejected')),
    word_count INTEGER NOT NULL DEFAULT 0 CHECK(word_count >= 0),
    audit_score REAL CHECK(audit_score IS NULL OR (audit_score >= 0.0 AND audit_score <= 10.0)),
    revision_count INTEGER NOT NULL DEFAULT 0 CHECK(revision_count >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, number)
);

CREATE INDEX idx_chapters_novel ON chapters(novel_id, number);
CREATE INDEX idx_chapters_status ON chapters(novel_id, status);

-- ═══════════════════════════════════════════════════════════
-- Sessions
-- ═══════════════════════════════════════════════════════════
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    novel_id TEXT,
    session_type TEXT NOT NULL DEFAULT 'chat' CHECK(session_type IN ('chat', 'pipeline', 'review')),
    title TEXT NOT NULL DEFAULT '',
    summary TEXT,
    message_count INTEGER NOT NULL DEFAULT 0 CHECK(message_count >= 0),
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    cost REAL NOT NULL DEFAULT 0.0 CHECK(cost >= 0.0),
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'paused', 'completed', 'archived')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE SET NULL
);

CREATE INDEX idx_sessions_novel ON sessions(novel_id);
CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Messages（含 thinking/model/provider/tokens/latency 新列）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL DEFAULT '',
    tool_calls TEXT,
    tool_results TEXT,
    token_count INTEGER CHECK(token_count IS NULL OR token_count >= 0),
    -- 新增列：支持 ChatPage 渲染 thinking 动画 + 模型/token 观测
    thinking_content TEXT,
    model TEXT,
    provider TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    latency_ms INTEGER CHECK(latency_ms IS NULL OR latency_ms >= 0),
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);
CREATE INDEX idx_messages_created ON messages(created_at);

-- ═══════════════════════════════════════════════════════════
-- Prompts
-- ═══════════════════════════════════════════════════════════
CREATE TABLE prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general' CHECK(length(category) <= 100),
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_prompts_category ON prompts(category);
CREATE INDEX idx_prompts_updated ON prompts(updated_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Trends
-- ═══════════════════════════════════════════════════════════
CREATE TABLE trends (
    id TEXT PRIMARY KEY,
    keyword TEXT NOT NULL CHECK(length(keyword) > 0 AND length(keyword) <= 255),
    platform TEXT NOT NULL CHECK(length(platform) > 0 AND length(platform) <= 100),
    score REAL NOT NULL DEFAULT 0.0,
    metadata TEXT NOT NULL DEFAULT '{}',
    scanned_at TEXT NOT NULL
);

CREATE INDEX idx_trends_keyword ON trends(keyword);
CREATE INDEX idx_trends_platform ON trends(platform);
CREATE INDEX idx_trends_scanned ON trends(scanned_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Radar Scans
-- ═══════════════════════════════════════════════════════════
CREATE TABLE radar_scans (
    id TEXT PRIMARY KEY,
    market_summary TEXT NOT NULL,
    recommendations_json TEXT NOT NULL DEFAULT '[]',
    raw_rankings_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);

CREATE INDEX idx_radar_scans_created ON radar_scans(created_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Wiki Entries
-- ═══════════════════════════════════════════════════════════
CREATE TABLE wiki_entries (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK(length(title) > 0 AND length(title) <= 500),
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general' CHECK(category IN ('general', 'character', 'location', 'event', 'concept', 'reference')),
    source_type TEXT NOT NULL DEFAULT 'manual' CHECK(source_type IN ('manual', 'ai_extracted', 'imported')),
    source_chapter INTEGER CHECK(source_chapter IS NULL OR source_chapter > 0),
    tags TEXT NOT NULL DEFAULT '[]',
    importance INTEGER NOT NULL DEFAULT 0 CHECK(importance >= 0 AND importance <= 10),
    word_count INTEGER NOT NULL DEFAULT 0 CHECK(word_count >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE
);

CREATE INDEX idx_wiki_novel ON wiki_entries(novel_id);
CREATE INDEX idx_wiki_category ON wiki_entries(novel_id, category);
CREATE INDEX idx_wiki_source_chapter ON wiki_entries(novel_id, source_chapter);

-- Wiki FTS5（中文用 unicode61，TODO 后续换 trigram）
CREATE VIRTUAL TABLE wiki_entries_fts USING fts5(
    title, content, tags,
    content='wiki_entries', content_rowid='rowid',
    tokenize='unicode61'
);

CREATE TRIGGER wiki_fts_insert AFTER INSERT ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;
CREATE TRIGGER wiki_fts_update AFTER UPDATE ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(wiki_entries_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
    INSERT INTO wiki_entries_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;
CREATE TRIGGER wiki_fts_delete AFTER DELETE ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(wiki_entries_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
END;

-- ═══════════════════════════════════════════════════════════
-- Wiki Entity Links
-- ═══════════════════════════════════════════════════════════
CREATE TABLE wiki_entity_links (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    source_entry_id TEXT NOT NULL,
    target_entry_id TEXT NOT NULL,
    relation_type TEXT NOT NULL CHECK(length(relation_type) > 0 AND length(relation_type) <= 100),
    relation_desc TEXT NOT NULL DEFAULT '',
    weight INTEGER NOT NULL DEFAULT 1 CHECK(weight >= 1 AND weight <= 10),
    source_chapter INTEGER CHECK(source_chapter IS NULL OR source_chapter > 0),
    created_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    FOREIGN KEY (source_entry_id) REFERENCES wiki_entries(id) ON DELETE CASCADE,
    FOREIGN KEY (target_entry_id) REFERENCES wiki_entries(id) ON DELETE CASCADE,
    UNIQUE(source_entry_id, target_entry_id, relation_type)
);

CREATE INDEX idx_wiki_links_novel ON wiki_entity_links(novel_id);
CREATE INDEX idx_wiki_links_source ON wiki_entity_links(source_entry_id);
CREATE INDEX idx_wiki_links_target ON wiki_entity_links(target_entry_id);

-- ═══════════════════════════════════════════════════════════
-- Chapter Versions
-- ═══════════════════════════════════════════════════════════
CREATE TABLE chapter_versions (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL CHECK(chapter_number > 0),
    version_number INTEGER NOT NULL CHECK(version_number > 0),
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    word_count INTEGER NOT NULL DEFAULT 0 CHECK(word_count >= 0),
    revision_reason TEXT NOT NULL DEFAULT '',
    revision_mode TEXT NOT NULL DEFAULT 'auto' CHECK(revision_mode IN ('auto', 'polish', 'rewrite', 'rework', 'spot_fix', 'manual')),
    created_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, chapter_number, version_number)
);

CREATE INDEX idx_versions_novel_chapter ON chapter_versions(novel_id, chapter_number);
CREATE INDEX idx_versions_created ON chapter_versions(novel_id, created_at DESC);
CREATE INDEX idx_versions_hash ON chapter_versions(content_hash);

-- ═══════════════════════════════════════════════════════════
-- Kanban Tasks
-- ═══════════════════════════════════════════════════════════
CREATE TABLE kanban_tasks (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    title TEXT NOT NULL CHECK(length(title) > 0 AND length(title) <= 500),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'plan' CHECK(status IN ('plan', 'compose', 'write', 'audit', 'revise', 'done', 'cancelled')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('low', 'medium', 'high', 'urgent')),
    assigned_agent TEXT,
    chapter_id TEXT,
    parent_task_id TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    sort_order INTEGER NOT NULL DEFAULT 0,
    due_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_task_id) REFERENCES kanban_tasks(id) ON DELETE SET NULL
);

CREATE INDEX idx_kanban_novel ON kanban_tasks(novel_id);
CREATE INDEX idx_kanban_status ON kanban_tasks(novel_id, status);
CREATE INDEX idx_kanban_parent ON kanban_tasks(parent_task_id);

-- ═══════════════════════════════════════════════════════════
-- Kanban Columns
-- ═══════════════════════════════════════════════════════════
CREATE TABLE kanban_columns (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 100),
    status_key TEXT NOT NULL CHECK(length(status_key) > 0 AND length(status_key) <= 50),
    color TEXT NOT NULL DEFAULT '#6366f1',
    sort_order INTEGER NOT NULL DEFAULT 0,
    wip_limit INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, status_key)
);

CREATE INDEX idx_kanban_cols_novel ON kanban_columns(novel_id);

-- ═══════════════════════════════════════════════════════════
-- Loop States
-- ═══════════════════════════════════════════════════════════
CREATE TABLE loop_states (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    pattern_id TEXT NOT NULL CHECK(length(pattern_id) > 0 AND length(pattern_id) <= 100),
    status TEXT NOT NULL DEFAULT 'idle' CHECK(status IN ('idle', 'running', 'paused', 'error')),
    readiness_level TEXT NOT NULL DEFAULT 'L0' CHECK(readiness_level IN ('L0', 'L1', 'L2', 'L3')),
    state_payload TEXT NOT NULL DEFAULT '{}',
    config TEXT NOT NULL DEFAULT '{}',
    token_usage_today INTEGER NOT NULL DEFAULT 0 CHECK(token_usage_today >= 0),
    token_cap_daily INTEGER NOT NULL DEFAULT 50000 CHECK(token_cap_daily > 0),
    last_run_at TEXT,
    last_run_result TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE
);

CREATE INDEX idx_loop_states_novel ON loop_states(novel_id);
CREATE INDEX idx_loop_states_pattern ON loop_states(novel_id, pattern_id);

-- ═══════════════════════════════════════════════════════════
-- Loop Run Logs
-- ═══════════════════════════════════════════════════════════
CREATE TABLE loop_run_logs (
    id TEXT PRIMARY KEY,
    loop_state_id TEXT NOT NULL,
    pattern_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('success', 'partial', 'failed', 'escalated')),
    phase_results TEXT NOT NULL DEFAULT '[]',
    tokens_used INTEGER NOT NULL DEFAULT 0 CHECK(tokens_used >= 0),
    duration_ms INTEGER NOT NULL DEFAULT 0 CHECK(duration_ms >= 0),
    findings TEXT NOT NULL DEFAULT '[]',
    actions_taken TEXT NOT NULL DEFAULT '[]',
    escalations TEXT NOT NULL DEFAULT '[]',
    error_message TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (loop_state_id) REFERENCES loop_states(id) ON DELETE CASCADE
);

CREATE INDEX idx_loop_logs_state ON loop_run_logs(loop_state_id);
CREATE INDEX idx_loop_logs_created ON loop_run_logs(loop_state_id, created_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Loop Patterns
-- ═══════════════════════════════════════════════════════════
CREATE TABLE loop_patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 200),
    description TEXT NOT NULL DEFAULT '',
    goal TEXT NOT NULL DEFAULT '',
    cadence TEXT NOT NULL DEFAULT '1d',
    risk_level TEXT NOT NULL DEFAULT 'low' CHECK(risk_level IN ('low', 'medium', 'high')),
    phases TEXT NOT NULL DEFAULT '[]',
    human_gates TEXT NOT NULL DEFAULT '[]',
    cost_config TEXT NOT NULL DEFAULT '{}',
    skills_required TEXT NOT NULL DEFAULT '[]',
    state_schema TEXT NOT NULL DEFAULT '{}',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- AI 日志：LLM 调用（原 logs.sqlite 合并）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE llm_calls (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_role TEXT NOT NULL DEFAULT 'chat',
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    system_prompt TEXT,
    messages_json TEXT NOT NULL DEFAULT '[]',
    tools_json TEXT,
    temperature REAL,
    max_tokens INTEGER,
    response_content TEXT,
    response_tool_calls TEXT,
    finish_reason TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL DEFAULT 0 CHECK(cache_read_tokens >= 0),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    latency_ms INTEGER CHECK(latency_ms IS NULL OR latency_ms >= 0),
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'timeout')),
    error_message TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_llm_calls_session ON llm_calls(session_id, created_at);
CREATE INDEX idx_llm_calls_model ON llm_calls(model, created_at);
CREATE INDEX idx_llm_calls_status ON llm_calls(status, created_at);

-- ═══════════════════════════════════════════════════════════
-- AI 日志：工具执行
-- ═══════════════════════════════════════════════════════════
CREATE TABLE tool_executions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    llm_call_id TEXT,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    result_content TEXT,
    is_error INTEGER NOT NULL DEFAULT 0 CHECK(is_error IN (0, 1)),
    error_message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    sandbox_allowed INTEGER NOT NULL DEFAULT 1 CHECK(sandbox_allowed IN (0, 1)),
    sandbox_violation TEXT,
    pve_blocked INTEGER NOT NULL DEFAULT 0 CHECK(pve_blocked IN (0, 1)),
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (llm_call_id) REFERENCES llm_calls(id) ON DELETE SET NULL
);

CREATE INDEX idx_tool_exec_session ON tool_executions(session_id, created_at);
CREATE INDEX idx_tool_exec_tool ON tool_executions(tool_name, created_at);
CREATE INDEX idx_tool_exec_errors ON tool_executions(is_error, created_at);

-- ═══════════════════════════════════════════════════════════
-- AI 日志：Agent 思考过程
-- ═══════════════════════════════════════════════════════════
CREATE TABLE agent_thinking (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    llm_call_id TEXT,
    thinking_content TEXT NOT NULL,
    thinking_level TEXT DEFAULT 'medium',
    thinking_tokens INTEGER NOT NULL DEFAULT 0 CHECK(thinking_tokens >= 0),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (llm_call_id) REFERENCES llm_calls(id) ON DELETE SET NULL
);

CREATE INDEX idx_thinking_session ON agent_thinking(session_id, created_at);

-- ═══════════════════════════════════════════════════════════
-- AI 日志：沙箱违规
-- ═══════════════════════════════════════════════════════════
CREATE TABLE sandbox_violations (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    violation_type TEXT NOT NULL,
    resource TEXT NOT NULL,
    action TEXT NOT NULL,
    rule_matched TEXT,
    tool_name TEXT,
    arguments_json TEXT,
    detected_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
);

CREATE INDEX idx_violations_session ON sandbox_violations(session_id, detected_at);
CREATE INDEX idx_violations_type ON sandbox_violations(violation_type, detected_at);

-- ═══════════════════════════════════════════════════════════
-- AI 日志：记忆操作
-- ═══════════════════════════════════════════════════════════
CREATE TABLE memory_operations (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    book_id TEXT NOT NULL,
    operation TEXT NOT NULL CHECK(operation IN ('archive', 'search', 'page_in', 'page_out')),
    entry_type TEXT,
    chapter INTEGER,
    content_preview TEXT,
    search_query TEXT,
    search_results_count INTEGER,
    performed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
);

CREATE INDEX idx_memory_ops_book ON memory_operations(book_id, performed_at);
CREATE INDEX idx_memory_ops_session ON memory_operations(session_id, performed_at);
