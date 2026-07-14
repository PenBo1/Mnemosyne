// metric_points 表的 CRUD —— OpenTelemetry 风格的 metric 持久化。
//
// - append-only:每个数据点一行，不做原地更新
// - counter/gauge/histogram 统一存储，kind 字段区分
// - 聚合由查询层完成（aggregate_metrics 按 interval 桶聚合）
//
// 架构约束(AGENTS.md):
// - infrastructure 层只依赖 shared/，不依赖 core/agent/

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::super::connection::Database;
use super::super::connection::db_err;
use crate::shared::error::AppError;

/// Metric point 的 DB 行级表示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricPointRow {
    pub id: i64,
    pub name: String,
    /// "counter" / "gauge" / "histogram"
    pub kind: String,
    pub value: f64,
    /// JSON object string
    pub attributes: String,
    /// unix ms
    pub timestamp: i64,
}

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

impl Database {
    /// 写入一条 metric 数据点。
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

    /// 记录 counter（累加型指标，如 "agent.token_usage"）。
    pub fn record_counter(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "counter", value, attributes, timestamp)
    }

    /// 记录 gauge（瞬时值指标，如 "agent.active_count"）。
    pub fn record_gauge(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "gauge", value, attributes, timestamp)
    }

    /// 记录 histogram（分布型指标，如 "llm.latency_ms"）。
    pub fn record_histogram(
        &self,
        name: &str,
        value: f64,
        attributes: &str,
        timestamp: i64,
    ) -> Result<i64, AppError> {
        self.insert_metric_point(name, "histogram", value, attributes, timestamp)
    }

    /// 查询指定 metric 在时间范围内的所有数据点。
    /// from/to 为 unix ms，None 表示不限。limit 上限 5000。
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
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// 按时间桶聚合 metric（用于时间序列可视化）。
    ///
    /// interval_ms 为桶宽度（毫秒）。返回每个桶的 count / sum / min / max / avg。
    /// counter 类型按 sum 聚合，gauge 类型按 avg 聚合，histogram 按 count + avg。
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
        // 用 (timestamp - ?from) / interval 作为桶序号
        // from 未指定时用 MIN(timestamp) 作为基准
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
        // ?1 = base (from 或 0)
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
        rows.map(|r| Ok(r.map_err(db_err)?)).collect()
    }

    /// GC:删除 timestamp 早于 cutoff_ms 的 metric 数据点。
    /// 返回删除的行数。
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

    /// Metric 聚合统计:总数 / distinct name 数。
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

/// Metric 时间桶聚合结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricBucket {
    pub bucket_start: i64,
    pub bucket_end: i64,
    pub count: u32,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

/// Metric 聚合统计。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricStats {
    pub total_points: i64,
    pub distinct_names: i64,
}

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

        // 桶宽 1000ms，base=0
        let buckets = db.aggregate_metrics("m", Some(0), None, 1000).unwrap();
        assert_eq!(buckets.len(), 3);
        // 桶 0 (0-1000): 1 point (10.0)
        assert_eq!(buckets[0].count, 1);
        assert_eq!(buckets[0].sum, 10.0);
        // 桶 1 (1000-2000): 2 points (20.0, 30.0 is 2100 -> bucket 2)
        // wait: 1500 is bucket 1, 2100 is bucket 2, 2800 is bucket 2
        assert_eq!(buckets[1].count, 1);
        assert_eq!(buckets[1].sum, 20.0);
        assert_eq!(buckets[2].count, 2);
        assert_eq!(buckets[2].sum, 70.0);
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
