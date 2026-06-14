-- Mnemosyne Database Schema
-- State DB: Core business data (novels, chapters, sessions, messages, agents)

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════
-- Workspaces
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
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
    title TEXT NOT NULL,
    genre TEXT NOT NULL DEFAULT 'general',
    platform TEXT NOT NULL DEFAULT 'local',
    status TEXT NOT NULL DEFAULT 'drafting',
    language TEXT NOT NULL DEFAULT 'zh',
    word_count INTEGER NOT NULL DEFAULT 0,
    chapter_count INTEGER NOT NULL DEFAULT 0,
    target_chapters INTEGER NOT NULL DEFAULT 100,
    chapter_words INTEGER NOT NULL DEFAULT 3000,
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
    number INTEGER NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'drafting',
    word_count INTEGER NOT NULL DEFAULT 0,
    audit_score REAL,
    revision_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, number)
);

CREATE INDEX IF NOT EXISTS idx_chapters_novel ON chapters(novel_id, number);

-- ═══════════════════════════════════════════════════════════
-- Sessions
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    novel_id TEXT,
    session_type TEXT NOT NULL DEFAULT 'chat',
    title TEXT NOT NULL DEFAULT '',
    summary TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'active',
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
    role TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    tool_calls TEXT,
    tool_results TEXT,
    token_count INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);

-- ═══════════════════════════════════════════════════════════
-- Agents
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT 'gpt-4',
    system_prompt TEXT NOT NULL DEFAULT '',
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL
);

-- ═══════════════════════════════════════════════════════════
-- Prompts
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
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
    keyword TEXT NOT NULL,
    platform TEXT NOT NULL,
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
