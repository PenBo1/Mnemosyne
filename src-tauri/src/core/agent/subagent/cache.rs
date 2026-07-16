use std::collections::HashMap;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

use super::types::{SubAgentRole, SubAgentResult};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    role: SubAgentRole,
    task_hash: u64,
    context_hash: u64,
}

impl CacheKey {
    pub fn new(role: SubAgentRole, task: &str, context: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        task.hash(&mut hasher);
        let task_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        context.hash(&mut hasher);
        let context_hash = hasher.finish();

        Self { role, task_hash, context_hash }
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    // 用 Arc<SubAgentResult> 让 `get()` 仅做 refcount bump，避免深拷贝
    // （SubAgentResult.output 可能是大段 LLM 输出文本）。
    result: Arc<SubAgentResult>,
    created_at: DateTime<Utc>,
    hits: u32,
}

impl CacheEntry {
    pub fn new(result: SubAgentResult) -> Self {
        Self {
            result: Arc::new(result),
            created_at: Utc::now(),
            hits: 0,
        }
    }

    pub fn is_expired(&self, ttl_seconds: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.created_at);
        age > Duration::seconds(ttl_seconds)
    }

    pub fn result(&self) -> &SubAgentResult {
        &*self.result
    }

    pub fn increment_hits(&mut self) {
        self.hits += 1;
    }
}

pub struct SubAgentCache {
    entries: Arc<RwLock<HashMap<CacheKey, CacheEntry>>>,
    max_entries: usize,
    ttl_seconds: i64,
}

impl SubAgentCache {
    pub fn new(max_entries: usize, ttl_seconds: i64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            ttl_seconds,
        }
    }

    pub async fn get(&self, role: SubAgentRole, task: &str, context: &str) -> Option<Arc<SubAgentResult>> {
        let key = CacheKey::new(role, task, context);
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(&key) {
            if entry.is_expired(self.ttl_seconds) {
                entries.remove(&key);
                return None;
            }

            entry.increment_hits();
            // Arc::clone 仅 refcount bump，避免深拷贝 SubAgentResult.output（可能很长）。
            return Some(Arc::clone(&entry.result));
        }

        None
    }

    pub async fn set(&self, role: SubAgentRole, task: &str, context: &str, result: SubAgentResult) {
        let key = CacheKey::new(role, task, context);
        let entry = CacheEntry::new(result);

        let mut entries = self.entries.write().await;

        if entries.len() >= self.max_entries {
            self.evict_oldest(&mut entries);
        }

        entries.insert(key, entry);
    }

    // TODO(perf): evict_oldest 是 O(n) 全表扫描。max_entries 默认 100，开销可忽略；
    // 若未来大幅扩容，可改为 BTreeMap<DateTime, Vec<CacheKey>> 索引实现 O(log n) 淘汰。
    // 当前保留 O(n) 实现，避免引入额外的索引维护复杂度与一致性问题。
    fn evict_oldest(&self, entries: &mut HashMap<CacheKey, CacheEntry>) {
        if let Some((oldest_key, _)) = entries
            .iter()
            .min_by_key(|(_, e)| e.created_at)
        {
            let key = oldest_key.clone();
            entries.remove(&key);
        }
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total_entries = entries.len();
        let total_hits: u32 = entries.values().map(|e| e.hits).sum();
        let expired_count = entries.values().filter(|e| e.is_expired(self.ttl_seconds)).count();

        CacheStats {
            total_entries,
            total_hits,
            expired_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u32,
    pub expired_count: usize,
}

impl Default for SubAgentCache {
    fn default() -> Self {
        Self::new(100, 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = SubAgentCache::new(10, 60);
        let result = SubAgentResult {
            role: SubAgentRole::Researcher,
            task: "test task".to_string(),
            output: "test output".to_string(),
            tokens_used: 100,
        };

        cache.set(SubAgentRole::Researcher, "test task", "context", result.clone()).await;

        let cached = cache.get(SubAgentRole::Researcher, "test task", "context").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().output, "test output");
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = SubAgentCache::new(10, 60);

        let cached = cache.get(SubAgentRole::Researcher, "nonexistent", "context").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = SubAgentCache::new(2, 60);

        for i in 0..3 {
            let result = SubAgentResult {
                role: SubAgentRole::Researcher,
                task: format!("task {}", i),
                output: format!("output {}", i),
                tokens_used: 100,
            };
            cache.set(SubAgentRole::Researcher, &format!("task {}", i), "context", result).await;
        }

        assert_eq!(cache.len().await, 2);
    }
}