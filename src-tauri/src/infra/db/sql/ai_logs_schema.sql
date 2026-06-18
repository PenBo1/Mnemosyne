-- Mnemosyne AI Logs Schema
-- Comprehensive persistent storage for all AI-related data
-- Designed for analysis, debugging, and audit trails

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════
-- LLM API Calls - Every LLM request/response
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS llm_calls (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_role TEXT NOT NULL DEFAULT 'chat',
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    -- Request data
    system_prompt TEXT,
    messages_json TEXT NOT NULL DEFAULT '[]',
    tools_json TEXT,
    temperature REAL,
    max_tokens INTEGER,
    -- Response data
    response_content TEXT,
    response_tool_calls TEXT,
    finish_reason TEXT,
    -- Token usage
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL DEFAULT 0 CHECK(cache_read_tokens >= 0),
    -- Timing
    started_at TEXT NOT NULL,
    completed_at TEXT,
    latency_ms INTEGER CHECK(latency_ms IS NULL OR latency_ms >= 0),
    -- Status
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'timeout')),
    error_message TEXT,
    -- Metadata
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_llm_calls_session ON llm_calls(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_llm_calls_model ON llm_calls(model, created_at);
CREATE INDEX IF NOT EXISTS idx_llm_calls_status ON llm_calls(status, created_at);

-- ═══════════════════════════════════════════════════════════
-- Tool Executions - Every tool call and result
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS tool_executions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    llm_call_id TEXT,
    -- Tool info
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    -- Result
    result_content TEXT,
    is_error INTEGER NOT NULL DEFAULT 0 CHECK(is_error IN (0, 1)),
    error_message TEXT,
    -- Timing
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    -- Security
    sandbox_allowed INTEGER NOT NULL DEFAULT 1 CHECK(sandbox_allowed IN (0, 1)),
    sandbox_violation TEXT,
    pve_blocked INTEGER NOT NULL DEFAULT 0 CHECK(pve_blocked IN (0, 1)),
    -- Metadata
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tool_exec_session ON tool_executions(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_tool_exec_tool ON tool_executions(tool_name, created_at);
CREATE INDEX IF NOT EXISTS idx_tool_exec_errors ON tool_executions(is_error, created_at);

-- ═══════════════════════════════════════════════════════════
-- Agent Thinking - Reasoning/thinking traces
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS agent_thinking (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    llm_call_id TEXT,
    -- Thinking data
    thinking_content TEXT NOT NULL,
    thinking_level TEXT DEFAULT 'medium',
    thinking_tokens INTEGER NOT NULL DEFAULT 0 CHECK(thinking_tokens >= 0),
    -- Timing
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_thinking_session ON agent_thinking(session_id, created_at);

-- ═══════════════════════════════════════════════════════════
-- Pipeline Stage Logs - Per-stage execution details
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS pipeline_stage_logs (
    id TEXT PRIMARY KEY,
    pipeline_run_id TEXT NOT NULL,
    novel_id TEXT NOT NULL,
    chapter_number INTEGER NOT NULL,
    -- Stage info
    stage_name TEXT NOT NULL,
    agent_role TEXT NOT NULL,
    model TEXT,
    -- Execution
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'skipped')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER CHECK(duration_ms IS NULL OR duration_ms >= 0),
    -- I/O
    input_summary TEXT,
    output_summary TEXT,
    -- Tokens
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    -- Quality
    audit_score REAL CHECK(audit_score IS NULL OR (audit_score >= 0.0 AND audit_score <= 10.0)),
    gate_passed INTEGER CHECK(gate_passed IS NULL OR gate_passed IN (0, 1)),
    -- Error
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK(retry_count >= 0),
    recovery_strategy TEXT,
    -- Metadata
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_stage_logs_pipeline ON pipeline_stage_logs(pipeline_run_id, created_at);
CREATE INDEX IF NOT EXISTS idx_stage_logs_novel ON pipeline_stage_logs(novel_id, chapter_number);
CREATE INDEX IF NOT EXISTS idx_stage_logs_stage ON pipeline_stage_logs(stage_name, status);

-- ═══════════════════════════════════════════════════════════
-- Sandbox Violations - Security audit trail
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS sandbox_violations (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    -- Violation info
    violation_type TEXT NOT NULL,
    resource TEXT NOT NULL,
    action TEXT NOT NULL,
    rule_matched TEXT,
    -- Context
    tool_name TEXT,
    arguments_json TEXT,
    -- Timing
    detected_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_violations_session ON sandbox_violations(session_id, detected_at);
CREATE INDEX IF NOT EXISTS idx_violations_type ON sandbox_violations(violation_type, detected_at);

-- ═══════════════════════════════════════════════════════════
-- Memory Operations - What was archived/searched
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS memory_operations (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    book_id TEXT NOT NULL,
    -- Operation
    operation TEXT NOT NULL CHECK(operation IN ('archive', 'search', 'page_in', 'page_out')),
    entry_type TEXT,
    chapter INTEGER,
    content_preview TEXT,
    search_query TEXT,
    search_results_count INTEGER,
    -- Timing
    performed_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_ops_book ON memory_operations(book_id, performed_at);
CREATE INDEX IF NOT EXISTS idx_memory_ops_session ON memory_operations(session_id, performed_at);

-- ═══════════════════════════════════════════════════════════
-- Enhanced Messages - Add thinking and model fields
-- ═══════════════════════════════════════════════════════════
-- Note: ALTER TABLE for existing messages table
-- Adding thinking_content, model, provider columns

-- These columns are added via migration in code
-- ALTER TABLE messages ADD COLUMN thinking_content TEXT;
-- ALTER TABLE messages ADD COLUMN model TEXT;
-- ALTER TABLE messages ADD COLUMN provider TEXT;
-- ALTER TABLE messages ADD COLUMN input_tokens INTEGER DEFAULT 0;
-- ALTER TABLE messages ADD COLUMN output_tokens INTEGER DEFAULT 0;
-- ALTER TABLE messages ADD COLUMN latency_ms INTEGER;
