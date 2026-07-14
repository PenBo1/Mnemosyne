-- P2.4: MEMORY.md 归档索引表
--
-- daily_summary 任务的 archive_old_memory 在 MEMORY.md 超 100KB 时
-- 导出 7 天前内容到 MEMORY.archive.<date>.md。本表记录归档元数据，
-- 供前端检索/搜索归档内容，避免只能 ls 文件目录。
--
-- 与 memory_entries（agent 工作记忆）不同：本表只记录"归档文件元信息"，
-- 不存储实际内容（内容在 .md 文件中）。
--
-- 多 role 支持：每个 agent role（main/planner/writer/...共 16 个）
-- 各自有独立的 MEMORY.md 和归档文件，role 字段区分。

CREATE TABLE memory_archives (
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

CREATE INDEX idx_memory_archives_role ON memory_archives(role);
CREATE INDEX idx_memory_archives_archived_at ON memory_archives(archived_at DESC);
