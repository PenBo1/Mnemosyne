-- Loop-Engineering 运行日志表。
--
-- - append-only:只追加,不修改(30 天后由 GC 修剪)
-- - 全局可观测性:所有 pattern 的运行记录汇聚于此
-- - run_id 关联:ISO8601 时间戳作为唯一标识
--
-- 与 chapter_snapshots 的区别:
-- - snapshots:per-chapter 的内容快照(状态)
-- - loop_runs:跨章节的执行流记录(历史)

CREATE TABLE loop_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,              -- ISO8601 时间戳
    pattern_id TEXT NOT NULL,                 -- chapter-write-loop / audit-revise-loop 等
    book_id TEXT,                             -- 关联书籍(可为空,如全局 triage)
    chapter_number INTEGER,                   -- 关联章节(可为空)
    started_at TEXT NOT NULL,                 -- ISO8601 开始时间
    ended_at TEXT,                            -- ISO8601 结束时间(运行中为 NULL)
    duration_s INTEGER,                       -- 运行时长(秒)
    outcome TEXT NOT NULL DEFAULT 'running',  -- running/report-only/fix-proposed/escalated/no-op/failed
    items_found INTEGER NOT NULL DEFAULT 0,   -- 发现的可操作项数
    actions_taken INTEGER NOT NULL DEFAULT 0, -- 已执行的操作数
    escalations INTEGER NOT NULL DEFAULT 0,   -- 升级到人类的事项数
    tokens_estimate INTEGER NOT NULL DEFAULT 0, -- token 估算
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,      -- revise 循环尝试次数
    notes TEXT,                               -- 自由文本备注
    CHECK(length(run_id) > 0 AND length(run_id) <= 64),
    CHECK(length(pattern_id) > 0 AND length(pattern_id) <= 64),
    CHECK(outcome IN ('running','report-only','fix-proposed','escalated','no-op','failed'))
);

CREATE INDEX idx_loop_runs_pattern ON loop_runs(pattern_id);
CREATE INDEX idx_loop_runs_book ON loop_runs(book_id);
CREATE INDEX idx_loop_runs_started_at ON loop_runs(started_at);
CREATE INDEX idx_loop_runs_pattern_started ON loop_runs(pattern_id, started_at);
