-- Mnemosyne Database Schema
-- State DB: Core business data (novels, chapters, sessions, messages)

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════
-- Schema Migrations (version tracking)
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

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
