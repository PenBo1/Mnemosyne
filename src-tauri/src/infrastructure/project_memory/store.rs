//! ═══════════════════════════════════════════════════════════════════════════
//! 项目记忆存储 - project_memory.md 读写
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::RwLock;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

/// project_memory.md 的软上限:256KB。
///
/// 设计依据:MEMORY.md 经验值 50-100KB,project_memory 通常更大但不宜超过 256KB。
/// 超过时让用户主动清理(导出 + 截断),不自动截断(避免静默丢数据)。
const MAX_FILE_SIZE: usize = 256 * 1024;

/// in-memory cache 最多保留多少个 workspace 的内容。
/// 超出后清空整个 cache（下次 read 会重新从磁盘加载）。
/// workspace 数量本身由用户控制，此上限仅防御异常调用场景。
const MAX_CACHED_WORKSPACES: usize = 32;

#[derive(Clone)]
pub struct ProjectMemoryStore {
    data_dir: DataDir,
    /// 简单的读缓存 —— workspace_id → 最新内容。
    /// 写入时 write-through 同步刷新缓存。
    /// 不持久化缓存,程序重启后按需重新读取。
    cache: Arc<RwLock<HashMap<String, String>>>,
}

use std::sync::Arc;

impl ProjectMemoryStore {
    pub fn new(data_dir: DataDir) -> Self {
        Self {
            data_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 读取 workspace 的 project_memory.md 内容。
    ///
    /// - 文件不存在 → 返回空字符串(不视为错误)
    /// - 文件存在但读取失败 → 返回 Err
    /// - 优先从缓存读,缓存未命中则读盘并写入缓存
    pub fn read(&self, workspace_id: &str) -> Result<String, AppError> {
        // 缓存命中
        if let Ok(cache) = self.cache.read() {
            if let Some(content) = cache.get(workspace_id) {
                return Ok(content.clone());
            }
        }

        let path = self.data_dir.workspace_memory_path(workspace_id);
        if !path.exists() {
            // 文件不存在:缓存空字符串,下次读直接命中
            self.cache_insert(workspace_id, String::new());
            return Ok(String::new());
        }

        let content = std::fs::read_to_string(&path).map_err(|e| {
            AppError::internal(format!(
                "Failed to read project memory for workspace {}: {}",
                workspace_id, e
            ))
        })?;

        self.cache_insert(workspace_id, content.clone());
        Ok(content)
    }

    /// 覆盖写入 workspace 的 project_memory.md。
    ///
    /// - 内容超过 256KB → 返回 Err(不静默截断)
    /// - 父目录不存在 → 自动创建
    /// - 写入成功 → 同步刷新缓存
    pub fn write(&self, workspace_id: &str, content: &str) -> Result<(), AppError> {
        if content.len() > MAX_FILE_SIZE {
            return Err(AppError::bad_request(format!(
                "project_memory.md exceeds size limit ({} > {} bytes). Please export and trim.",
                content.len(),
                MAX_FILE_SIZE
            )));
        }

        let dir = self.data_dir.workspace_dir(workspace_id);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::internal(format!(
                "Failed to create project memory dir for workspace {}: {}",
                workspace_id, e
            ))
        })?;

        let path = self.data_dir.workspace_memory_path(workspace_id);
        std::fs::write(&path, content).map_err(|e| {
            AppError::internal(format!(
                "Failed to write project memory for workspace {}: {}",
                workspace_id, e
            ))
        })?;

        self.cache_insert(workspace_id, content.to_string());
        tracing::info!(
            workspace_id,
            bytes = content.len(),
            "project_memory.md updated"
        );
        Ok(())
    }

    /// 在文件末尾追加一段内容(自动加换行分隔)。
    ///
    /// 用于 agent 自动记录"本次 session 在此 workspace 学到的事实"。
    /// - 文件不存在 → 等价于 write(直接创建)
    /// - 追加后超过 256KB → 返回 Err(已追加的部分会落盘,调用方需自行处理)
    pub fn append(&self, workspace_id: &str, section: &str) -> Result<(), AppError> {
        let existing = self.read(workspace_id)?;
        let separator = if existing.is_empty() || existing.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let new_content = format!("{existing}{separator}{section}\n");

        if new_content.len() > MAX_FILE_SIZE {
            return Err(AppError::bad_request(format!(
                "project_memory.md would exceed size limit after append ({} > {} bytes)",
                new_content.len(),
                MAX_FILE_SIZE
            )));
        }
        self.write(workspace_id, &new_content)
    }

    /// 删除 workspace 的 project_memory.md 及其所在目录。
    ///
    /// 由 delete_workspace 调用以级联清理。
    /// - 文件/目录不存在 → 静默返回 Ok
    /// - 删除失败 → 返回 Err(不阻塞 workspace 删除流程,由调用方决定如何处理)
    pub fn delete(&self, workspace_id: &str) -> Result<(), AppError> {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(workspace_id);
        }
        let dir = self.data_dir.workspace_dir(workspace_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| {
                AppError::internal(format!(
                    "Failed to remove project memory dir for workspace {}: {}",
                    workspace_id, e
                ))
            })?;
        }
        Ok(())
    }

    /// 清空 workspace 的 project_memory.md 内容(文件保留,内容置空)。
    ///
    /// 与 delete 区别:delete 删除文件,clear 清空内容(保留文件路径)。
    pub fn clear(&self, workspace_id: &str) -> Result<(), AppError> {
        self.write(workspace_id, "")
    }

    /// 返回文件大小上限(供前端显示"已使用 X / Y KB")
    pub fn max_size(&self) -> usize {
        MAX_FILE_SIZE
    }

    /// 检查 workspace 的 project_memory.md 是否存在(供 stats 命令使用)。
    ///
    /// 与 `read` 不同:read 返回空串可能意味着"文件不存在"或"文件存在但内容为空",
    /// 此方法明确区分两种状态(避免空文件被误判为不存在)。
    pub fn exists(&self, workspace_id: &str) -> bool {
        self.data_dir.workspace_memory_path(workspace_id).exists()
    }

    /// 写入 cache 前的防御性检查：超过上限时清空整个 cache。
    /// workspace 数量本身由用户控制（通常 < 10），此清理仅防御异常调用。
    fn cache_insert(&self, workspace_id: &str, content: String) {
        if let Ok(mut cache) = self.cache.write() {
            if cache.len() >= MAX_CACHED_WORKSPACES && !cache.contains_key(workspace_id) {
                cache.clear();
            }
            cache.insert(workspace_id.to_string(), content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (ProjectMemoryStore, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        let store = ProjectMemoryStore::new(data_dir);
        (store, tmp)
    }

    #[test]
    fn read_missing_returns_empty() {
        let (store, _tmp) = temp_store();
        let result = store.read("ws-1").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn write_then_read_roundtrip() {
        let (store, _tmp) = temp_store();
        store.write("ws-1", "hello world").unwrap();
        assert_eq!(store.read("ws-1").unwrap(), "hello world");
    }

    #[test]
    fn append_creates_file_if_missing() {
        let (store, _tmp) = temp_store();
        store.append("ws-2", "first entry").unwrap();
        assert_eq!(store.read("ws-2").unwrap(), "first entry\n");
    }

    #[test]
    fn append_adds_separator() {
        let (store, _tmp) = temp_store();
        store.write("ws-3", "existing content").unwrap();
        store.append("ws-3", "new section").unwrap();
        let result = store.read("ws-3").unwrap();
        assert!(result.contains("existing content"));
        assert!(result.contains("new section"));
        // 中间应该有换行分隔
        assert!(result.contains("existing content\nnew section"));
    }

    #[test]
    fn delete_removes_dir_and_invalidates_cache() {
        let (store, _tmp) = temp_store();
        store.write("ws-del", "to be deleted").unwrap();
        let path = store.data_dir.workspace_memory_path("ws-del");
        assert!(path.exists());

        store.delete("ws-del").unwrap();
        assert!(!path.exists());
        // 删除后读取应返回空(走文件不存在分支)
        assert_eq!(store.read("ws-del").unwrap(), "");
    }

    #[test]
    fn write_overwrites_existing() {
        let (store, _tmp) = temp_store();
        store.write("ws-4", "v1").unwrap();
        store.write("ws-4", "v2").unwrap();
        assert_eq!(store.read("ws-4").unwrap(), "v2");
    }

    #[test]
    fn oversized_write_rejected() {
        let (store, _tmp) = temp_store();
        let big = "x".repeat(MAX_FILE_SIZE + 1);
        let err = store.write("ws-5", &big).unwrap_err();
        // bad_request 错误码
        assert!(err.to_string().contains("size limit") || err.to_string().contains("exceeds"));
    }

    #[test]
    fn clear_empties_content() {
        let (store, _tmp) = temp_store();
        store.write("ws-6", "content").unwrap();
        store.clear("ws-6").unwrap();
        assert_eq!(store.read("ws-6").unwrap(), "");
    }
}
