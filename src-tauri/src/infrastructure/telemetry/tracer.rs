// Tracer —— OpenTelemetry 风格的 span 创建与管理。
//
// 设计:
// - Tracer 持有 Database（Clone 廉价，Arc<Connection>），用于 span 持久化
// - Span 实现 RAII Drop，自动记录 end_time 并写入 DB
// - 支持父子 span（通过 parent_span_id 串联）
// - 默认 trace_id 自动生成；若提供 parent context 则继承其 trace_id
//
// 与 W3C Trace Context 的关系:
// - context.rs 负责跨进程传播（解析/生成 traceparent）
// - 本模块负责进程内 span 的生命周期管理
//
// 错误处理:
// - span 写入失败时通过 tracing::warn! 记录，不传播错误
//   （observability 失败不应阻断业务流程；这是显式日志，非 silent fallback）

use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::stores::trace::SpanRow;

use super::context::W3cTraceContext;

/// Span 类型，对齐 OpenTelemetry SpanKind。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Internal,
    Client,
    Server,
    Producer,
    Consumer,
}

impl SpanKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::Internal => "internal",
            SpanKind::Client => "client",
            SpanKind::Server => "server",
            SpanKind::Producer => "producer",
            SpanKind::Consumer => "consumer",
        }
    }
}

/// Span 状态，对齐 OpenTelemetry SpanStatus。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

impl SpanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanStatus::Unset => "unset",
            SpanStatus::Ok => "ok",
            SpanStatus::Error => "error",
        }
    }
}

/// Span 事件（对齐 OTel Event）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp_ms: i64,
    pub attributes: serde_json::Value,
}

/// Tracer —— 创建 span 的入口。
///
/// 持有 Database 副本，每个 span 在 Drop 时写入 DB。
/// Tracer 本身无状态，可自由创建。
#[derive(Clone)]
pub struct Tracer {
    db: Database,
}

impl Tracer {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 创建一个 root span（无父 span，自动生成 trace_id）。
    ///
    /// name 示例: "agent.send_message", "llm.chat", "tool.read_file", "pipeline.run"
    pub fn start_span(&self, name: &str, kind: SpanKind) -> Span {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();
        self.create_span(
            span_id,
            trace_id,
            None,
            name,
            kind,
            None,
            None,
        )
    }

    /// 创建一个子 span（继承父 span 的 trace_id）。
    pub fn start_child_span(
        &self,
        parent: &Span,
        name: &str,
        kind: SpanKind,
    ) -> Span {
        self.create_span(
            generate_span_id(),
            parent.trace_id.clone(),
            Some(parent.id.clone()),
            name,
            kind,
            parent.workspace_id.clone(),
            parent.session_id.clone(),
        )
    }

    /// 从 W3C trace context 创建子 span（用于跨进程传播后的接续）。
    pub fn start_span_from_context(
        &self,
        ctx: &W3cTraceContext,
        name: &str,
        kind: SpanKind,
    ) -> Span {
        self.create_span(
            generate_span_id(),
            ctx.trace_id.clone(),
            Some(ctx.parent_span_id.clone()),
            name,
            kind,
            None,
            None,
        )
    }

    fn create_span(
        &self,
        span_id: String,
        trace_id: String,
        parent_span_id: Option<String>,
        name: &str,
        kind: SpanKind,
        workspace_id: Option<String>,
        session_id: Option<String>,
    ) -> Span {
        let now_ms = chrono::Utc::now().timestamp_millis();
        Span {
            id: span_id,
            trace_id,
            parent_span_id,
            name: name.to_string(),
            kind,
            start_time: now_ms,
            end_time: None,
            attributes: serde_json::Map::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
            status_message: None,
            workspace_id,
            session_id,
            db: Some(self.db.clone()),
            finished: false,
        }
    }
}

/// Span —— 一次操作的执行轨迹片段。
///
/// RAII 语义:Drop 时自动记录 end_time 并写入 TraceStore。
/// 若已显式调用 finish()，Drop 不会重复写入。
pub struct Span {
    pub id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: SpanKind,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub attributes: serde_json::Map<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
    pub status: SpanStatus,
    pub status_message: Option<String>,
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
    /// Some(db) = 未写入，Drop 时写入；None = 已写入或不可持久化
    db: Option<Database>,
    /// 防止重复 finish 的标志
    finished: bool,
}

impl Span {
    /// 设置单个属性。
    pub fn set_attribute<K: Into<String>, V: Into<serde_json::Value>>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 批量设置属性（合并到现有属性）。
    pub fn set_attributes(&mut self, attrs: serde_json::Map<String, serde_json::Value>) -> &mut Self {
        for (k, v) in attrs {
            self.attributes.insert(k, v);
        }
        self
    }

    /// 添加事件。
    pub fn add_event<N: Into<String>>(&mut self, name: N) -> &mut Self {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            attributes: serde_json::Value::Null,
        });
        self
    }

    /// 添加带属性的事件。
    pub fn add_event_with_attrs<N: Into<String>>(
        &mut self,
        name: N,
        attrs: serde_json::Value,
    ) -> &mut Self {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            attributes: attrs,
        });
        self
    }

    /// 设置状态。
    pub fn set_status(&mut self, status: SpanStatus) -> &mut Self {
        self.status = status;
        self
    }

    /// 设置错误状态（带消息）。
    pub fn set_error<M: Into<String>>(&mut self, message: M) -> &mut Self {
        self.status = SpanStatus::Error;
        self.status_message = Some(message.into());
        self
    }

    /// 设置 workspace 关联。
    pub fn set_workspace<W: Into<String>>(&mut self, workspace_id: W) -> &mut Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    /// 设置 session 关联。
    pub fn set_session<S: Into<String>>(&mut self, session_id: S) -> &mut Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 显式结束 span 并写入 DB。
    ///
    /// 调用后 Drop 不会重复写入。重复调用 finish() 是 no-op。
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.end_time = Some(chrono::Utc::now().timestamp_millis());
        self.persist();
    }

    fn persist(&mut self) {
        let db = match self.db.take() {
            Some(d) => d,
            None => return,
        };
        let row = SpanRow {
            id: self.id.clone(),
            trace_id: self.trace_id.clone(),
            parent_span_id: self.parent_span_id.clone(),
            name: self.name.clone(),
            kind: self.kind.as_str().to_string(),
            start_time: self.start_time,
            end_time: self.end_time,
            attributes: serde_json::Value::Object(self.attributes.clone()).to_string(),
            events: serde_json::to_value(&self.events).unwrap_or_else(|_| serde_json::Value::Array(vec![])).to_string(),
            status: self.status.as_str().to_string(),
            status_message: self.status_message.clone(),
            workspace_id: self.workspace_id.clone(),
            session_id: self.session_id.clone(),
        };
        if let Err(e) = db.insert_span(&row) {
            // observability 失败不阻断业务，但显式记录（非 silent fallback）
            tracing::warn!(
                span_id = %self.id,
                span_name = %self.name,
                error = %e,
                "Failed to persist trace span"
            );
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if !self.finished {
            self.end_time = Some(chrono::Utc::now().timestamp_millis());
            self.persist();
        }
    }
}

/// 生成 W3C trace_id（32 hex chars）。
fn generate_trace_id() -> String {
    // 用 16 字节随机数 hex 编码 = 32 chars
    let bytes = generate_random_bytes(16);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 生成 W3C span_id（16 hex chars）。
fn generate_span_id() -> String {
    // 用 8 字节随机数 hex 编码 = 16 chars
    let bytes = generate_random_bytes(8);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 生成随机字节（使用 std::time + thread_id 作为简易熵源，避免引入 rand 依赖）。
///
/// 注意:这不满足密码学强度，但满足 trace_id/span_id 的唯一性需求。
/// 若未来需要更强唯一性保证，可引入 uuid crate。
fn generate_random_bytes(len: usize) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut result = Vec::with_capacity(len);
    let mut counter = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    for i in 0..len {
        let mut hasher = DefaultHasher::new();
        counter = counter.wrapping_add(0x9E3779B97F4A7C15);
        counter.hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        i.hash(&mut hasher);
        result.push((hasher.finish() & 0xFF) as u8);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tracer() -> Tracer {
        let db = Database::connect_in_memory().expect("in-memory db");
        Tracer::new(db)
    }

    #[test]
    fn start_span_persists_on_drop() {
        let tracer = make_tracer();
        {
            let _span = tracer.start_span("test.op", SpanKind::Internal);
            // span drops here
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].name, "test.op");
        assert!(spans[0].end_time.is_some());
    }

    #[test]
    fn finish_writes_span_with_attributes() {
        let tracer = make_tracer();
        {
            let mut span = tracer.start_span("test.op", SpanKind::Internal);
            span.set_attribute("key", "value");
            span.set_status(SpanStatus::Ok);
            span.finish();
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        assert_eq!(spans.len(), 1);
        let attrs: serde_json::Value = serde_json::from_str(&spans[0].attributes).unwrap();
        assert_eq!(attrs["key"], "value");
        assert_eq!(spans[0].status, "ok");
    }

    #[test]
    fn child_span_inherits_trace_id() {
        let tracer = make_tracer();
        let parent = tracer.start_span("parent", SpanKind::Internal);
        let parent_trace = parent.trace_id.clone();
        let parent_id = parent.id.clone();
        {
            let mut parent = parent;
            let mut child = tracer.start_child_span(&parent, "child", SpanKind::Internal);
            child.finish();
            parent.finish();
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        assert_eq!(spans.len(), 2);
        let child = spans.iter().find(|s| s.name == "child").unwrap();
        assert_eq!(child.trace_id, parent_trace);
        assert_eq!(child.parent_span_id, Some(parent_id));
    }

    #[test]
    fn error_status_with_message_persists() {
        let tracer = make_tracer();
        {
            let mut span = tracer.start_span("fail.op", SpanKind::Client);
            span.set_error("connection refused");
            span.finish();
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        assert_eq!(spans[0].status, "error");
        assert_eq!(spans[0].status_message, Some("connection refused".to_string()));
    }

    #[test]
    fn double_finish_is_noop() {
        let tracer = make_tracer();
        {
            let mut span = tracer.start_span("test", SpanKind::Internal);
            span.finish();
            span.finish(); // no-op
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn events_persist_as_json_array() {
        let tracer = make_tracer();
        {
            let mut span = tracer.start_span("test", SpanKind::Internal);
            span.add_event("started");
            span.add_event_with_attrs("progress", serde_json::json!({"step": 1}));
            span.finish();
        }
        let spans = tracer.db.list_recent_spans(10).unwrap();
        let events: serde_json::Value = serde_json::from_str(&spans[0].events).unwrap();
        assert_eq!(events.as_array().unwrap().len(), 2);
        assert_eq!(events[0]["name"], "started");
        assert_eq!(events[1]["attributes"]["step"], 1);
    }
}
