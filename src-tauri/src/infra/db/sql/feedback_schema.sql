-- Mnemosyne Feedback Database Schema
-- Error events, lessons, gate evaluations, pipeline runs

PRAGMA journal_mode = WAL;

-- ═══════════════════════════════════════════════════════════
-- Error Events
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS error_events (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    agent_role TEXT NOT NULL,
    error_type TEXT NOT NULL,
    dimension TEXT,
    severity TEXT NOT NULL DEFAULT 'warning',
    description TEXT NOT NULL,
    suggestion TEXT,
    lesson_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_error_events_novel ON error_events(novel_id, error_type);
CREATE INDEX IF NOT EXISTS idx_error_events_lesson ON error_events(lesson_id);

-- ═══════════════════════════════════════════════════════════
-- Constraint Lessons
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS lessons (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    error_type TEXT NOT NULL,
    constraint_text TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    first_seen_chapter INTEGER NOT NULL,
    last_seen_chapter INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'active',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    suppressed_at TEXT,
    archived_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_lessons_novel_state ON lessons(novel_id, state);
CREATE INDEX IF NOT EXISTS idx_lessons_novel_type ON lessons(novel_id, error_type);

-- ═══════════════════════════════════════════════════════════
-- Gate Evaluations
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS gate_evaluations (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    stage TEXT NOT NULL,
    total_gates INTEGER NOT NULL,
    passed_gates INTEGER NOT NULL,
    failed_gates INTEGER NOT NULL,
    overall_passed INTEGER NOT NULL,
    recommended_action TEXT NOT NULL,
    evaluation_time_ms INTEGER NOT NULL,
    gate_results TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gate_eval_novel ON gate_evaluations(novel_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_gate_eval_stage ON gate_evaluations(novel_id, stage);

-- ═══════════════════════════════════════════════════════════
-- Pipeline Runs
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS pipeline_runs (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER,
    tokens_used INTEGER,
    cost REAL,
    error_message TEXT,
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_pipeline_runs_novel ON pipeline_runs(novel_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_stage ON pipeline_runs(novel_id, stage);
