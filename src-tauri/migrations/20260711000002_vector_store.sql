-- 向量存储表
--
-- 支撑 RAG / 语义搜索 / 章节检索。本地与云端 embedding 统一走 OpenAI 兼容
-- /v1/embeddings 协议,向量以 BLOB(f32 小端序)存储,余弦相似度在 Rust 端计算。
--
-- 设计:
-- - doc_type + doc_id 定位来源(chapter/wiki/message),chunk_idx 标记切分序号
-- - embedding 存原始 f32 字节序列,dim 记录维度(不同模型维度不同)
-- - model 字段防止混用不同 embedding 模型时维度不一致导致搜索错误
-- - workspace_id 隔离工作空间,与现有业务表语义一致

CREATE TABLE vectors (
    id TEXT PRIMARY KEY,
    workspace_id TEXT,
    doc_type TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    chunk_idx INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    embedding BLOB NOT NULL,
    dim INTEGER NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_vectors_doc ON vectors(doc_type, doc_id);
CREATE INDEX idx_vectors_workspace ON vectors(workspace_id, model);
