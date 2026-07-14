// MEMORY.md 归档索引 CRUD —— memory_archives 表。
//
// 用途:
// - daily_summary 任务的 archive_old_memory 在 MEMORY.md 超 100KB 时
//   导出 7 天前内容到 MEMORY.archive.<date>.md,本表记录归档元数据
// - 前端通过 IPC list/search/read/delete 浏览历史归档,不必扫描文件目录
//
// 架构约束:
// - infrastructure 层只依赖 shared/,不依赖 core/agent/
// - 因此定义 MemoryArchiveRow DTO,role 用 String
// - 业务层(daily_summary)在写归档文件后调用 insert_archive 写元数据
//
// 多 role 支持:每个 agent role(main/planner/writer/...共 16 个)
// 各自有独立的 MEMORY.md 和归档文件,role 字段区分。
// 归档文件路径由业务层拼装:<data_dir>/agents/<role>/MEMORY.archive.<date>.md

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

const UPSERT_SQL: &str = "\
INSERT INTO memory_archives (\
    role, archive_file, archived_at, content_size, \
    content_summary, date_range_start, date_range_end, created_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
ON CONFLICT(role, archive_file) DO UPDATE SET \
    archived_at = excluded.archived_at, \
    content_size = excluded.content_size, \
    content_summary = excluded.content_summary, \
    date_range_start = excluded.date_range_start, \
    date_range_end = excluded.date_range_end, \
    created_at = excluded.created_at";

const SELECT_COLUMNS: &str = "\
id, role, archive_file, archived_at, content_size, \
content_summary, date_range_start, date_range_end, created_at";

/// 归档行(对应 memory_archives 表)
///
/// `archived_at` 为 Unix 秒时间戳(整数,便于排序),
/// `created_at` 为 ISO 8601 字符串(与其它表一致)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryArchiveRow {
    pub id: i64,
    pub role: String,
    /// 归档文件名,如 `MEMORY.archive.2026-07-14.md`
    pub archive_file: String,
    /// 归档发生时间(Unix 秒)
    pub archived_at: i64,
    /// 归档内容字节大小
    pub content_size: i64,
    /// 归档内容摘要(前 200 字符)
    pub content_summary: String,
    /// 归档内容覆盖的日期范围起(YYYY-MM-DD,可能为空)
    pub date_range_start: String,
    /// 归档内容覆盖的日期范围止(YYYY-MM-DD,可能为空)
    pub date_range_end: String,
    pub created_at: String,
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryArchiveRow> {
    Ok(MemoryArchiveRow {
        id: row.get(0)?,
        role: row.get(1)?,
        archive_file: row.get(2)?,
        archived_at: row.get(3)?,
        content_size: row.get(4)?,
        content_summary: row.get(5)?,
        date_range_start: row.get(6)?,
        date_range_end: row.get(7)?,
        created_at: row.get(8)?,
    })
}

/// 新建归档的输入参数(由业务层 daily_summary 装配)
#[derive(Debug, Clone)]
pub struct NewMemoryArchive<'a> {
    pub role: &'a str,
    pub archive_file: &'a str,
    /// Unix 秒时间戳
    pub archived_at: i64,
    pub content_size: i64,
    pub content_summary: &'a str,
    pub date_range_start: &'a str,
    pub date_range_end: &'a str,
    /// ISO 8601 created_at
    pub created_at: &'a str,
}

impl Database {
    /// 插入或更新归档元数据。
    ///
    /// UNIQUE(role, archive_file) 约束保证同 role 同归档文件只保留一条记录。
    /// 重新归档同一天的文件时,UPSERT 覆盖旧元数据。
    pub fn insert_archive(&self, row: &NewMemoryArchive) -> Result<i64, AppError> {
        let conn = self.conn()?;
        conn.execute(
            UPSERT_SQL,
            params![
                row.role,
                row.archive_file,
                row.archived_at,
                row.content_size,
                row.content_summary,
                row.date_range_start,
                row.date_range_end,
                row.created_at,
            ],
        ).map_err(db_err)?;
        // UPSERT 后取 id(role+archive_file 唯一)
        let id: i64 = conn.query_row(
            "SELECT id FROM memory_archives WHERE role = ?1 AND archive_file = ?2",
            params![row.role, row.archive_file],
            |r| r.get(0),
        ).map_err(db_err)?;
        Ok(id)
    }

    /// 列出某 role 的所有归档,按 archived_at 倒序(最新在前)
    pub fn list_archives(&self, role: &str) -> Result<Vec<MemoryArchiveRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives WHERE role = ?1 \
             ORDER BY archived_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([role], map_row).map_err(db_err)?;
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// 列出所有 role 的归档,按 archived_at 倒序
    pub fn list_all_archives(&self) -> Result<Vec<MemoryArchiveRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives ORDER BY archived_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([], map_row).map_err(db_err)?;
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// 在 content_summary 中搜索(LIKE %query%),按 role 过滤
    ///
    /// query 为空时返回空数组(避免全表扫描)
    pub fn search_archives(
        &self,
        role: &str,
        query: &str,
    ) -> Result<Vec<MemoryArchiveRow>, AppError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives \
             WHERE role = ?1 AND content_summary LIKE ?2 \
             ORDER BY archived_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let pattern = format!("%{}%", trimmed);
        let rows = stmt.query_map(params![role, pattern], map_row).map_err(db_err)?;
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// 按 id 查找单条归档(用于 read_archive / delete_archive 前取文件路径)
    pub fn get_archive_by_id(&self, id: i64) -> Result<Option<MemoryArchiveRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives WHERE id = ?1",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let mut rows = stmt.query_map(params![id], map_row).map_err(db_err)?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(db_err)?))
        } else {
            Ok(None)
        }
    }

    /// 按 id 删除归档记录,返回是否删除了行
    ///
    /// 注意:本方法只删 DB 记录,不删磁盘文件(文件删除由 IPC 命令层负责,
    /// 因为 infrastructure 层不应依赖 data_dir 的具体路径布局)。
    pub fn delete_archive(&self, id: i64) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM memory_archives WHERE id = ?1",
            params![id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_in_memory_db() -> Database {
        Database::connect_in_memory().expect("in-memory db should init")
    }

    fn make_new<'a>(
        role: &'a str,
        archive_file: &'a str,
        archived_at: i64,
        summary: &'a str,
        range_start: &'a str,
        range_end: &'a str,
    ) -> NewMemoryArchive<'a> {
        NewMemoryArchive {
            role,
            archive_file,
            archived_at,
            content_size: 2048,
            content_summary: summary,
            date_range_start: range_start,
            date_range_end: range_end,
            created_at: "2026-07-14T12:00:00Z",
        }
    }

    #[test]
    fn insert_then_list_by_role() {
        let db = make_in_memory_db();
        let id1 = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "main agent 第一条归档", "2026-07-01", "2026-07-07",
        )).unwrap();
        let id2 = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-13.md", 1720872000,
            "main agent 第二条归档", "2026-06-30", "2026-07-06",
        )).unwrap();
        db.insert_archive(&make_new(
            "planner", "MEMORY.archive.2026-07-14.md", 1720958400,
            "planner 归档", "2026-07-01", "2026-07-07",
        )).unwrap();

        let main_archives = db.list_archives("main").unwrap();
        assert_eq!(main_archives.len(), 2);
        // 倒序:archived_at 大的在前
        assert_eq!(main_archives[0].id, id1);
        assert_eq!(main_archives[1].id, id2);

        let planner_archives = db.list_archives("planner").unwrap();
        assert_eq!(planner_archives.len(), 1);
    }

    #[test]
    fn upsert_overrides_same_role_and_file() {
        let db = make_in_memory_db();
        let id1 = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "原摘要", "2026-07-01", "2026-07-07",
        )).unwrap();
        let id2 = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "更新后的摘要", "2026-07-02", "2026-07-08",
        )).unwrap();

        assert_eq!(id1, id2, "UPSERT 应保持同一 id");
        let all = db.list_archives("main").unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].content_summary, "更新后的摘要");
        assert_eq!(all[0].date_range_end, "2026-07-08");
    }

    #[test]
    fn list_all_returns_every_role() {
        let db = make_in_memory_db();
        db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "main", "2026-07-01", "2026-07-07",
        )).unwrap();
        db.insert_archive(&make_new(
            "writer", "MEMORY.archive.2026-07-14.md", 1720958400,
            "writer", "2026-07-01", "2026-07-07",
        )).unwrap();

        let all = db.list_all_archives().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn search_by_summary_content() {
        let db = make_in_memory_db();
        db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "主角觉醒剧情关键事件", "2026-07-01", "2026-07-07",
        )).unwrap();
        db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-13.md", 1720872000,
            "配角对话风格调整", "2026-06-30", "2026-07-06",
        )).unwrap();

        let hits = db.search_archives("main", "主角").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].archive_file, "MEMORY.archive.2026-07-14.md");

        let empty = db.search_archives("main", "  ").unwrap();
        assert!(empty.is_empty(), "空 query 应返回空数组");

        let none = db.search_archives("main", "不存在的关键字").unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn get_by_id_returns_row() {
        let db = make_in_memory_db();
        let id = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "test", "2026-07-01", "2026-07-07",
        )).unwrap();

        let fetched = db.get_archive_by_id(id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().role, "main");

        let missing = db.get_archive_by_id(99999).unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn delete_removes_row() {
        let db = make_in_memory_db();
        let id = db.insert_archive(&make_new(
            "main", "MEMORY.archive.2026-07-14.md", 1720958400,
            "to delete", "2026-07-01", "2026-07-07",
        )).unwrap();

        let deleted = db.delete_archive(id).unwrap();
        assert!(deleted);

        // 再删一次应返回 false
        let deleted_again = db.delete_archive(id).unwrap();
        assert!(!deleted_again);

        let remaining = db.list_archives("main").unwrap();
        assert!(remaining.is_empty());
    }
}
