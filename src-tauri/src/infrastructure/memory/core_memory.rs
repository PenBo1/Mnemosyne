//! ═══════════════════════════════════════════════════════════════════════════
//! 核心记忆存储 - MEMORY.md 文件存储
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use std::sync::Arc;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::fs::fs_utils::validate_id_component;
use crate::shared::error::AppError;

pub struct CoreMemoryStore {
    data_dir: Arc<DataDir>,
}

impl CoreMemoryStore {
    pub fn new(data_dir: Arc<DataDir>) -> Self {
        Self { data_dir }
    }

    fn memory_file_path(&self, role: &str) -> Result<PathBuf, AppError> {
        validate_id_component(role, "role")?;
        Ok(self.data_dir.agents_dir().join(role).join("MEMORY.md"))
    }

    pub async fn load(&self, role: &str) -> Result<String, AppError> {
        let path = self.memory_file_path(role)?;
        let path_clone = path.clone();

        tokio::task::spawn_blocking(move || {
            if !path_clone.exists() {
                return Ok(String::new());
            }
            std::fs::read_to_string(&path_clone)
                .map_err(|_e| AppError::file_read_error(path_clone.display().to_string()))
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))?
    }

    pub async fn append(&self, role: &str, content: &str) -> Result<(), AppError> {
        if content.trim().is_empty() {
            return Err(AppError::invalid_input("Content cannot be empty"));
        }

        let path = self.memory_file_path(role)?;
        let path_clone = path.clone();
        let content_owned = content.to_string();

        tokio::task::spawn_blocking(move || {
            if let Some(parent) = path_clone.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::internal(format!("Failed to create dir: {}", e)))?;
            }

            let mut existing = String::new();
            if path_clone.exists() {
                existing = std::fs::read_to_string(&path_clone)
                    .map_err(|_e| AppError::file_read_error(path_clone.display().to_string()))?;
            }

            let new_content = if existing.is_empty() {
                content_owned
            } else {
                format!("{}\n\n{}", existing.trim_end(), content_owned.trim_start())
            };

            std::fs::write(&path_clone, new_content)
                .map_err(|_e| AppError::file_write_error(path_clone.display().to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))?
    }

    pub async fn replace(&self, role: &str, old_content: &str, new_content: &str) -> Result<bool, AppError> {
        if old_content.trim().is_empty() {
            return Err(AppError::invalid_input("Old content cannot be empty"));
        }

        let path = self.memory_file_path(role)?;
        let path_clone = path.clone();
        let old_owned = old_content.to_string();
        let new_owned = new_content.to_string();

        tokio::task::spawn_blocking(move || {
            if !path_clone.exists() {
                return Ok(false);
            }

            let existing = std::fs::read_to_string(&path_clone)
                .map_err(|_e| AppError::file_read_error(path_clone.display().to_string()))?;

            if !existing.contains(&old_owned) {
                return Ok(false);
            }

            let updated = existing.replace(&old_owned, &new_owned);

            std::fs::write(&path_clone, updated)
                .map_err(|_e| AppError::file_write_error(path_clone.display().to_string()))?;

            Ok(true)
        })
        .await
        .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (CoreMemoryStore, TempDir) {
        let tmp = TempDir::new().expect("temp dir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        data_dir.initialize().expect("init");
        (CoreMemoryStore::new(Arc::new(data_dir)), tmp)
    }

    #[tokio::test]
    async fn load_empty_when_not_exists() {
        let (store, _tmp) = make_store();
        // 修复：load/append/replace 为 async 方法，须 .await（历史测试代码漏写）
        let content = store.load("test-agent").await.unwrap();
        assert!(content.is_empty());
    }

    #[tokio::test]
    async fn append_creates_file() {
        let (store, _tmp) = make_store();
        store.append("test-agent", "First memory").await.unwrap();
        let loaded = store.load("test-agent").await.unwrap();
        assert!(loaded.contains("First memory"));
    }

    #[tokio::test]
    async fn append_appends_to_existing() {
        let (store, _tmp) = make_store();
        store.append("test-agent", "First").await.unwrap();
        store.append("test-agent", "Second").await.unwrap();
        let loaded = store.load("test-agent").await.unwrap();
        assert!(loaded.contains("First"));
        assert!(loaded.contains("Second"));
    }

    #[tokio::test]
    async fn replace_updates_content() {
        let (store, _tmp) = make_store();
        store.append("test-agent", "Old content here").await.unwrap();
        let replaced = store
            .replace("test-agent", "Old content", "New content")
            .await
            .unwrap();
        assert!(replaced);
        let loaded = store.load("test-agent").await.unwrap();
        assert!(loaded.contains("New content"));
        assert!(!loaded.contains("Old content"));
    }

    #[tokio::test]
    async fn replace_returns_false_when_not_found() {
        let (store, _tmp) = make_store();
        store.append("test-agent", "Some content").await.unwrap();
        let replaced = store
            .replace("test-agent", "Nonexistent", "New")
            .await
            .unwrap();
        assert!(!replaced);
    }

    #[tokio::test]
    async fn rejects_empty_append() {
        let (store, _tmp) = make_store();
        let result = store.append("test-agent", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_empty_old_in_replace() {
        let (store, _tmp) = make_store();
        let result = store.replace("test-agent", "", "new").await;
        assert!(result.is_err());
    }
}