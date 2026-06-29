use serde::{Deserialize, Serialize};

/// 带元数据的 RAG 文本 chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub content: String,
    pub source: String,
    pub chunk_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chapter: Option<u32>,
    pub section: Option<String>,
    pub tokens: Option<u32>,
}

/// 文本分块的配置
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub max_chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub min_chunk_tokens: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_tokens: 512,
            overlap_tokens: 64,
            min_chunk_tokens: 50,
        }
    }
}

/// 估算 token 数（与 token_budget 相同）
fn estimate_tokens(text: &str) -> usize {
    let mut count = 0usize;
    for ch in text.chars() {
        if ch.is_ascii() { count += 1; } else { count += 2; }
    }
    (count + 3) / 4
}

/// 将文本切分为带重叠的 chunk
pub fn chunk_text(text: &str, source: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < chars.len() {
        // 基于 token 估算查找结束位置
        let mut end = start;
        let mut tokens = 0;
        while end < chars.len() && tokens < config.max_chunk_tokens {
            let ch = chars[end];
            if ch.is_ascii() { tokens += 1; } else { tokens += 2; }
            end += 1;
        }

        let content: String = chars[start..end].iter().collect();
        let content_tokens = estimate_tokens(&content);

        if content_tokens >= config.min_chunk_tokens {
            chunks.push(Chunk {
                id: format!("{}_{}", source, chunk_index),
                content: content.clone(),
                source: source.to_string(),
                chunk_index,
                start_offset: start,
                end_offset: end,
                metadata: ChunkMetadata {
                    tokens: Some(content_tokens as u32),
                    ..Default::default()
                },
            });
            chunk_index += 1;
        }

        // 带 overlap 向前推进 start
        let advance = end - start;
        let overlap_chars = config.overlap_tokens * 4; // 粗略字符估算
        start = if advance > overlap_chars { end - overlap_chars } else { end };
    }

    chunks
}

/// chunk 的 TF-IDF 风格 embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub chunk_id: String,
    pub vector: Vec<f64>,
}

/// 简单 TF-IDF embedding（生产环境应使用 neural embedding）
pub fn embed_chunk(chunk: &Chunk, vocabulary: &[String]) -> Embedding {
    let content_lower = chunk.content.to_lowercase();
    let words: Vec<&str> = content_lower.split_whitespace().collect();
    let total_words = words.len() as f64;

    let vector: Vec<f64> = vocabulary.iter().map(|term| {
        let term_lower = term.to_lowercase();
        let count = words.iter().filter(|w| w.contains(term_lower.as_str())).count() as f64;
        // TF
        let tf = if total_words > 0.0 { count / total_words } else { 0.0 };
        // 简单 IDF 近似
        let idf = 1.0; // 生产环境中应从语料库计算
        tf * idf
    }).collect();

    Embedding { chunk_id: chunk.id.clone(), vector }
}

/// 两个向量之间的余弦相似度
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

/// 带相关性分数的搜索结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub score: f64,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MatchType {
    Semantic,
    Keyword,
    Hybrid,
}

/// RAG 的内存向量存储
pub struct VectorStore {
    chunks: Vec<Chunk>,
    embeddings: Vec<Embedding>,
    vocabulary: Vec<String>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            embeddings: Vec::new(),
            vocabulary: Vec::new(),
        }
    }

    /// 通过分块和 embedding 索引一个文档
    pub fn index_document(&mut self, text: &str, source: &str, config: &ChunkConfig) {
        let chunks = chunk_text(text, source, config);

        // 从 chunk 构建词表
        for chunk in &chunks {
            for word in chunk.content.to_lowercase().split_whitespace() {
                let word = word.to_string();
                if !self.vocabulary.contains(&word) && word.len() > 2 {
                    self.vocabulary.push(word);
                }
            }
        }

        // embedding 并存储
        for chunk in chunks {
            let embedding = embed_chunk(&chunk, &self.vocabulary);
            self.embeddings.push(embedding);
            self.chunks.push(chunk);
        }
    }

    /// 使用余弦相似度进行语义搜索
    pub fn search_semantic(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let query_embedding = embed_chunk(
            &Chunk { id: "query".into(), content: query.to_string(), source: "".into(),
                     chunk_index: 0, start_offset: 0, end_offset: 0, metadata: Default::default() },
            &self.vocabulary,
        );

        let mut results: Vec<SearchResult> = self.embeddings.iter().zip(self.chunks.iter())
            .map(|(emb, chunk)| {
                let score = cosine_similarity(&query_embedding.vector, &emb.vector);
                SearchResult { chunk: chunk.clone(), score, match_type: MatchType::Semantic }
            })
            .filter(|r| r.score > 0.0)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    /// BM25 风格的关键词搜索
    pub fn search_keyword(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let query_terms: Vec<String> = query.to_lowercase().split_whitespace()
            .filter(|t| t.len() > 2)
            .map(|t| t.to_string())
            .collect();

        let avg_dl = if self.chunks.is_empty() { 1.0 } else {
            self.chunks.iter().map(|c| c.content.len() as f64).sum::<f64>() / self.chunks.len() as f64
        };

        let mut results: Vec<SearchResult> = self.chunks.iter().map(|chunk| {
            let content_lower = chunk.content.to_lowercase();
            let dl = content_lower.len() as f64;

            let score: f64 = query_terms.iter().map(|term| {
                let tf = content_lower.matches(term.as_str()).count() as f64;
                let k1 = 1.5;
                let b = 0.75;
                let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avg_dl));
                // IDF 近似
                let df = self.chunks.iter().filter(|c| c.content.to_lowercase().contains(term.as_str())).count() as f64;
                let n = self.chunks.len() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).max(0.01);
                tf_norm * idf
            }).sum();

            SearchResult { chunk: chunk.clone(), score, match_type: MatchType::Keyword }
        }).collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    /// 结合语义和关键词的混合搜索，使用倒数排名融合
    pub fn search_hybrid(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let semantic_results = self.search_semantic(query, top_k * 2);
        let keyword_results = self.search_keyword(query, top_k * 2);

        // 倒数排名融合
        let k = 60.0;
        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut chunk_map: std::collections::HashMap<String, Chunk> = std::collections::HashMap::new();

        for (rank, result) in semantic_results.iter().enumerate() {
            *scores.entry(result.chunk.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64 + 1.0);
            chunk_map.insert(result.chunk.id.clone(), result.chunk.clone());
        }
        for (rank, result) in keyword_results.iter().enumerate() {
            *scores.entry(result.chunk.id.clone()).or_insert(0.0) += 1.0 / (k + rank as f64 + 1.0);
            chunk_map.insert(result.chunk.id.clone(), result.chunk.clone());
        }

        let mut results: Vec<SearchResult> = scores.into_iter().filter_map(|(id, score)| {
            chunk_map.get(&id).map(|chunk| SearchResult {
                chunk: chunk.clone(),
                score,
                match_type: MatchType::Hybrid,
            })
        }).collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }

    /// 获取已索引的 chunk 总数
    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    /// 清除所有已索引数据
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.embeddings.clear();
        self.vocabulary.clear();
    }
}

/// HyDE：假设文档 embedding — 弥合 query 与 document 之间的词表差距
pub fn hyde_expand(query: &str) -> String {
    // 生产环境中，使用 LLM 生成假设性回答
    // 然后对假设性回答而非 query 进行 embedding
    format!(
        "假设以下是对该问题的详细回答：\n{}\n\n基于此回答进行搜索。",
        query
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let text = "This is a test document with enough words to create multiple chunks for testing the chunking algorithm. We need many more words here to exceed the minimum chunk token threshold. Adding more content ensures the chunking function will actually produce output. The quick brown fox jumps over the lazy dog near the riverbank. Artificial intelligence is transforming how we interact with technology and the world around us. Machine learning algorithms can process vast amounts of data to find patterns that humans might miss. Deep learning neural networks have achieved remarkable results in image recognition and natural language processing.";
        let chunks = chunk_text(text, "test.md", &ChunkConfig::default());
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);
    }

    #[test]
    fn test_vector_store_index_and_search() {
        let mut store = VectorStore::new();
        let doc1 = "Alice met Bob in the forest on a sunny day. They walked together through the trees discussing their plans for the afternoon adventure that awaited them in the nearby village. The birds were singing and the wind was gentle.";
        let doc2 = "Charlie found treasure in the cave behind the waterfall. The golden coins sparkled in the dim light as he reached for the ancient chest that had been hidden for centuries beneath the mountain.";
        let config = ChunkConfig { min_chunk_tokens: 10, ..Default::default() };
        store.index_document(doc1, "doc1.md", &config);
        store.index_document(doc2, "doc2.md", &config);

        let results = store.search_keyword("Alice", 5);
        assert!(!results.is_empty());
    }
}
