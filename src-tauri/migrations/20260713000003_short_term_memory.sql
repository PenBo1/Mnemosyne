-- 短期记忆系统 —— 按日期存储 session 摘要。
--
-- 用 SQLite 持久化而非 markdown 文件,优势:
-- 1. 可索引(按 date / session_id / agent_role 查询)
-- 2. 可聚合(每晚合并短期→长期 MEMORY.md)
-- 3. 可 GC(30 天滚动保留)
--
-- 与 memory_entries 表的区别:
-- - memory_entries 是 book-scoped 章节记忆(Character/Plot/Setting)
-- - memory_short_term 是 session-scoped 会话摘要(每条 = 一个 session 的当天摘要)
--
-- 与 MEMORY.md(长期记忆)的区别:
-- - MEMORY.md 是 agent 跨 session 持久化教训(低频更新)
-- - memory_short_term 是 session 级日志(高频写入,每日合并到 MEMORY.md)

CREATE TABLE memory_short_term (
    id TEXT PRIMARY KEY,                    -- ULID/UUID
    session_id TEXT NOT NULL,              -- 关联 sessions.id
    book_id TEXT,                           -- 可选,关联 books.id(若 session 绑定 book)
    entry_date TEXT NOT NULL,              -- YYYY-MM-DD(用于按日查询)
    summary TEXT NOT NULL,                 -- LLM 生成的 session 摘要(2-3 句话)
    key_topics TEXT NOT NULL DEFAULT '[]',  -- JSON 数组,如 ["chapter-3","dialogue","plot-hole"]
    agent_role TEXT,                        -- 可选,agent 角色(如 main/auditor/reviser)
    token_count INTEGER NOT NULL DEFAULT 0, -- 本次 session 的总 token 用量
    message_count INTEGER NOT NULL DEFAULT 0, -- 本次 session 的消息数
    created_at TEXT NOT NULL,              -- ISO8601 时间戳
    UNIQUE(session_id, entry_date)          -- 同一 session 同一天只保留一条最新
);

CREATE INDEX idx_short_term_date ON memory_short_term(entry_date);
CREATE INDEX idx_short_term_session ON memory_short_term(session_id);
CREATE INDEX idx_short_term_book ON memory_short_term(book_id);
