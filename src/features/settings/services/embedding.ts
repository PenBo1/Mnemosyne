import { ipc, ipcVoid } from "@/services/ipc";

/** Embedding 配置(与 Rust 端 EmbeddingConfig 对齐,camelCase) */
export interface EmbeddingConfig {
  enabled: boolean;
  baseUrl: string;
  apiKey: string;
  model: string;
  dim: number;
}

/** 语义搜索结果 */
export interface SearchResult {
  docType: string;
  docId: string;
  chunkIdx: number;
  content: string;
  score: number;
}

/** 向量索引统计 */
export interface VectorStats {
  total: number;
  byDocType: { docType: string; count: number }[];
}

/** 读取 embedding 配置 */
export async function embeddingGetConfig(): Promise<EmbeddingConfig> {
  return ipc<EmbeddingConfig>("embedding_get_config");
}

/** 保存 embedding 配置 */
export async function embeddingSetConfig(config: EmbeddingConfig): Promise<void> {
  await ipcVoid("embedding_set_config", { config });
}

/** 测试 embedding 连接,返回实际维度 */
export async function embeddingTest(): Promise<number> {
  return ipc<number>("embedding_test");
}

/** 向量索引统计 */
export async function embeddingStats(): Promise<VectorStats> {
  return ipc<VectorStats>("embedding_stats");
}
