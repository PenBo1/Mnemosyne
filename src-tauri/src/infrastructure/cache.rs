//! ═══════════════════════════════════════════════════════════════════════════
//! Cache - 通用两级缓存（in-process LRU + disk snapshot）
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 实现 in-process LRU + disk snapshot 两层缓存：
//! - LRU 命中直接返回
//! - LRU 未命中时查 disk snapshot，验证 manifest 后使用
//! - 两者都未命中则调用 loader 重新加载

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::shared::error::AppError;

// ── CacheEntry: LRU 缓存条目 ────────────────────────────────────────────────────────

/// LRU 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// 缓存的值
    pub value: T,
    /// 进入进程缓存的时间（单调时钟，进程内有效，不可持久化）
    pub inserted_at: Instant,
    /// 源文件的修改时间（用于校验 snapshot 是否过期）
    pub mtime: SystemTime,
    /// 源文件的大小（字节）
    pub size: u64,
}

// ── LruCache: in-process LRU 缓存 ────────────────────────────────────────────────────────

/// in-process LRU 缓存
///
/// 简单 LRU 实现：HashMap 存数据，Vec 维护访问/插入顺序，容量满时淘汰最旧条目。
/// 不引入外部 lru crate，遵循最小依赖原则。
pub struct LruCache<T: Clone> {
    entries: HashMap<String, CacheEntry<T>>,
    /// 访问顺序：头部为最旧（待淘汰），尾部为最新
    order: Vec<String>,
    max_entries: usize,
}

impl<T: Clone> LruCache<T> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            max_entries,
        }
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.entries.get(key).map(|e| &e.value)
    }

    pub fn insert(&mut self, key: String, value: T, mtime: SystemTime, size: u64) {
        if self.entries.contains_key(&key) {
            // 已存在：更新值并刷新插入时间，移到尾部（最新）
            let entry = self.entries.get_mut(&key).expect("checked above");
            entry.value = value;
            entry.mtime = mtime;
            entry.size = size;
            entry.inserted_at = Instant::now();
            self.touch(&key);
            return;
        }
        // 新增：容量满则淘汰头部最旧条目
        if self.entries.len() >= self.max_entries {
            if let Some(evicted) = self.order.first().cloned() {
                self.entries.remove(&evicted);
                self.order.retain(|k| k != &evicted);
            }
        }
        self.entries.insert(
            key.clone(),
            CacheEntry {
                value,
                inserted_at: Instant::now(),
                mtime,
                size,
            },
        );
        self.order.push(key);
    }

    /// 将 key 移到 order 尾部（标记为最近使用）
    fn touch(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push(key.to_string());
    }

    pub fn invalidate(&mut self, key: &str) {
        self.entries.remove(key);
        self.order.retain(|k| k != key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── SnapshotFile: disk snapshot 文件结构 ────────────────────────────────────────────────────────

/// disk snapshot 文件结构（manifest + 缓存值）
///
/// manifest 字段：source_path / mtime / mtime_nanos / size / cached_at
/// 读取时通过 source_path 重新 stat 源文件，对比 mtime/size 验证有效性。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotFile<T> {
    /// 源文件绝对路径（用于校验）
    source_path: String,
    /// 源文件修改时间（UNIX_EPOCH 起的秒数）
    mtime: u64,
    /// 源文件修改时间的纳秒部分
    mtime_nanos: u32,
    /// 源文件大小（字节）
    size: u64,
    /// 快照写入时间（ISO8601）
    cached_at: String,
    /// 缓存的值
    value: T,
}

// ── TwoTierCache: 两层缓存 ────────────────────────────────────────────────────────

/// 两层缓存：in-process LRU + disk snapshot
///
/// 注：`get_or_load` / `write_snapshot` 额外接收 `source_path` 参数，
/// 用于在读取 disk snapshot 时重新 stat 源文件、对比 mtime/size 验证有效性，
/// 避免源文件被修改后仍命中过期缓存。
pub struct TwoTierCache<T: Clone + Serialize + DeserializeOwned> {
    lru: Arc<RwLock<LruCache<T>>>,
    snapshot_dir: PathBuf,
}

impl<T: Clone + Serialize + DeserializeOwned> TwoTierCache<T> {
    pub fn new(snapshot_dir: PathBuf, lru_capacity: usize) -> Self {
        Self {
            lru: Arc::new(RwLock::new(LruCache::new(lru_capacity))),
            snapshot_dir,
        }
    }

    /// 获取：先查 LRU，再查 disk snapshot，最后调用 loader 重新加载
    ///
    /// `source_path` 为源文件路径，用于写入 snapshot 的 manifest，
    /// 供后续 `read_snapshot` 校验源文件是否变更。
    pub async fn get_or_load<F, Fut>(
        &self,
        key: &str,
        source_path: &Path,
        loader: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(T, SystemTime, u64), AppError>>,
    {
        // 1. LRU 命中
        {
            let lru = self.lru.read().await;
            if let Some(v) = lru.get(key) {
                return Ok(v.clone());
            }
        }

        // 2. disk snapshot 命中（含 manifest 验证）
        if let Some(entry) = self.read_snapshot(key).await? {
            let value = entry.value.clone();
            let mtime = entry.mtime;
            let size = entry.size;
            let mut lru = self.lru.write().await;
            lru.insert(key.to_string(), entry.value, mtime, size);
            return Ok(value);
        }

        // 3. 调用 loader 重新加载
        let (value, mtime, size) = loader().await?;
        let entry = CacheEntry {
            value: value.clone(),
            inserted_at: Instant::now(),
            mtime,
            size,
        };
        // 写 disk snapshot：失败仅记录日志，不阻塞返回（缓存写入失败不影响正确性）
        if let Err(e) = self.write_snapshot(key, source_path, &entry).await {
            tracing::warn!(key = %key, error = %e, "Failed to write skill cache snapshot");
        }
        // 回填 LRU
        let mut lru = self.lru.write().await;
        lru.insert(key.to_string(), value, mtime, size);
        Ok(entry.value)
    }

    /// 写入 disk snapshot（含 manifest）
    async fn write_snapshot(
        &self,
        key: &str,
        source_path: &Path,
        entry: &CacheEntry<T>,
    ) -> Result<(), AppError> {
        let dir = self.skill_cache_dir();
        crate::infrastructure::fs::fs_utils::ensure_dir(&dir)?;

        let (mtime_secs, mtime_nanos) = system_time_to_unix(entry.mtime);
        let snapshot = SnapshotFile {
            source_path: source_path.to_string_lossy().to_string(),
            mtime: mtime_secs,
            mtime_nanos,
            size: entry.size,
            cached_at: chrono::Utc::now().to_rfc3339(),
            value: entry.value.clone(),
        };

        let path = self.snapshot_path(key);
        crate::infrastructure::fs::fs_utils::atomic_write_json(&path, &snapshot)
    }

    /// 读取 disk snapshot（验证 manifest 后才使用）
    ///
    /// 验证逻辑：从 manifest 读取源文件路径 + 记录的 mtime/size，
    /// 重新 stat 源文件，对比 mtime/size，不匹配则丢弃 snapshot。
    async fn read_snapshot(&self, key: &str) -> Result<Option<CacheEntry<T>>, AppError> {
        let path = self.snapshot_path(key);
        if !path.exists() {
            return Ok(None);
        }

        let snapshot: SnapshotFile<T> = match crate::infrastructure::fs::fs_utils::read_json(&path)
        {
            Ok(s) => s,
            Err(e) => {
                // 解析失败：删除损坏的 snapshot，避免反复尝试
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to parse cache snapshot, removing"
                );
                let _ = std::fs::remove_file(&path);
                return Ok(None);
            }
        };

        // 校验源文件：stat 当前 mtime/size，与 manifest 记录对比
        let source = Path::new(&snapshot.source_path);
        let metadata = match std::fs::metadata(source) {
            Ok(m) => m,
            Err(_) => {
                // 源文件已不存在：snapshot 失效，清理后返回 None
                let _ = std::fs::remove_file(&path);
                return Ok(None);
            }
        };
        let current_size = metadata.len();
        let current_mtime = metadata.modified().map_err(|e| {
            AppError::internal(format!("Failed to read source file mtime: {}", e))
        })?;

        let (cur_secs, cur_nanos) = system_time_to_unix(current_mtime);
        let mtime_match = cur_secs == snapshot.mtime && cur_nanos == snapshot.mtime_nanos;
        let size_match = current_size == snapshot.size;

        if !mtime_match || !size_match {
            // 源文件已变更：丢弃过期 snapshot
            tracing::debug!(
                key = %key,
                mtime_match,
                size_match,
                "Cache snapshot stale, discarding"
            );
            let _ = std::fs::remove_file(&path);
            return Ok(None);
        }

        // 校验通过：重建 CacheEntry（inserted_at 重置为当前进程时间）
        Ok(Some(CacheEntry {
            value: snapshot.value,
            inserted_at: Instant::now(),
            mtime: current_mtime,
            size: current_size,
        }))
    }

    /// 失效缓存（LRU + disk snapshot）
    pub async fn invalidate(&self, key: &str) {
        {
            let mut lru = self.lru.write().await;
            lru.invalidate(key);
        }
        let path = self.snapshot_path(key);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }

    /// 清空整个 LRU（不影响 disk snapshot）
    pub async fn clear_lru(&self) {
        let mut lru = self.lru.write().await;
        lru.clear();
    }

    fn skill_cache_dir(&self) -> PathBuf {
        self.snapshot_dir.join("skill_cache")
    }

    fn snapshot_path(&self, key: &str) -> PathBuf {
        self.skill_cache_dir()
            .join(format!("{}.json", sanitize_key(key)))
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────

/// 将 SystemTime 转为 (UNIX_EPOCH 起的秒, 纳秒部分)
fn system_time_to_unix(t: SystemTime) -> (u64, u32) {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs(), d.subsec_nanos()),
        Err(_) => (0, 0),
    }
}

/// 将 key 净化为安全的文件名（避免路径分隔符注入）
fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_lru_get_insert() {
        let mut cache: LruCache<String> = LruCache::new(3);
        assert!(cache.get("a").is_none());

        cache.insert("a".into(), "value-a".into(), SystemTime::now(), 10);
        assert_eq!(cache.get("a"), Some(&"value-a".to_string()));
    }

    #[test]
    fn test_lru_invalidate() {
        let mut cache: LruCache<String> = LruCache::new(3);
        cache.insert("a".into(), "v".into(), SystemTime::now(), 1);
        assert!(cache.get("a").is_some());

        cache.invalidate("a");
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn test_lru_eviction_drops_oldest() {
        // 容量为 2：插入 3 个 key 后，最旧的 a 被淘汰
        let mut cache: LruCache<String> = LruCache::new(2);
        cache.insert("a".into(), "1".into(), SystemTime::now(), 1);
        cache.insert("b".into(), "2".into(), SystemTime::now(), 2);
        cache.insert("c".into(), "3".into(), SystemTime::now(), 3);

        assert!(cache.get("a").is_none(), "oldest entry should be evicted");
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    /// 读取源文件的 (mtime, size)，供 loader 返回
    fn stat_source(path: &Path) -> (SystemTime, u64) {
        let meta = std::fs::metadata(path).expect("stat source");
        (
            meta.modified().expect("modified time"),
            meta.len(),
        )
    }

    #[tokio::test]
    async fn test_two_tier_cache_hit_does_not_query_disk() {
        // LRU 命中时不查 disk（loader 第二次不应被调用）
        let tmp = tempdir().expect("tempdir");
        let src = tmp.path().join("src.txt");
        std::fs::write(&src, "hello").unwrap();

        let cache: TwoTierCache<String> = TwoTierCache::new(tmp.path().to_path_buf(), 4);
        let calls = Arc::new(AtomicU32::new(0));

        // 首次加载：loader 被调用一次
        let calls_clone = calls.clone();
        let src_clone = src.clone();
        let v1 = cache
            .get_or_load("k", &src, move || {
                let calls_clone = calls_clone.clone();
                async move {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    let (mt, sz) = stat_source(&src_clone);
                    Ok(("hello".to_string(), mt, sz))
                }
            })
            .await
            .unwrap();
        assert_eq!(v1, "hello");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // 第二次：应命中 LRU，loader 不再被调用
        let calls_clone = calls.clone();
        let src_clone = src.clone();
        let v2 = cache
            .get_or_load("k", &src, move || {
                let calls_clone = calls_clone.clone();
                async move {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    let (mt, sz) = stat_source(&src_clone);
                    Ok(("hello".to_string(), mt, sz))
                }
            })
            .await
            .unwrap();
        assert_eq!(v2, "hello");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "loader should not be called on LRU hit");
    }

    #[tokio::test]
    async fn test_two_tier_cache_disk_hit() {
        // LRU miss 时从 disk 加载（loader 不再被调用）
        let tmp = tempdir().expect("tempdir");
        let src = tmp.path().join("src.txt");
        std::fs::write(&src, "payload").unwrap();

        let cache: TwoTierCache<String> = TwoTierCache::new(tmp.path().to_path_buf(), 4);

        // 首次加载：写 LRU + disk snapshot（loader 返回预读的 mtime/size，不捕获 src）
        let (mt, sz) = stat_source(&src);
        cache
            .get_or_load("k", &src, || async move { Ok(("payload".to_string(), mt, sz)) })
            .await
            .unwrap();

        // 清空 LRU，强制下次走 disk
        cache.clear_lru().await;

        // 第二次：LRU miss → disk 命中（源文件未变）→ loader 不应被调用
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();
        let src_clone = src.clone();
        let v = cache
            .get_or_load("k", &src, move || {
                let calls_clone = calls_clone.clone();
                async move {
                    calls_clone.fetch_add(1, Ordering::SeqCst);
                    let (mt, sz) = stat_source(&src_clone);
                    Ok(("payload".to_string(), mt, sz))
                }
            })
            .await
            .unwrap();
        assert_eq!(v, "payload");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "loader should not be called on disk hit");
    }

    #[tokio::test]
    async fn test_snapshot_validation() {
        // 源文件 mtime/size 变更后，disk snapshot 被丢弃
        let tmp = tempdir().expect("tempdir");
        let src = tmp.path().join("src.txt");
        std::fs::write(&src, "v1").unwrap();

        let cache: TwoTierCache<String> = TwoTierCache::new(tmp.path().to_path_buf(), 4);

        // 写入 snapshot（manifest 记录当前 mtime/size）
        let (mt, sz) = stat_source(&src);
        cache
            .get_or_load("k", &src, || async move { Ok(("v1".to_string(), mt, sz)) })
            .await
            .unwrap();
        // 确认 snapshot 文件已生成
        assert!(cache.snapshot_path("k").exists(), "snapshot file should exist");

        // 修改源文件：内容变长（size 改变）+ mtime 改变
        std::fs::write(&src, "v2-much-longer-content").unwrap();

        // 清空 LRU，强制走 disk 校验
        cache.clear_lru().await;

        // read_snapshot 应因 size/mtime 不匹配而丢弃 snapshot，返回 None
        let result = cache.read_snapshot("k").await.unwrap();
        assert!(result.is_none(), "stale snapshot should be discarded");

        // snapshot 文件应已被清理
        assert!(!cache.snapshot_path("k").exists(), "stale snapshot file should be removed");
    }

    #[tokio::test]
    async fn test_invalidate_clears_both_tiers() {
        let tmp = tempdir().expect("tempdir");
        let src = tmp.path().join("src.txt");
        std::fs::write(&src, "data").unwrap();

        let cache: TwoTierCache<String> = TwoTierCache::new(tmp.path().to_path_buf(), 4);
        let (mt, sz) = stat_source(&src);
        cache
            .get_or_load("k", &src, || async move { Ok(("data".to_string(), mt, sz)) })
            .await
            .unwrap();
        assert!(cache.snapshot_path("k").exists());

        cache.invalidate("k").await;

        // LRU 与 disk snapshot 均被清空
        {
            let lru = cache.lru.read().await;
            assert!(lru.get("k").is_none());
        }
        assert!(!cache.snapshot_path("k").exists());
    }
}
