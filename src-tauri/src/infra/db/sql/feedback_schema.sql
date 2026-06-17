-- Mnemosyne Feedback Database Schema
-- Error events, lessons, gate evaluations, pipeline runs

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
-- Error Events
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS error_events (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL CHECK(chapter_number > 0),
    agent_role TEXT NOT NULL CHECK(length(agent_role) > 0 AND length(agent_role) <= 50),
    error_type TEXT NOT NULL CHECK(length(error_type) > 0 AND length(error_type) <= 100),
    dimension TEXT,
    severity TEXT NOT NULL DEFAULT 'warning' CHECK(severity IN ('info', 'warning', 'critical')),
    description TEXT NOT NULL CHECK(length(description) > 0),
    suggestion TEXT,
    lesson_id TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_error_events_novel ON error_events(novel_id, error_type);
CREATE INDEX IF NOT EXISTS idx_error_events_lesson ON error_events(lesson_id);
CREATE INDEX IF NOT EXISTS idx_error_events_chapter ON error_events(novel_id, chapter_number);

-- ═══════════════════════════════════════════════════════════
-- Constraint Lessons
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS lessons (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    error_type TEXT NOT NULL CHECK(length(error_type) > 0),
    constraint_text TEXT NOT NULL CHECK(length(constraint_text) > 0),
    occurrence_count INTEGER NOT NULL DEFAULT 0 CHECK(occurrence_count >= 0),
    first_seen_chapter INTEGER NOT NULL CHECK(first_seen_chapter > 0),
    last_seen_chapter INTEGER NOT NULL CHECK(last_seen_chapter > 0),
    state TEXT NOT NULL DEFAULT 'active' CHECK(state IN ('active', 'suppressed', 'archived')),
    priority INTEGER NOT NULL DEFAULT 0 CHECK(priority >= 0 AND priority <= 10),
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
    chapter_number INTEGER NOT NULL CHECK(chapter_number > 0),
    stage TEXT NOT NULL CHECK(length(stage) > 0 AND length(stage) <= 50),
    total_gates INTEGER NOT NULL CHECK(total_gates >= 0),
    passed_gates INTEGER NOT NULL CHECK(passed_gates >= 0),
    failed_gates INTEGER NOT NULL CHECK(failed_gates >= 0),
    overall_passed INTEGER NOT NULL CHECK(overall_passed IN (0, 1)),
    recommended_action TEXT NOT NULL,
    evaluation_time_ms INTEGER NOT NULL CHECK(evaluation_time_ms >= 0),
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
    chapter_number INTEGER NOT NULL CHECK(chapter_number > 0),
    stage TEXT NOT NULL CHECK(length(stage) > 0 AND length(stage) <= 50),
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    tokens_used INTEGER CHECK(tokens_used IS NULL OR tokens_used >= 0),
    cost REAL CHECK(cost IS NULL OR cost >= 0.0),
    error_message TEXT,
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_pipeline_runs_novel ON pipeline_runs(novel_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_stage ON pipeline_runs(novel_id, stage);
