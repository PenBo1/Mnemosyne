-- S4：时序记忆库
-- 把 StoryState.facts 和 StoryState.summaries 从单一 JSON 文件迁移到 SQLite，
-- 支持按章节范围查询（valid_from_chapter / valid_until_chapter 时序语义）。
--
-- 设计：
-- - story_facts：时序事实表，按 novel_id + chapter 范围索引
-- - chapter_summaries：章节摘要表，按 novel_id + chapter 唯一索引
--
-- 与 state.json 的关系：StateManager 仍保留 state.json 作为 hooks 和元信息存储，
-- facts/summaries 双写到 SQLite。调用方可选择从 SQLite 查询（时序）或从 state.json
-- 读取（全量）。后续阶段可逐步淘汰 state.json 中的 facts/summaries 字段。

-- ═══════════════════════════════════════════════════════════
-- Story Facts（时序事实表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE story_facts (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    fact_id TEXT NOT NULL CHECK(length(fact_id) > 0 AND length(fact_id) <= 200),
    subject TEXT NOT NULL DEFAULT '',
    predicate TEXT NOT NULL DEFAULT '',
    object TEXT NOT NULL DEFAULT '',
    -- 时序语义：在 [valid_from_chapter, valid_until_chapter) 区间内有效
    -- valid_until_chapter 为 NULL 表示"至今仍有效"
    valid_from_chapter INTEGER NOT NULL CHECK(valid_from_chapter >= 0),
    valid_until_chapter INTEGER CHECK(valid_until_chapter IS NULL OR valid_until_chapter > valid_from_chapter),
    source_chapter INTEGER NOT NULL CHECK(source_chapter >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    -- 同一 novel 内 fact_id 唯一（upsert 时按 fact_id 合并）
    UNIQUE(novel_id, fact_id)
);

-- 时序查询：按 chapter 范围找有效 facts
CREATE INDEX idx_story_facts_novel_chapter ON story_facts(novel_id, valid_from_chapter, valid_until_chapter);
CREATE INDEX idx_story_facts_novel_subject ON story_facts(novel_id, subject);
CREATE INDEX idx_story_facts_novel_source ON story_facts(novel_id, source_chapter);

-- ═══════════════════════════════════════════════════════════
-- Chapter Summaries（章节摘要表）
-- ═══════════════════════════════════════════════════════════
CREATE TABLE chapter_summaries (
    id TEXT PRIMARY KEY,
    novel_id TEXT NOT NULL,
    chapter INTEGER NOT NULL CHECK(chapter > 0),
    title TEXT NOT NULL DEFAULT '',
    -- characters / events / state_changes / hook_activity 用 JSON 数组存储
    -- （查询时不需展开，按 novel_id + chapter 取即可）
    characters_json TEXT NOT NULL DEFAULT '[]',
    events_json TEXT NOT NULL DEFAULT '[]',
    state_changes_json TEXT NOT NULL DEFAULT '[]',
    hook_activity_json TEXT NOT NULL DEFAULT '[]',
    mood TEXT NOT NULL DEFAULT '',
    chapter_type TEXT NOT NULL DEFAULT 'other',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (novel_id) REFERENCES novels(id) ON DELETE CASCADE,
    -- 同一 novel 内 chapter 唯一（upsert 时按 chapter 替换）
    UNIQUE(novel_id, chapter)
);

CREATE INDEX idx_chapter_summaries_novel_chapter ON chapter_summaries(novel_id, chapter);
CREATE INDEX idx_chapter_summaries_novel_type ON chapter_summaries(novel_id, chapter_type);
