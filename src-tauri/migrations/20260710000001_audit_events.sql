-- 审计事件持久化表
--
-- SecurityKernel 审计事件原本只在内存 AuditStore 中保留(10000 条上限、7 天 TTL),
-- 重启即丢失。本表将审计事件落盘,供仪表盘 Violations 统计与审计日志页面查询。
--
-- 设计:
-- - payload 存储完整 SecurityEvent JSON,保证事件全字段可还原
-- - 提取 event_type / operation / workspace_id / is_denied / is_security_related
--   作为索引列,支持聚合统计与过滤查询
-- - recorded_at 与 AuditEntry.recorded_at 对齐(UTC ISO8601)

CREATE TABLE audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    operation TEXT,
    workspace_id TEXT,
    is_denied INTEGER NOT NULL DEFAULT 0 CHECK(is_denied IN (0, 1)),
    is_security_related INTEGER NOT NULL DEFAULT 0 CHECK(is_security_related IN (0, 1)),
    payload TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE INDEX idx_audit_events_type ON audit_events(event_type, recorded_at DESC);
CREATE INDEX idx_audit_events_workspace ON audit_events(workspace_id, recorded_at DESC);
CREATE INDEX idx_audit_events_recorded ON audit_events(recorded_at DESC);
CREATE INDEX idx_audit_events_denied ON audit_events(is_denied, recorded_at DESC);
