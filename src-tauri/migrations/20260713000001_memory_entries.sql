-- S5：Agent 记忆加速层（rusqlite）
-- 把 MemoryStore 从 <data_dir>/memory/<book_id>.json 文件持久化迁移到 SQLite，
-- 支持按 book_id / memory_type / key / importance 索引查询。
--
-- 本地差异:Mnemosyne 没有 markdown truth files，原 JSON 文件即被取代（首次启动时一次性导入旧 JSON）。
--
-- 与 story_facts / chapter_summaries 的关系：
-- - story_facts：时序事实（带 valid_from/until chapter），按 novel_id 索引
-- - chapter_summaries：章节摘要，按 novel_id + chapter 唯一
-- - memory_entries（本表）：无时序的通用 agent 工作记忆 + 归档
--   （Character/Plot/Setting/Dialogue/Style/Fact/Context/Preference/Lesson/Conversation/Research/General）
--   book_id 不强制 FK 到 novels（agent 工作记忆可能绑 workspace 或临时 session）

CREATE TABLE memory_entries (
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

CREATE INDEX idx_memory_entries_book ON memory_entries(book_id);
CREATE INDEX idx_memory_entries_book_type ON memory_entries(book_id, memory_type);
CREATE INDEX idx_memory_entries_book_key ON memory_entries(book_id, key);
CREATE INDEX idx_memory_entries_book_importance ON memory_entries(book_id, importance);
