// 向量存储:vectors 表的读写与余弦相似度搜索。
//
// 向量以 BLOB(f32 小端序)存储,搜索时全量加载到 Rust 端计算余弦相似度。
// 桌面应用规模(数千块)下性能足够,无需引入向量数据库依赖。

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;
use crate::infrastructure::llm::embedding::types::{SearchResult, VectorStats, DocTypeCount};

/// f32 向量序列化为小端字节 BLOB
pub fn encode_vector(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for f in vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// BLOB 反序列化为 f32 向量
pub fn decode_vector(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4)
        .map(|c| {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(c);
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// 余弦相似度
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

/// 内部行结构(含向量字节)
struct VectorRowInner {
    doc_type: String,
    doc_id: String,
    chunk_idx: i64,
    content: String,
    embedding: Vec<u8>,
}

impl Database {
    /// 批量插入向量(先按 doc_type+doc_id 删除旧记录,再插入新记录)
    pub fn upsert_vectors(
        &self,
        workspace_id: Option<&str>,
        doc_type: &str,
        doc_id: &str,
        chunks: &[(String, Vec<f32>)], // (content, embedding)
        model: &str,
        dim: usize,
    ) -> Result<usize, AppError> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        // 先删除该文档的旧向量
        conn.execute(
            "DELETE FROM vectors WHERE doc_type = ?1 AND doc_id = ?2",
            rusqlite::params![doc_type, doc_id],
        ).map_err(db_err)?;

        let mut count = 0usize;
        for (idx, (content, embedding)) in chunks.iter().enumerate() {
            if embedding.len() != dim {
                return Err(AppError::invalid_format(format!(
                    "Vector dim mismatch at chunk {}: expected {}, got {}", idx, dim, embedding.len()
                )));
            }
            let id = uuid::Uuid::now_v7().to_string();
            let blob = encode_vector(embedding);
            conn.execute(
                "INSERT INTO vectors (id, workspace_id, doc_type, doc_id, chunk_idx, content, embedding, dim, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    workspace_id,
                    doc_type,
                    doc_id,
                    idx as i64,
                    content,
                    blob,
                    dim as i64,
                    model,
                    now,
                ],
            ).map_err(db_err)?;
            count += 1;
        }
        tracing::info!(doc_type, doc_id, count, "Vectors upserted");
        Ok(count)
    }

    /// 删除指定文档的全部向量
    pub fn delete_vectors_by_doc(&self, doc_type: &str, doc_id: &str) -> Result<usize, AppError> {
        let conn = self.conn()?;
        let n = conn.execute(
            "DELETE FROM vectors WHERE doc_type = ?1 AND doc_id = ?2",
            rusqlite::params![doc_type, doc_id],
        ).map_err(db_err)?;
        tracing::info!(doc_type, doc_id, deleted = n, "Vectors deleted");
        Ok(n)
    }

    /// 语义搜索:加载同 workspace+model 的向量,计算余弦相似度,返回 top-N
    pub fn search_similar(
        &self,
        workspace_id: Option<&str>,
        query_vec: &[f32],
        model: &str,
        doc_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<SearchResult>, AppError> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;

        let mut sql = String::from(
            "SELECT doc_type, doc_id, chunk_idx, content, embedding FROM vectors WHERE model = ?1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(model.to_string())];
        let mut param_idx = 2;

        if let Some(ws) = workspace_id {
            sql.push_str(&format!(" AND workspace_id = ?{}", param_idx));
            params.push(Box::new(ws.to_string()));
            param_idx += 1;
        }
        if let Some(dt) = doc_type {
            sql.push_str(&format!(" AND doc_type = ?{}", param_idx));
            params.push(Box::new(dt.to_string()));
        }

        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let emb_bytes: Vec<u8> = row.get(4)?;
            Ok(VectorRowInner {
                doc_type: row.get(0)?,
                doc_id: row.get(1)?,
                chunk_idx: row.get(2)?,
                content: row.get(3)?,
                embedding: emb_bytes,
            })
        }).map_err(db_err)?;

        let mut results: Vec<SearchResult> = Vec::new();
        for row in rows {
            let r = row.map_err(db_err)?;
            let emb = decode_vector(&r.embedding);
            let score = cosine_similarity(query_vec, &emb);
            results.push(SearchResult {
                doc_type: r.doc_type,
                doc_id: r.doc_id,
                chunk_idx: r.chunk_idx,
                content: r.content,
                score,
            });
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);
        Ok(results)
    }

    /// 向量索引统计
    pub fn vector_stats(&self) -> Result<VectorStats, AppError> {
        let conn = self.conn()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM vectors", [], |row| row.get(0))
            .map_err(db_err)?;

        let mut stmt = conn.prepare_cached(
            "SELECT doc_type, COUNT(*) FROM vectors GROUP BY doc_type ORDER BY COUNT(*) DESC",
        ).map_err(db_err)?;
        let rows = stmt.query_map([], |row| {
            Ok(DocTypeCount {
                doc_type: row.get(0)?,
                count: row.get(1)?,
            })
        }).map_err(db_err)?;
        let by_doc_type = rows.collect::<Result<Vec<_>, _>>().map_err(db_err)?;

        Ok(VectorStats { total, by_doc_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let vec = vec![0.1, -0.2, 0.3, 1.0, -1.5];
        let bytes = encode_vector(&vec);
        let decoded = decode_vector(&bytes);
        assert_eq!(vec.len(), decoded.len());
        for (a, b) in vec.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn cosine_similarity_basic() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
    }

    #[test]
    fn cosine_empty_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_mismatched_len_returns_zero() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 1.0]), 0.0);
    }
}
