-- Mnemosyne Database Schema
-- State DB: Core business data (novels, chapters, sessions, messages)

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════
-- Workspaces
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
    path TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- Novels
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS novels (
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

CREATE INDEX IF NOT EXISTS idx_novels_workspace ON novels(workspace_id);

-- ═══════════════════════════════════════════════════════════
-- Chapters
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS chapters (
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

CREATE INDEX IF NOT EXISTS idx_chapters_novel ON chapters(novel_id, number);
CREATE INDEX IF NOT EXISTS idx_chapters_status ON chapters(novel_id, status);

-- ═══════════════════════════════════════════════════════════
-- Sessions
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS sessions (
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

CREATE INDEX IF NOT EXISTS idx_sessions_novel ON sessions(novel_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Messages
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL DEFAULT '',
    tool_calls TEXT,
    tool_results TEXT,
    token_count INTEGER CHECK(token_count IS NULL OR token_count >= 0),
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);

-- ═══════════════════════════════════════════════════════════
-- Prompts
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general' CHECK(length(category) <= 100),
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_prompts_category ON prompts(category);

-- ═══════════════════════════════════════════════════════════
-- Trends
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS trends (
    id TEXT PRIMARY KEY,
    keyword TEXT NOT NULL CHECK(length(keyword) > 0 AND length(keyword) <= 255),
    platform TEXT NOT NULL CHECK(length(platform) > 0 AND length(platform) <= 100),
    score REAL NOT NULL DEFAULT 0.0,
    metadata TEXT NOT NULL DEFAULT '{}',
    scanned_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trends_keyword ON trends(keyword);
CREATE INDEX IF NOT EXISTS idx_trends_platform ON trends(platform);

-- ═══════════════════════════════════════════════════════════
-- Radar Scans
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS radar_scans (
    id TEXT PRIMARY KEY,
    market_summary TEXT NOT NULL,
    recommendations_json TEXT NOT NULL DEFAULT '[]',
    raw_rankings_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_radar_scans_created ON radar_scans(created_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Wiki Entries (Novel-specific Knowledge Base)
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS wiki_entries (
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

CREATE INDEX IF NOT EXISTS idx_wiki_novel ON wiki_entries(novel_id);
CREATE INDEX IF NOT EXISTS idx_wiki_category ON wiki_entries(novel_id, category);
CREATE INDEX IF NOT EXISTS idx_wiki_source_chapter ON wiki_entries(novel_id, source_chapter);

-- ═══════════════════════════════════════════════════════════
-- Wiki Entity Links (Knowledge Graph)
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS wiki_entity_links (
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

CREATE INDEX IF NOT EXISTS idx_wiki_links_novel ON wiki_entity_links(novel_id);
CREATE INDEX IF NOT EXISTS idx_wiki_links_source ON wiki_entity_links(source_entry_id);
CREATE INDEX IF NOT EXISTS idx_wiki_links_target ON wiki_entity_links(target_entry_id);

-- ═══════════════════════════════════════════════════════════
-- Chapter Versions (Diff History)
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS chapter_versions (
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

CREATE INDEX IF NOT EXISTS idx_versions_novel_chapter ON chapter_versions(novel_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_versions_created ON chapter_versions(novel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_versions_hash ON chapter_versions(content_hash);

-- ═══════════════════════════════════════════════════════════
-- Kanban Tasks
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS kanban_tasks (
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

CREATE INDEX IF NOT EXISTS idx_kanban_novel ON kanban_tasks(novel_id);
CREATE INDEX IF NOT EXISTS idx_kanban_status ON kanban_tasks(novel_id, status);
CREATE INDEX IF NOT EXISTS idx_kanban_parent ON kanban_tasks(parent_task_id);

-- ═══════════════════════════════════════════════════════════
-- Kanban Columns
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS kanban_columns (
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

CREATE INDEX IF NOT EXISTS idx_kanban_cols_novel ON kanban_columns(novel_id);

-- ═══════════════════════════════════════════════════════════
-- Loop States
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS loop_states (
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

CREATE INDEX IF NOT EXISTS idx_loop_states_novel ON loop_states(novel_id);
CREATE INDEX IF NOT EXISTS idx_loop_states_pattern ON loop_states(novel_id, pattern_id);

-- ═══════════════════════════════════════════════════════════
-- Loop Run Logs
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS loop_run_logs (
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

CREATE INDEX IF NOT EXISTS idx_loop_logs_state ON loop_run_logs(loop_state_id);
CREATE INDEX IF NOT EXISTS idx_loop_logs_created ON loop_run_logs(loop_state_id, created_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Loop Patterns (Registry)
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS loop_patterns (
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
