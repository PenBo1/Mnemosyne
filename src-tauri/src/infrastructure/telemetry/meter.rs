// Meter —— OpenTelemetry 风格的 metric 记录。
//
// 设计:
// - Meter 持有 Database（Clone 廉价），用于 metric 持久化
// - counter/gauge/histogram 统一存储到 metric_points 表，kind 字段区分
// - 每次调用立即写入 DB（append-only），不做内存缓冲
//
// 与 stats.rs 的关系:
// - stats.rs 从 messages 表聚合 AI 指标（被动统计）
// - Meter 提供主动记录任意业务 metric 的能力（主动埋点）

use crate::infrastructure::db::connection::Database;

/// Metric 类型，对齐 OpenTelemetry InstrumentKind。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

impl MetricKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricKind::Counter => "counter",
            MetricKind::Gauge => "gauge",
            MetricKind::Histogram => "histogram",
        }
    }
}

/// Meter —— metric 记录入口。
#[derive(Clone)]
pub struct Meter {
    db: Database,
}

impl Meter {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 记录 counter 值（累加型，如 token 用量）。
    pub fn record_counter(
        &self,
        name: &str,
        value: f64,
        attributes: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<i64, crate::shared::error::AppError> {
        let attrs = serde_json::Value::Object(attributes.clone()).to_string();
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.db.record_counter(name, value, &attrs, now_ms)
    }

    /// 记录 gauge 值（瞬时值，如活跃 agent 数）。
    pub fn record_gauge(
        &self,
        name: &str,
        value: f64,
        attributes: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<i64, crate::shared::error::AppError> {
        let attrs = serde_json::Value::Object(attributes.clone()).to_string();
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.db.record_gauge(name, value, &attrs, now_ms)
    }

    /// 记录 histogram 值（分布型，如 LLM 延迟）。
    pub fn record_histogram(
        &self,
        name: &str,
        value: f64,
        attributes: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<i64, crate::shared::error::AppError> {
        let attrs = serde_json::Value::Object(attributes.clone()).to_string();
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.db.record_histogram(name, value, &attrs, now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meter() -> Meter {
        let db = Database::connect_in_memory().expect("in-memory db");
        Meter::new(db)
    }

    #[test]
    fn record_counter_persists() {
        let meter = make_meter();
        let mut attrs = serde_json::Map::new();
        attrs.insert("model".to_string(), serde_json::json!("gpt-4"));
        meter.record_counter("agent.token_usage", 100.0, &attrs).unwrap();

        let points = meter.db.query_metrics("agent.token_usage", None, None, 10).unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 100.0);
        assert_eq!(points[0].kind, "counter");
        let parsed: serde_json::Value = serde_json::from_str(&points[0].attributes).unwrap();
        assert_eq!(parsed["model"], "gpt-4");
    }

    #[test]
    fn record_gauge_and_histogram_distinguish_kind() {
        let meter = make_meter();
        let attrs = serde_json::Map::new();
        meter.record_gauge("agent.active", 3.0, &attrs).unwrap();
        meter.record_histogram("llm.latency", 150.0, &attrs).unwrap();

        let gauges = meter.db.query_metrics("agent.active", None, None, 10).unwrap();
        assert_eq!(gauges[0].kind, "gauge");
        let hists = meter.db.query_metrics("llm.latency", None, None, 10).unwrap();
        assert_eq!(hists[0].kind, "histogram");
    }
}
