use serde::{Deserialize, Serialize};

/// A text chunk with metadata for RAG
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

/// Configuration for text chunking
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

/// Estimate tokens (same as token_budget)
fn estimate_tokens(text: &str) -> usize {
    let mut count = 0usize;
    for ch in text.chars() {
        if ch.is_ascii() { count += 1; } else { count += 2; }
    }
    (count + 3) / 4
}

/// Split text into overlapping chunks
pub fn chunk_text(text: &str, source: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < chars.len() {
        // Find end position based on token estimate
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

        // Move start forward with overlap
        let advance = end - start;
        let overlap_chars = config.overlap_tokens * 4; // rough char estimate
        start = if advance > overlap_chars { end - overlap_chars } else { end };
    }

    chunks
}

/// TF-IDF style embedding for a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub chunk_id: String,
    pub vector: Vec<f64>,
}

/// Simple TF-IDF embedding (production should use neural embeddings)
pub fn embed_chunk(chunk: &Chunk, vocabulary: &[String]) -> Embedding {
    let content_lower = chunk.content.to_lowercase();
    let words: Vec<&str> = content_lower.split_whitespace().collect();
    let total_words = words.len() as f64;

    let vector: Vec<f64> = vocabulary.iter().map(|term| {
        let term_lower = term.to_lowercase();
        let count = words.iter().filter(|w| w.contains(term_lower.as_str())).count() as f64;
        // TF
        let tf = if total_words > 0.0 { count / total_words } else { 0.0 };
        // Simple IDF approximation
        let idf = 1.0; // In production, compute from corpus
        tf * idf
    }).collect();

    Embedding { chunk_id: chunk.id.clone(), vector }
}

/// Cosine similarity between two vectors
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

/// Search result with relevance score
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

/// In-memory vector store for RAG
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

    /// Index a document by chunking and embedding it
    pub fn index_document(&mut self, text: &str, source: &str, config: &ChunkConfig) {
        let chunks = chunk_text(text, source, config);

        // Build vocabulary from chunks
        for chunk in &chunks {
            for word in chunk.content.to_lowercase().split_whitespace() {
                let word = word.to_string();
                if !self.vocabulary.contains(&word) && word.len() > 2 {
                    self.vocabulary.push(word);
                }
            }
        }

        // Embed and store
        for chunk in chunks {
            let embedding = embed_chunk(&chunk, &self.vocabulary);
            self.embeddings.push(embedding);
            self.chunks.push(chunk);
        }
    }

    /// Semantic search using cosine similarity
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

    /// BM25-style keyword search
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
                // IDF approximation
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

    /// Hybrid search combining semantic and keyword with reciprocal rank fusion
    pub fn search_hybrid(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let semantic_results = self.search_semantic(query, top_k * 2);
        let keyword_results = self.search_keyword(query, top_k * 2);

        // Reciprocal Rank Fusion
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

    /// Get total chunks indexed
    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    /// Clear all indexed data
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.embeddings.clear();
        self.vocabulary.clear();
    }
}

/// HyDE: Hypothetical Document Embedding - bridges query-document vocabulary gap
pub fn hyde_expand(query: &str) -> String {
    // In production, use LLM to generate a hypothetical answer
    // Then embed the hypothetical answer instead of the query
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
