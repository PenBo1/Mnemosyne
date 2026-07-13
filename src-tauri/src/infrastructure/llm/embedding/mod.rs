// 向量模型 (Embedding Model) 模块
//
// 本地与云端统一走 OpenAI 兼容 /v1/embeddings 协议:
// - 本地: Ollama OpenAI 兼容端点 (http://localhost:11434/v1) 或 LM Studio
// - 云端: OpenAI / 任意 OpenAI 兼容服务
//
// 向量以 BLOB(f32 小端序)存入 SQLite,余弦相似度在 Rust 端计算,零新依赖。

pub mod types;
pub mod client;
pub mod chunker;
pub mod commands;
