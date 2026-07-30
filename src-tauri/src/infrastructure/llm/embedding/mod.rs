//! ═══════════════════════════════════════════════════════════════════════════
//! 向量模型 - Embedding 模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 本地与云端统一走 OpenAI 兼容 /v1/embeddings 协议:
//! - 本地: Ollama OpenAI 兼容端点 (http://localhost:11434/v1) 或 LM Studio
//! - 云端: OpenAI / 任意 OpenAI 兼容服务
//!
//! 向量以 BLOB(f32 小端序)存入 SQLite，余弦相似度在 Rust 端计算。
//!
//! ## 分层架构
//!
//! - `commands.rs`: IPC 命令入口，仅参数验证和委托
//! - `service.rs`: 业务逻辑层，索引、搜索、摄入逻辑
//! - `config.rs`: 配置读写服务
//! - `types.rs`: 类型定义和常量
//! - `client.rs`: HTTP 客户端，OpenAI API 调用
//! - `chunker.rs`: 文本分块
//! - `ingest.rs`: 文档解析，PDF/HTML/EPUB 提取

pub mod types;
pub mod client;
pub mod chunker;
pub mod ingest;
pub mod config;
pub mod service;
pub mod commands;

// 公开类型导出
pub use types::{
    EmbeddingConfig, IndexDocParams, SearchParams, IngestFileParams,
    SearchResult, VectorStats, IngestResult,
};
