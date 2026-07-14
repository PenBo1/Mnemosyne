-- Metric points 持久化表
--
-- OpenTelemetry 风格的 metric 存储，记录 counter / gauge / histogram 数据点。
-- 与 stats.rs（基于 messages 表聚合的 AI 指标）互补：本表支持任意业务自定义
-- metric 的实时记录与时间序列查询。
--
-- 设计:
-- - append-only:每个数据点一行，不做原地更新
-- - kind ∈ {"counter","gauge","histogram"}
-- - value 用 REAL，兼容整数与浮点
-- - attributes 存 JSON，支持多维度标签（如 model、provider、agent_role）
-- - timestamp 用 unix ms
-- - 聚合由查询层完成（aggregate_metrics 按 interval 桶聚合）

CREATE TABLE metric_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    value REAL NOT NULL,
    attributes TEXT NOT NULL DEFAULT '{}',
    timestamp INTEGER NOT NULL
);

CREATE INDEX idx_metric_points_name_time ON metric_points(name, timestamp DESC);
CREATE INDEX idx_metric_points_time ON metric_points(timestamp DESC);
