//! ═══════════════════════════════════════════════════════════════════════════
//! 指标存储 - OpenTelemetry 风格的 metric 持久化
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 设计要点：
//! - 追加写入（append-only），每个数据点一行
//! - counter/gauge/histogram 统一存储，kind 字段区分
//! - 聚合由查询层完成

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

// ── 数据类型 ────────────────────────────────────────────────────────────────

/// 指标数据点行
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricPointRow {
    /// 数据点 ID
    pub id: i64,
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub kind: String,
    /// 数值
    pub value: f64,
    /// 属性 JSON
    pub attributes: String,
    /// 时间戳（Unix 毫秒）
    pub timestamp: i64,
}

/// 指标时间桶聚合结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricBucket {
    /// 桶起始时间
    pub bucket_start: i64,
    /// 桶结束时间
    pub bucket_end: i64,
    /// 数据点数
    pub count: u32,
    /// 总和
    pub sum: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 平均值
    pub avg: f64,
}

/// 指标统计
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricStats {
    /// 总数据点数
    pub total_points: i64,
    /// 不同指标名数
    pub distinct_names: i64,
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 映射数据库行到指标数据点
fn map_metric_row(row: &rusqlite::Row) -> rusqlite::Result<MetricPointRow> {
    Ok(MetricPointRow {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        value: row.get(3)?,
        attributes: row.get(4)?,
        timestamp: row.get(5)?,
    })
}

const METRIC_SELECT_COLUMNS: &str = "id, name, kind, value, attributes, timestamp";

// ── 数据库操作 ──────────────────────────────────────────────────────────────

impl Database {
    /// 写入一条 metric 数据点
    pub fn insert_metric_point(
        &self,
        name: &str,
        kind: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO metric_points (name, kind, value, attributes, timestamp) VALUES (?, ?, ?, ?, ?)",
            params![name, kind, value, attributes, timestamp],
        )
        .map_err(db_err)?;
        Ok(conn.last_insert_rowid())
    }

    /// 记录 counter 类型指标
    pub fn record_counter(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "counter", value, attributes, timestamp)
    }

    /// 记录 gauge 类型指标
    pub fn record_gauge(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "gauge", value, attributes, timestamp)
    }

    /// 记录 histogram 类型指标
    pub fn record_histogram(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "histogram", value, attributes, timestamp)
    }

    /// 查询指定 metric 在时间范围内的所有数据点
    pub fn query_metrics(
        &self,
        name: &str,
        from: Option<i64>,
        to: Option<i64>,
        limit: i64,
    ) -> Result<Vec<MetricPointRow>, AppError> {
        let limit = limit.clamp(1, 5000);
        let conn = self.conn()?;
        let mut sql = format!(
            "SELECT {} FROM metric_points WHERE name = ?1",
            METRIC_SELECT_COLUMNS
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(name.to_string())];
        let mut idx = 2usize;
        if let Some(f) = from {
            sql.push_str(&format!(" AND timestamp >= ?{}", idx));
            params_vec.push(Box::new(f));
            idx += 1;
        }
        if let Some(t) = to {
            sql.push_str(&format!(" AND timestamp <= ?{}", idx));
            params_vec.push(Box::new(t));
            idx += 1;
        }
        sql.push_str(&format!(" ORDER BY timestamp ASC LIMIT ?{}", idx));
        params_vec.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), map_metric_row)
            .map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 按时间桶聚合 metric
    pub fn aggregate_metrics(
        &self,
        name: &str,
        from: Option<i64>,
        to: Option<i64>,
        interval_ms: i64,
    ) -> Result<Vec<MetricBucket>, AppError> {
        if interval_ms <= 0 {
            return Err(AppError::bad_request(format!(
                "interval_ms must be positive, got {}",
                interval_ms
            )));
        }
        let conn = self.conn()?;
        let mut sql = String::from(
            "SELECT \
                (timestamp - ?1) / ?2 AS bucket_idx, \
                MIN(timestamp) AS bucket_start, \
                MAX(timestamp) AS bucket_end, \
                COUNT(*) AS cnt, \
                SUM(value) AS sum_v, \
                MIN(value) AS min_v, \
                MAX(value) AS max_v, \
                AVG(value) AS avg_v \
             FROM metric_points WHERE name = ?3",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let base = from.unwrap_or(0);
        params_vec.push(Box::new(base));
        params_vec.push(Box::new(interval_ms));
        params_vec.push(Box::new(name.to_string()));
        let mut idx = 4usize;
        if let Some(f) = from {
            sql.push_str(&format!(" AND timestamp >= ?{}", idx));
            params_vec.push(Box::new(f));
            idx += 1;
        }
        if let Some(t) = to {
            sql.push_str(&format!(" AND timestamp <= ?{}", idx));
            params_vec.push(Box::new(t));
        }
        sql.push_str(" GROUP BY bucket_idx ORDER BY bucket_idx ASC");

        let mut stmt = conn.prepare(&sql).map_err(db_err)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(MetricBucket {
                    bucket_start: row.get(1)?,
                    bucket_end: row.get(2)?,
                    count: row.get::<_, i64>(3)? as u32,
                    sum: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                    min: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                    max: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                    avg: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                })
            })
            .map_err(db_err)?;
        rows.map(|r| r.map_err(db_err)).collect()
    }

    /// 删除指定时间之前的 metric 数据点
    pub fn delete_metrics_before(&self, cutoff_ms: i64) -> Result<u64, AppError> {
        let conn = self.conn()?;
        let affected = conn
            .execute(
                "DELETE FROM metric_points WHERE timestamp < ?1",
                params![cutoff_ms],
            )
            .map_err(db_err)?;
        Ok(affected as u64)
    }

    /// 获取指标统计
    pub fn metric_stats(&self) -> Result<MetricStats, AppError> {
        let conn = self.conn()?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM metric_points", [], |row| row.get(0))
            .map_err(db_err)?;
        let distinct_names: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT name) FROM metric_points",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        Ok(MetricStats {
            total_points: total,
            distinct_names,
        })
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
    fn record_and_query_counter() {
        let db = make_in_memory_db();
        db.record_counter("agent.token_usage", 100.0, "{}", 1000).unwrap();
        db.record_counter("agent.token_usage", 200.0, "{}", 2000).unwrap();
        db.record_counter("agent.token_usage", 300.0, "{}", 3000).unwrap();

        let points = db.query_metrics("agent.token_usage", None, None, 100).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].value, 100.0);
        assert_eq!(points[2].value, 300.0);
        assert_eq!(points[0].kind, "counter");
    }

    #[test]
    fn query_metrics_with_time_range() {
        let db = make_in_memory_db();
        db.record_counter("m", 1.0, "{}", 1000).unwrap();
        db.record_counter("m", 2.0, "{}", 2000).unwrap();
        db.record_counter("m", 3.0, "{}", 3000).unwrap();

        let points = db.query_metrics("m", Some(1500), Some(2500), 100).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 2.0);
    }

    #[test]
    fn aggregate_metrics_buckets_correctly() {
        let db = make_in_memory_db();
        db.record_counter("m", 10.0, "{}", 1000).unwrap();
        db.record_counter("m", 20.0, "{}", 1500).unwrap();
        db.record_counter("m", 30.0, "{}", 2100).unwrap();
        db.record_counter("m", 40.0, "{}", 2800).unwrap();

        let buckets = db.aggregate_metrics("m", Some(0), None, 1000).unwrap();
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].count, 2);
        assert_eq!(buckets[0].sum, 30.0);
        assert_eq!(buckets[1].count, 2);
        assert_eq!(buckets[1].sum, 70.0);
    }

    #[test]
    fn delete_before_gc_old_metrics() {
        let db = make_in_memory_db();
        db.record_counter("m", 1.0, "{}", 1000).unwrap();
        db.record_counter("m", 2.0, "{}", 2000).unwrap();

        let deleted = db.delete_metrics_before(1500).unwrap();
        assert_eq!(deleted, 1);

        let remaining = db.query_metrics("m", None, None, 100).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].value, 2.0);
    }

    #[test]
    fn metric_stats_counts() {
        let db = make_in_memory_db();
        db.record_counter("a", 1.0, "{}", 1000).unwrap();
        db.record_counter("a", 2.0, "{}", 2000).unwrap();
        db.record_gauge("b", 3.0, "{}", 3000).unwrap();

        let stats = db.metric_stats().unwrap();
        assert_eq!(stats.total_points, 3);
        assert_eq!(stats.distinct_names, 2);
    }
}