-- Trace spans 持久化表
--
-- OpenTelemetry 风格的 span 存储，记录 agent / LLM / tool / pipeline 各阶段
-- 的执行轨迹。与 audit_events（安全审计）正交：本表关注性能与调用链，
-- audit_events 关注安全决策。
--
-- 设计:
-- - id 为 span ID（W3C span_id, 16 hex）
-- - trace_id 关联同一调用链（W3C trace_id, 32 hex）
-- - parent_span_id 构成 span 树（NULL = root span）
-- - attributes / events 存 JSON 字符串，保留 OTel 语义
-- - start_time / end_time 用 unix ms（INTEGER），便于时间差计算
-- - status ∈ {"ok","error","unset"}，对齐 OTel SpanStatus
-- - workspace_id / session_id 用于按业务维度过滤

CREATE TABLE trace_spans (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    parent_span_id TEXT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'internal',
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    attributes TEXT NOT NULL DEFAULT '{}',
    events TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'unset',
    status_message TEXT,
    workspace_id TEXT,
    session_id TEXT
);

CREATE INDEX idx_trace_spans_trace ON trace_spans(trace_id, start_time ASC);
CREATE INDEX idx_trace_spans_name ON trace_spans(name, start_time DESC);
CREATE INDEX idx_trace_spans_start ON trace_spans(start_time DESC);
CREATE INDEX idx_trace_spans_workspace ON trace_spans(workspace_id, start_time DESC);
