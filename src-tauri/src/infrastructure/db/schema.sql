-- Mnemosyne 统一 schema（state.sqlite）
-- 合并自原 23 个迁移文件，开发阶段单一最终态 schema。
-- 由 migrate.rs::run_migrate 通过 execute_batch 一次性应用。
--
-- 注意：
-- - PRAGMA 必须在事务外执行，由 Database::new() 在连接池建立后设置。
-- - 所有 CREATE 语句使用 IF NOT EXISTS，保证 execute_batch 幂等。
-- - sessions / loop_runs / loop_patterns / workspaces 已合并为最终态（含所有 ALTER 增量列）。
-- - 已删除的 8 张死表（kanban_* / llm_calls / tool_executions / agent_thinking /
--   sandbox_violations / memory_operations）不再包含。

-- ═══════════════════════════════════════════════════════════
-- Workspaces（含 last_opened_at 扩展列）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
    path TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_opened_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_workspaces_last_opened ON workspaces(last_opened_at DESC);

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
-- Sessions（最终态：含 workspace_id / parent_session_id / split_type / split_reason）
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
    workspace_id TEXT,
    parent_session_id TEXT,
    split_type TEXT CHECK(split_type IS NULL OR split_type IN ('branch', 'compression', 'delegate')),
    split_reason TEXT,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_novel ON sessions(novel_id);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON sessions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);

-- ═══════════════════════════════════════════════════════════
-- Messages（含 thinking/model/provider/tokens/latency 列）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL DEFAULT '',
    tool_calls TEXT,
    tool_results TEXT,
    token_count INTEGER CHECK(token_count IS NULL OR token_count >= 0),
    thinking_content TEXT,
    model TEXT,
    provider TEXT,
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK(output_tokens >= 0),
    latency_ms INTEGER CHECK(latency_ms IS NULL OR latency_ms >= 0),
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);

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
CREATE INDEX IF NOT EXISTS idx_prompts_updated ON prompts(updated_at DESC);

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
CREATE INDEX IF NOT EXISTS idx_trends_scanned ON trends(scanned_at DESC);

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
-- Wiki Entries
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

-- Wiki FTS5（中文用 unicode61，TODO 后续换 trigram）
CREATE VIRTUAL TABLE IF NOT EXISTS wiki_entries_fts USING fts5(
    title, content, tags,
    content='wiki_entries', content_rowid='rowid',
    tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS wiki_fts_insert AFTER INSERT ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS wiki_fts_update AFTER UPDATE ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(wiki_entries_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
    INSERT INTO wiki_entries_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS wiki_fts_delete AFTER DELETE ON wiki_entries BEGIN
    INSERT INTO wiki_entries_fts(wiki_entries_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
END;

-- ═══════════════════════════════════════════════════════════
-- Wiki Entity Links
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
-- Chapter Versions
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
CREATE INDEX IF NOT EXISTS idx_loop_states_status ON loop_states(status);

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
-- Loop Patterns（含 is_builtin 扩展列）
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
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_loop_patterns_builtin ON loop_patterns(is_builtin);
CREATE INDEX IF NOT EXISTS idx_loop_patterns_active ON loop_patterns(is_active);

-- ═══════════════════════════════════════════════════════════
-- Loop Runs（最终态：含 6 个扩展列）
-- - append-only:只追加,不修改(30 天后由 GC 修剪)
-- - 全局可观测性:所有 pattern 的运行记录汇聚于此
-- - run_id 关联:ISO8601 时间戳作为唯一标识
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS loop_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    pattern_id TEXT NOT NULL,
    book_id TEXT,
    chapter_number INTEGER,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_s INTEGER,
    outcome TEXT NOT NULL DEFAULT 'running',
    items_found INTEGER NOT NULL DEFAULT 0,
    actions_taken INTEGER NOT NULL DEFAULT 0,
    escalations INTEGER NOT NULL DEFAULT 0,
    tokens_estimate INTEGER NOT NULL DEFAULT 0,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    loop_state_id TEXT,
    findings_json TEXT,
    actions_json TEXT,
    escalations_json TEXT,
    phase_results_json TEXT,
    error_message TEXT,
    CHECK(length(run_id) > 0 AND length(run_id) <= 64),
    CHECK(length(pattern_id) > 0 AND length(pattern_id) <= 64),
    CHECK(outcome IN ('running','report-only','fix-proposed','escalated','no-op','failed'))
);

CREATE INDEX IF NOT EXISTS idx_loop_runs_pattern ON loop_runs(pattern_id);
CREATE INDEX IF NOT EXISTS idx_loop_runs_book ON loop_runs(book_id);
CREATE INDEX IF NOT EXISTS idx_loop_runs_started_at ON loop_runs(started_at);
CREATE INDEX IF NOT EXISTS idx_loop_runs_pattern_started ON loop_runs(pattern_id, started_at);
CREATE INDEX IF NOT EXISTS idx_loop_runs_state ON loop_runs(loop_state_id);

-- ═══════════════════════════════════════════════════════════
-- Story Facts（时序事实表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS story_facts (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    fact_id TEXT NOT NULL CHECK(length(fact_id) > 0 AND length(fact_id) <= 200),
    subject TEXT NOT NULL DEFAULT '',
    predicate TEXT NOT NULL DEFAULT '',
    object TEXT NOT NULL DEFAULT '',
    valid_from_chapter INTEGER NOT NULL CHECK(valid_from_chapter >= 0),
    valid_until_chapter INTEGER CHECK(valid_until_chapter IS NULL OR valid_until_chapter > valid_from_chapter),
    source_chapter INTEGER NOT NULL CHECK(source_chapter >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, fact_id)
);

CREATE INDEX IF NOT EXISTS idx_story_facts_novel_chapter ON story_facts(novel_id, valid_from_chapter, valid_until_chapter);
CREATE INDEX IF NOT EXISTS idx_story_facts_novel_subject ON story_facts(novel_id, subject);
CREATE INDEX IF NOT EXISTS idx_story_facts_novel_source ON story_facts(novel_id, source_chapter);

-- ═══════════════════════════════════════════════════════════
-- Chapter Summaries（章节摘要表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS chapter_summaries (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter INTEGER NOT NULL CHECK(chapter > 0),
    title TEXT NOT NULL DEFAULT '',
    characters_json TEXT NOT NULL DEFAULT '[]',
    events_json TEXT NOT NULL DEFAULT '[]',
    state_changes_json TEXT NOT NULL DEFAULT '[]',
    hook_activity_json TEXT NOT NULL DEFAULT '[]',
    mood TEXT NOT NULL DEFAULT '',
    chapter_type TEXT NOT NULL DEFAULT 'other',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    UNIQUE(novel_id, chapter)
);

CREATE INDEX IF NOT EXISTS idx_chapter_summaries_novel_chapter ON chapter_summaries(novel_id, chapter);
CREATE INDEX IF NOT EXISTS idx_chapter_summaries_novel_type ON chapter_summaries(novel_id, chapter_type);

-- ═══════════════════════════════════════════════════════════
-- Resource Quotas
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS resource_quotas (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    cpu_percent REAL,
    memory_mb INTEGER,
    disk_mb INTEGER,
    network_mb INTEGER,
    token_limit INTEGER,
    cost_limit REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS resource_usage (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    cpu_usage REAL NOT NULL DEFAULT 0,
    memory_usage INTEGER NOT NULL DEFAULT 0,
    disk_usage INTEGER NOT NULL DEFAULT 0,
    network_usage INTEGER NOT NULL DEFAULT 0,
    token_usage INTEGER NOT NULL DEFAULT 0,
    cost_usage REAL NOT NULL DEFAULT 0,
    last_updated TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_resource_quotas_created_at ON resource_quotas(created_at);
CREATE INDEX IF NOT EXISTS idx_resource_usage_last_updated ON resource_usage(last_updated);

-- ═══════════════════════════════════════════════════════════
-- Audit Events（审计事件持久化表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    operation TEXT,
    workspace_id TEXT,
    is_denied INTEGER NOT NULL DEFAULT 0 CHECK(is_denied IN (0, 1)),
    is_security_related INTEGER NOT NULL DEFAULT 0 CHECK(is_security_related IN (0, 1)),
    payload TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_events_type ON audit_events(event_type, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_workspace ON audit_events(workspace_id, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_recorded ON audit_events(recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_denied ON audit_events(is_denied, recorded_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Vectors（向量存储表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS vectors (
    id TEXT PRIMARY KEY,
    workspace_id TEXT,
    doc_type TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    chunk_idx INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,
    dim INTEGER NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vectors_doc ON vectors(doc_type, doc_id);
CREATE INDEX IF NOT EXISTS idx_vectors_workspace ON vectors(workspace_id, model);

-- ═══════════════════════════════════════════════════════════
-- Memory Entries（Agent 记忆加速层）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS memory_entries (
    id TEXT PRIMARY KEY,
    book_id TEXT NOT NULL,
    key TEXT NOT NULL DEFAULT '',
    value TEXT NOT NULL DEFAULT '',
    memory_type TEXT NOT NULL DEFAULT 'fact',
    source TEXT NOT NULL DEFAULT '',
    importance INTEGER NOT NULL DEFAULT 1,
    content TEXT,
    entry_type TEXT,
    chapter TEXT,
    timestamp TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(length(id) > 0 AND length(id) <= 200),
    CHECK(length(book_id) > 0 AND length(book_id) <= 200),
    CHECK(importance >= 0),
    UNIQUE(book_id, id)
);

CREATE INDEX IF NOT EXISTS idx_memory_entries_book ON memory_entries(book_id);
CREATE INDEX IF NOT EXISTS idx_memory_entries_book_type ON memory_entries(book_id, memory_type);
CREATE INDEX IF NOT EXISTS idx_memory_entries_book_key ON memory_entries(book_id, key);
CREATE INDEX IF NOT EXISTS idx_memory_entries_book_importance ON memory_entries(book_id, importance);

-- ═══════════════════════════════════════════════════════════
-- Memory Short Term（短期记忆系统）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS memory_short_term (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    book_id TEXT,
    entry_date TEXT NOT NULL,
    summary TEXT NOT NULL,
    key_topics TEXT NOT NULL DEFAULT '[]',
    agent_role TEXT,
    token_count INTEGER NOT NULL DEFAULT 0,
    message_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(session_id, entry_date)
);

CREATE INDEX IF NOT EXISTS idx_short_term_date ON memory_short_term(entry_date);
CREATE INDEX IF NOT EXISTS idx_short_term_session ON memory_short_term(session_id);
CREATE INDEX IF NOT EXISTS idx_short_term_book ON memory_short_term(book_id);

-- ═══════════════════════════════════════════════════════════
-- Learned Preferences（学习到的用户偏好）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS learned_preferences (
    id TEXT PRIMARY KEY,
    preference_key TEXT NOT NULL,
    preference_value TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.2,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    learned_from TEXT,
    last_seen_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(preference_key, preference_value)
);

CREATE INDEX IF NOT EXISTS idx_learned_pref_key ON learned_preferences(preference_key);
CREATE INDEX IF NOT EXISTS idx_learned_pref_confidence ON learned_preferences(confidence DESC);
CREATE INDEX IF NOT EXISTS idx_learned_pref_last_seen ON learned_preferences(last_seen_at);

-- ═══════════════════════════════════════════════════════════
-- Skill Usage Stats（技能使用统计）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS skill_usage_stats (
    skill_name TEXT PRIMARY KEY,
    used_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    user_feedback_score REAL NOT NULL DEFAULT 0.0,
    first_used_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL,
    last_session_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_skill_usage_last_used ON skill_usage_stats(last_used_at);

-- ═══════════════════════════════════════════════════════════
-- Skill Candidate Proposals（技能候选提案）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS skill_candidate_proposals (
    id TEXT PRIMARY KEY,
    candidate_name TEXT NOT NULL,
    candidate_description TEXT NOT NULL,
    source_session_id TEXT NOT NULL,
    source_summary TEXT NOT NULL,
    candidate_content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'superseded')),
    reviewer_notes TEXT,
    reviewed_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_candidate_status ON skill_candidate_proposals(status);
CREATE INDEX IF NOT EXISTS idx_candidate_name ON skill_candidate_proposals(candidate_name);
CREATE INDEX IF NOT EXISTS idx_candidate_session ON skill_candidate_proposals(source_session_id);

-- ═══════════════════════════════════════════════════════════
-- Trace Spans（OpenTelemetry 风格 span 存储）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS trace_spans (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    parent_span_id TEXT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'internal',
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    attributes TEXT NOT NULL DEFAULT '{}',
    events TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'unset',
    status_message TEXT,
    workspace_id TEXT,
    session_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_trace_spans_trace ON trace_spans(trace_id, start_time ASC);
CREATE INDEX IF NOT EXISTS idx_trace_spans_name ON trace_spans(name, start_time DESC);
CREATE INDEX IF NOT EXISTS idx_trace_spans_start ON trace_spans(start_time DESC);
CREATE INDEX IF NOT EXISTS idx_trace_spans_workspace ON trace_spans(workspace_id, start_time DESC);

-- ═══════════════════════════════════════════════════════════
-- Metric Points（OpenTelemetry 风格 metric 存储）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS metric_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    value REAL NOT NULL,
    attributes TEXT NOT NULL DEFAULT '{}',
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metric_points_name_time ON metric_points(name, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metric_points_time ON metric_points(timestamp DESC);

-- ═══════════════════════════════════════════════════════════
-- Memory Archives（MEMORY.md 归档索引表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS memory_archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    role TEXT NOT NULL CHECK(length(role) > 0 AND length(role) <= 100),
    archive_file TEXT NOT NULL CHECK(length(archive_file) > 0 AND length(archive_file) <= 255),
    archived_at INTEGER NOT NULL CHECK(archived_at > 0),
    content_size INTEGER NOT NULL CHECK(content_size >= 0),
    content_summary TEXT NOT NULL DEFAULT '',
    date_range_start TEXT NOT NULL DEFAULT '',
    date_range_end TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    UNIQUE(role, archive_file)
);

CREATE INDEX IF NOT EXISTS idx_memory_archives_role ON memory_archives(role);
CREATE INDEX IF NOT EXISTS idx_memory_archives_archived_at ON memory_archives(archived_at DESC);

-- ═══════════════════════════════════════════════════════════
-- Interaction Sessions（交互会话持久化表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS interaction_sessions (
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

CREATE INDEX IF NOT EXISTS idx_interaction_sessions_active_book ON interaction_sessions(active_book_id);
CREATE INDEX IF NOT EXISTS idx_interaction_sessions_updated_at ON interaction_sessions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_interaction_sessions_session_kind ON interaction_sessions(session_kind);

-- ═══════════════════════════════════════════════════════════
-- Recall Memory（对话历史检索系统）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS recall_memory (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    message_index INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(session_id, message_index)
);

CREATE INDEX IF NOT EXISTS idx_recall_session ON recall_memory(session_id);
CREATE INDEX IF NOT EXISTS idx_recall_role ON recall_memory(role);
CREATE INDEX IF NOT EXISTS idx_recall_created ON recall_memory(created_at);

-- ═══════════════════════════════════════════════════════════
-- Archival Memory（内容归档与向量检索系统）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS archival_memory (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]',
    citation_json TEXT,
    embedding_vector BLOB,
    embedding_model TEXT,
    embedding_dim INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_archival_created ON archival_memory(created_at);
CREATE INDEX IF NOT EXISTS idx_archival_model ON archival_memory(embedding_model);

-- ═══════════════════════════════════════════════════════════
-- Messages FTS5（trigram tokenizer 支持中文子串匹配）
-- 外部内容表模式：FTS5 索引不存储原文，通过 rowid 关联 messages 表
-- ═══════════════════════════════════════════════════════════
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content='messages',
    content_rowid='rowid',
    tokenize='trigram'
);

-- 回填存量数据（条件 INSERT 确保幂等，重复 execute_batch 不会重复插入）
INSERT INTO messages_fts(rowid, content)
SELECT rowid, content FROM messages
WHERE NOT EXISTS(SELECT 1 FROM messages_fts WHERE rowid = messages.rowid);

-- 触发器：保持 messages_fts 与 messages 同步
CREATE TRIGGER IF NOT EXISTS messages_fts_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;
