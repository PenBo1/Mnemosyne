//! ═══════════════════════════════════════════════════════════════════════════
//! 记忆归档存储 - MEMORY.md 归档索引
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 用途：
//! - daily_summary 任务在 MEMORY.md 超 100KB 时导出 7 天前内容
//! - 前端通过 IPC 浏览历史归档，不必扫描文件目录

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── SQL 语句 ────────────────────────────────────────────────────────────────

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

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 归档行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryArchiveRow {
    /// 归档 ID
    pub id: i64,
    /// 角色
    pub role: String,
    /// 归档文件名
    pub archive_file: String,
    /// 归档时间（Unix 秒）
    pub archived_at: i64,
    /// 内容大小
    pub content_size: i64,
    /// 内容摘要
    pub content_summary: String,
    /// 日期范围起始
    pub date_range_start: String,
    /// 日期范围结束
    pub date_range_end: String,
    /// 创建时间
    pub created_at: String,
}

/// 新建归档参数
#[derive(Debug, Clone)]
pub struct NewMemoryArchive<'a> {
    /// 角色
    pub role: &'a str,
    /// 归档文件名
    pub archive_file: &'a str,
    /// 归档时间（Unix 秒）
    pub archived_at: i64,
    /// 内容大小
    pub content_size: i64,
    /// 内容摘要
    pub content_summary: &'a str,
    /// 日期范围起始
    pub date_range_start: &'a str,
    /// 日期范围结束
    pub date_range_end: &'a str,
    /// 创建时间
    pub created_at: &'a str,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到归档行
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

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 插入或更新归档元数据
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
        let id: i64 = conn.query_row(
            "SELECT id FROM memory_archives WHERE role = ?1 AND archive_file = ?2",
            params![row.role, row.archive_file],
            |r| r.get(0),
        ).map_err(db_err)?;
        Ok(id)
    }

    /// 列出某角色的所有归档
    pub fn list_archives(&self, role: &str) -> Result<Vec<MemoryArchiveRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives WHERE role = ?1 \
             ORDER BY archived_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([role], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 列出所有归档
    pub fn list_all_archives(&self) -> Result<Vec<MemoryArchiveRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM memory_archives ORDER BY archived_at DESC",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let rows = stmt.query_map([], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 在摘要中搜索
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
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按 ID 获取归档
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

    /// 删除归档记录
    pub fn delete_archive(&self, id: i64) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM memory_archives WHERE id = ?1",
            params![id],
        ).map_err(db_err)?;
        Ok(affected > 0)
    }
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

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

        let deleted_again = db.delete_archive(id).unwrap();
        assert!(!deleted_again);

        let remaining = db.list_archives("main").unwrap();
        assert!(remaining.is_empty());
    }
}