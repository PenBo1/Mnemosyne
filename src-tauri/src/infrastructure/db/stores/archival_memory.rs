//! ═══════════════════════════════════════════════════════════════════════════
//! 归档记忆存储 - Agent 重要内容片段存储
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 用于 Agent 存储重要内容片段，并通过向量相似度搜索检索。
//! 与 vectors 表的区别：
//! - vectors 表是通用的向量索引（支持 workspace/doc_type/doc_id 维度）
//! - archival_memory 专为 Agent 记忆设计，集成 tags/citation 等元数据

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use super::vector::{encode_vector, decode_vector};
use crate::shared::error::AppError;

// ── SQL 语句 ────────────────────────────────────────────────────────────────

const UPSERT_SQL: &str = "\
INSERT INTO archival_memory (\
    id, content, tags_json, citation_json, embedding_vector, \
    embedding_model, embedding_dim, created_at, updated_at\
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
ON CONFLICT(id) DO UPDATE SET \
    content = excluded.content, \
    tags_json = excluded.tags_json, \
    citation_json = excluded.citation_json, \
    embedding_vector = excluded.embedding_vector, \
    embedding_model = excluded.embedding_model, \
    embedding_dim = excluded.embedding_dim, \
    updated_at = excluded.updated_at";

const SELECT_COLUMNS: &str = "\
id, content, tags_json, citation_json, embedding_vector, \
embedding_model, embedding_dim, created_at, updated_at";

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 归档记忆行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivalMemoryRow {
    /// 记忆 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// 标签 JSON
    pub tags_json: String,
    /// 引用 JSON
    pub citation_json: Option<String>,
    /// 嵌入模型
    pub embedding_model: Option<String>,
    /// 嵌入维度
    pub embedding_dim: Option<i64>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 归档记忆搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivalSearchResult {
    /// 记忆行
    pub row: ArchivalMemoryRow,
    /// 相似度评分
    pub score: f32,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到归档记忆行
fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ArchivalMemoryRow> {
    Ok(ArchivalMemoryRow {
        id: row.get(0)?,
        content: row.get(1)?,
        tags_json: row.get(2)?,
        citation_json: row.get(3)?,
        embedding_model: row.get(5)?,
        embedding_dim: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// 包含向量的内部行结构
struct ArchivalMemoryRowInner {
    id: String,
    content: String,
    tags_json: String,
    citation_json: Option<String>,
    embedding_vec: Vec<u8>,
    embedding_model: Option<String>,
    embedding_dim: Option<i64>,
    created_at: String,
    updated_at: String,
}

/// 映射数据库行到内部行结构
fn map_row_with_vector(row: &rusqlite::Row) -> rusqlite::Result<ArchivalMemoryRowInner> {
    let vec_bytes: Option<Vec<u8>> = row.get(4)?;
    Ok(ArchivalMemoryRowInner {
        id: row.get(0)?,
        content: row.get(1)?,
        tags_json: row.get(2)?,
        citation_json: row.get(3)?,
        embedding_vec: vec_bytes.unwrap_or_default(),
        embedding_model: row.get(5)?,
        embedding_dim: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// 计算余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 插入或更新归档记忆
    pub fn upsert_archival_memory(
        &self,
        id: &str,
        content: &str,
        tags_json: &str,
        citation_json: Option<&str>,
        embedding_vec: Option<&[f32]>,
        embedding_model: Option<&str>,
        embedding_dim: Option<usize>,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        let vec_blob = embedding_vec.map(encode_vector);
        let dim_i64 = embedding_dim.map(|d| d as i64);

        let conn = self.conn()?;
        conn.execute(
            UPSERT_SQL,
            params![
                id,
                content,
                tags_json,
                citation_json,
                vec_blob,
                embedding_model,
                dim_i64,
                &now,
                &now,
            ],
        ).map_err(db_err)?;
        Ok(())
    }

    /// 按内容搜索归档记忆
    pub fn search_archival_memory_by_content(
        &self,
        query: &str,
        tags_filter: Option<&[&str]>,
        limit: i64,
    ) -> Result<Vec<ArchivalMemoryRow>, AppError> {
        let limit = limit.clamp(1, 200);
        let conn = self.conn()?;

        let mut sql = format!(
            "SELECT {} FROM archival_memory WHERE content LIKE ?",
            SELECT_COLUMNS
        );
        let pattern = format!("%{}%", query);

        if let Some(tags) = tags_filter {
            for tag in tags {
                sql.push_str(&format!(" AND tags_json LIKE '%{}%'", tag));
            }
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt.query_map(params![pattern, limit], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按向量搜索归档记忆
    pub fn search_archival_memory_by_vector(
        &self,
        query_vec: &[f32],
        embedding_model: &str,
        tags_filter: Option<&[&str]>,
        limit: i64,
    ) -> Result<Vec<ArchivalSearchResult>, AppError> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;

        let mut sql = format!(
            "SELECT {} FROM archival_memory WHERE embedding_model = ?",
            SELECT_COLUMNS
        );
        let params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(embedding_model.to_string())];

        if let Some(tags) = tags_filter {
            for tag in tags {
                sql.push_str(&format!(" AND tags_json LIKE '%{}%'", tag));
            }
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params.as_slice(), map_row_with_vector).map_err(db_err)?;

        let mut results: Vec<ArchivalSearchResult> = Vec::new();
        for row in rows {
            let r = row.map_err(db_err)?;
            if r.embedding_vec.is_empty() {
                continue;
            }
            let stored_vec = decode_vector(&r.embedding_vec);
            let score = cosine_similarity(query_vec, &stored_vec);

            results.push(ArchivalSearchResult {
                row: ArchivalMemoryRow {
                    id: r.id,
                    content: r.content,
                    tags_json: r.tags_json,
                    citation_json: r.citation_json,
                    embedding_model: r.embedding_model,
                    embedding_dim: r.embedding_dim,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                },
                score,
            });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);
        Ok(results)
    }

    /// 按标签列出归档记忆
    pub fn list_archival_memory_by_tags(
        &self,
        tags: &[&str],
        limit: i64,
    ) -> Result<Vec<ArchivalMemoryRow>, AppError> {
        let limit = limit.clamp(1, 200);
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn()?;
        let mut sql = format!("SELECT {} FROM archival_memory WHERE 1=1", SELECT_COLUMNS);
        for tag in tags {
            sql.push_str(&format!(" AND tags_json LIKE '%{}%'", tag));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut stmt = conn.prepare_cached(&sql).map_err(db_err)?;
        let rows = stmt.query_map(params![limit], map_row).map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按 ID 获取归档记忆
    pub fn get_archival_memory_by_id(&self, id: &str) -> Result<Option<ArchivalMemoryRow>, AppError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(&format!(
            "SELECT {} FROM archival_memory WHERE id = ?",
            SELECT_COLUMNS
        )).map_err(db_err)?;
        let mut rows = stmt.query_map(params![id], map_row).map_err(db_err)?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(db_err)?))
        } else {
            Ok(None)
        }
    }

    /// 删除归档记忆
    pub fn delete_archival_memory(&self, id: &str) -> Result<bool, AppError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM archival_memory WHERE id = ?",
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

    #[test]
    fn upsert_and_retrieve() {
        let db = make_in_memory_db();
        db.upsert_archival_memory(
            "id-1",
            "Important lesson about character development",
            r#"["lesson","character"]"#,
            Some(r#"{"source":"chapter-5"}"#),
            None,
            None,
            None,
        ).unwrap();

        let fetched = db.get_archival_memory_by_id("id-1").unwrap();
        assert!(fetched.is_some());
        let row = fetched.unwrap();
        assert_eq!(row.content, "Important lesson about character development");
        assert_eq!(row.tags_json, r#"["lesson","character"]"#);
    }

    #[test]
    fn search_by_content() {
        let db = make_in_memory_db();
        db.upsert_archival_memory("id-1", "Plot twist in chapter 3", r#"["plot"]"#, None, None, None, None).unwrap();
        db.upsert_archival_memory("id-2", "Character backstory", r#"["character"]"#, None, None, None, None).unwrap();

        let hits = db.search_archival_memory_by_content("chapter", None, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.contains("chapter"));

        let no_hits = db.search_archival_memory_by_content("nonexistent", None, 10).unwrap();
        assert!(no_hits.is_empty());
    }

    #[test]
    fn search_by_tags() {
        let db = make_in_memory_db();
        db.upsert_archival_memory("id-1", "Content 1", r#"["lesson","character"]"#, None, None, None, None).unwrap();
        db.upsert_archival_memory("id-2", "Content 2", r#"["plot"]"#, None, None, None, None).unwrap();

        let hits = db.list_archival_memory_by_tags(&["lesson"], 10).unwrap();
        assert_eq!(hits.len(), 1);

        let multi = db.list_archival_memory_by_tags(&["lesson", "character"], 10).unwrap();
        assert_eq!(multi.len(), 1);
    }

    #[test]
    fn search_by_vector() {
        let db = make_in_memory_db();
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.9, 0.1, 0.0];

        db.upsert_archival_memory(
            "id-1",
            "Content 1",
            r#"["tag1"]"#,
            None,
            Some(&vec1),
            Some("test-model"),
            Some(3),
        ).unwrap();
        db.upsert_archival_memory(
            "id-2",
            "Content 2",
            r#"["tag2"]"#,
            None,
            Some(&vec2),
            Some("test-model"),
            Some(3),
        ).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = db.search_archival_memory_by_vector(&query, "test-model", None, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].score > 0.9);
    }

    #[test]
    fn delete_removes_row() {
        let db = make_in_memory_db();
        db.upsert_archival_memory("id-1", "Content", r#"[]"#, None, None, None, None).unwrap();

        let deleted = db.delete_archival_memory("id-1").unwrap();
        assert!(deleted);

        let fetched = db.get_archival_memory_by_id("id-1").unwrap();
        assert!(fetched.is_none());
    }
}