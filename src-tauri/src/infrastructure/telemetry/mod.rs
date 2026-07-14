// Telemetry 模块 —— OpenTelemetry 风格的可观测性（traces + metrics）。
//
// 三件套:
// - traces:  Span 调用链（tracer.rs + db/stores/trace.rs）
// - metrics: counter/gauge/histogram 时间序列（meter.rs + db/stores/metric.rs）
// - logs:    已有 logs.sqlite + tracing（本模块不重复）
//
// W3C Trace Context（context.rs）支持跨进程传播，便于串联前后端调用链。
//
// 架构约束(AGENTS.md):
// - infrastructure 层只依赖 shared/，不依赖 core/agent/
// - IPC 命令（commands.rs）依赖 db::state::DbState，符合 IPC 边界约定

pub mod commands;
pub mod context;
pub mod meter;
pub mod tracer;

pub use context::{TraceContextPropagator, W3cTraceContext};
pub use meter::{Meter, MetricKind};
pub use tracer::{Span, SpanKind, SpanStatus, Tracer};
