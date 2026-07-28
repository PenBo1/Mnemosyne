//! ═══════════════════════════════════════════════════════════════════════════
//! StateManager - 状态管理器
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：
//! 1. 确保书籍目录结构 + control docs（author_intent.md / current_focus.md / style_guide.md）
//! 2. 加载 control docs
//! 3. book.json 读写
//! 4. 书籍写锁（跨进程：基于 lock 文件 + PID 存活检测）
//! 5. chapter index 读写（chapters.json）
//!
//! 跨进程锁实现：
//! - 锁文件路径：<bookDir>/.write.lock
//! - 锁文件内容：pid:<PID> ts:<unix_ms>
//! - 创建方式：OpenOptions::new().create_new(true).write(true) 原子创建
//! - stale 检测：锁文件存在时读取 PID，若 PID 进程已死则回收
//! - 同进程追踪：active_writes 集合检测"锁文件写着本进程 PID 但本进程未持有"

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::types::{BookConfig, Language};
use crate::shared::error::AppError;

/// 全局书籍写锁注册表（进程级单例）
fn global_lock_registry() -> &'static Mutex<HashMap<String, ()>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, ()>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 同进程活跃写集合（检测"锁文件写着本进程 PID 但本进程未持有"的悬挂场景）
fn active_writes_set() -> &'static Mutex<std::collections::HashSet<String>> {
    static SET: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// 书籍写锁 guard。Drop 时自动释放（进程内 HashMap + 跨进程 lock 文件）。
pub struct BookLockGuard {
    book_id: String,
    book_dir: PathBuf,
}

impl Drop for BookLockGuard {
    fn drop(&mut self) {
        if let Ok(mut registry) = global_lock_registry().lock() {
            registry.remove(&self.book_id);
        }
        if let Ok(mut set) = active_writes_set().lock() {
            set.remove(&self.book_id);
        }
        let lock_path = self.book_dir.join(".write.lock");
        let _ = std::fs::remove_file(&lock_path);
    }
}

/// 状态管理器
pub struct StateManager;

impl StateManager {
    /// 尝试获取书籍写锁。若已被占用返回 Err。
    ///
    /// 跨进程实现：
    /// 1. 先检查进程内 HashMap（快速路径，避免同进程重复 IO）
    /// 2. 再尝试原子创建 <bookDir>/.write.lock
    /// 3. 创建失败（EEXIST）→ 读取 PID + stale 检测 → 回收或抛错
    pub fn acquire_book_lock(book_id: &str, book_dir: &Path) -> Result<BookLockGuard, AppError> {
        // 迭代式获取锁，限制最大重试次数避免无限递归/循环。
        // 每轮：进程内快速路径 → 原子 create_new lock 文件 → 已存在则 stale 检测 → 回收后重试。
        const MAX_ACQUIRE_RETRIES: usize = 3;
        let lock_path = book_dir.join(".write.lock");

        for attempt in 0..MAX_ACQUIRE_RETRIES {
            // ── Step 1: 进程内快速路径 ──
            {
                let mut registry = global_lock_registry()
                    .lock()
                    .map_err(|e| AppError::internal(format!("lock registry poisoned: {}", e)))?;
                if registry.contains_key(book_id) {
                    return Err(AppError::agent_busy());
                }
                registry.insert(book_id.to_string(), ());
            }

            // ── Step 2: 跨进程 lock 文件（原子 create_new）──
            let lock_content = format!(
                "pid:{} ts:{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );

            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .truncate(true)
                .open(&lock_path)
            {
                Ok(mut f) => {
                    use std::io::Write;
                    if let Err(e) = f.write_all(lock_content.as_bytes()) {
                        rollback_registry(book_id);
                        let _ = std::fs::remove_file(&lock_path);
                        return Err(AppError::internal(format!("lock file write failed: {}", e)));
                    }
                    // ── Step 4: 加入同进程活跃写集合 ──
                    if let Ok(mut set) = active_writes_set().lock() {
                        set.insert(book_id.to_string());
                    }
                    return Ok(BookLockGuard {
                        book_id: book_id.to_string(),
                        book_dir: book_dir.to_path_buf(),
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // ── Step 3: stale 检测 ──
                    let stale = Self::check_lock_stale(&lock_path, book_id);
                    if stale {
                        if let Err(e) = std::fs::remove_file(&lock_path) {
                            rollback_registry(book_id);
                            return Err(AppError::internal(format!(
                                "failed to remove stale lock: {}",
                                e
                            )));
                        }
                        // 回收成功 → 回滚 registry 进入下一轮重试（原子 create_new 会重新竞争）
                        rollback_registry(book_id);
                        if attempt + 1 >= MAX_ACQUIRE_RETRIES {
                            tracing::warn!(
                                book_id = %book_id,
                                attempts = attempt + 1,
                                "Acquire lock: stale lock recycled but retry limit reached"
                            );
                            return Err(AppError::agent_busy());
                        }
                        continue;
                    }
                    rollback_registry(book_id);
                    let lock_data = std::fs::read_to_string(&lock_path).unwrap_or_default();
                    tracing::warn!(
                        book_id = %book_id,
                        lock_data = %lock_data,
                        "Book is locked by another process"
                    );
                    return Err(AppError::agent_busy());
                }
                Err(e) => {
                    rollback_registry(book_id);
                    return Err(AppError::internal(format!(
                        "failed to create lock file at {}: {}",
                        lock_path.display(),
                        e
                    )));
                }
            }
        }
        Err(AppError::agent_busy())
    }

    /// 检测 lock 文件是否 stale。
    ///
    /// stale 判定（满足任一即 stale）：
    /// A. lock 文件中的 PID 进程已死（is_process_alive 返回 false）
    /// B. lock 文件中的 PID == 当前进程 PID，但当前进程的 active_writes 中没有这本书
    fn check_lock_stale(lock_path: &Path, book_id: &str) -> bool {
        let content = match std::fs::read_to_string(lock_path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let lock_pid = match extract_lock_pid(&content) {
            Some(p) => p,
            None => return false,
        };

        if lock_pid == std::process::id() {
            // PID 是本进程 → 检查 active_writes 是否包含此 book
            let in_set = active_writes_set()
                .lock()
                .map(|s| s.contains(book_id))
                .unwrap_or(false);
            !in_set
        } else {
            // PID 是其他进程 → 探测存活
            !is_process_alive(lock_pid)
        }
    }

    /// 确保 control docs 存在（author_intent.md / current_focus.md / style_guide.md）
    /// 若已存在则不覆盖。
    pub fn ensure_control_documents(
        book_dir: &Path,
        language: Language,
        author_intent: Option<&str>,
    ) -> Result<(), AppError> {
        let story_dir = book_dir.join("story");
        let runtime_dir = story_dir.join("runtime");
        let outline_dir = story_dir.join("outline");
        let roles_major = story_dir.join("roles").join("主要角色");
        let roles_minor = story_dir.join("roles").join("次要角色");

        std::fs::create_dir_all(&story_dir)?;
        std::fs::create_dir_all(&runtime_dir)?;
        std::fs::create_dir_all(&outline_dir)?;
        std::fs::create_dir_all(&roles_major)?;
        std::fs::create_dir_all(&roles_minor)?;

        let author_intent_path = story_dir.join("author_intent.md");
        let author_intent_content = match author_intent {
            Some(text) if !text.trim().is_empty() => format!("{}\n", text.trim_end()),
            _ => default_author_intent(language),
        };
        write_if_missing(&author_intent_path, &author_intent_content)?;

        let current_focus_path = story_dir.join("current_focus.md");
        write_if_missing(&current_focus_path, &default_current_focus(language))?;

        let state_dir = story_dir.join("state");
        std::fs::create_dir_all(&state_dir)?;

        Ok(())
    }

    /// 加载 control docs
    pub fn load_control_documents(
        book_dir: &Path,
        language: Language,
    ) -> Result<ControlDocuments, AppError> {
        Self::ensure_control_documents(book_dir, language, None)?;

        let story_dir = book_dir.join("story");
        let author_intent = std::fs::read_to_string(story_dir.join("author_intent.md"))
            .unwrap_or_default();
        let current_focus = std::fs::read_to_string(story_dir.join("current_focus.md"))
            .unwrap_or_default();
        let style_guide = std::fs::read_to_string(story_dir.join("style_guide.md"))
            .unwrap_or_default();

        Ok(ControlDocuments {
            author_intent,
            current_focus,
            style_guide,
            story_dir,
        })
    }

    /// 读取 book.json
    pub fn load_book_config(book_dir: &Path) -> Result<BookConfig, AppError> {
        let path = book_dir.join("book.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|_| AppError::file_not_found(path.display().to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::invalid_format(format!("book.json parse: {}", e)))
    }

    /// 写入 book.json
    pub fn save_book_config(book_dir: &Path, config: &BookConfig) -> Result<(), AppError> {
        let path = book_dir.join("book.json");
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&path, content)
            .map_err(|_| AppError::file_write_error(path.display().to_string()))?;
        Ok(())
    }
}

// ── lock PID 提取与存活检测 ──────────────────────────────────

fn rollback_registry(book_id: &str) {
    if let Ok(mut registry) = global_lock_registry().lock() {
        registry.remove(book_id);
    }
}

/// 从 lock 文件内容中提取 PID。
/// 格式：`pid:<PID> ts:<unix_ms>`
fn extract_lock_pid(content: &str) -> Option<u32> {
    let pid_marker = "pid:";
    let start = content.find(pid_marker)?;
    let after = &content[start + pid_marker.len()..];
    let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        None
    } else {
        num_str.parse().ok()
    }
}

/// 检测 PID 对应进程是否存活。
///
/// Windows: OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) 试探。
/// Unix: kill(pid, 0) 试探。
/// 保守策略：探测失败时返回 true（避免误删他人持有的锁）。
#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    // PID 0 是 System Idle Process，永远存活且不属于任何用户进程；
    // 视为非法输入，保守返回 true 以避免误删可能存在的他人锁。
    if pid == 0 {
        return true;
    }

    extern "system" {
        fn OpenProcess(
            desired_access: u32,
            inherit_handle: i32,
            process_id: u32,
        ) -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn GetExitCodeProcess(handle: *mut std::ffi::c_void, exit_code: *mut u32) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    // SAFETY: 调用 Win32 API。`pid` 来自 lock 文件解析（已校验非 0）。
    // - OpenProcess 返回的句柄在 CloseHandle 前有效；本函数在所有返回路径前都调用 CloseHandle。
    // - GetExitCodeProcess 写入 exit_code 指向的栈变量，指针有效且对齐。
    // - 句柄不跨 await/FFI 边界泄漏，生命周期严格局限于本函数。
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            // OpenProcess 失败:可能进程不存在,也可能权限不足。
            // 保守起见返回 true(避免误删他人锁)。
            true
        } else {
            // 用 GetExitCodeProcess 判断进程是否仍在运行
            let mut exit_code: u32 = 0;
            let ok = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);
            // 调用成功且退出码为 STILL_ACTIVE 表示进程存活；
            // 退出码非 STILL_ACTIVE（包括 0）表示进程已退出。
            ok != 0 && exit_code == STILL_ACTIVE
        }
    }
}

#[cfg(not(windows))]
fn is_process_alive(pid: u32) -> bool {
    // Unix: kill(pid, 0) 试探
    let result = unsafe { libc::kill(pid as i32, 0) };
    if result == 0 {
        true
    } else {
        let err = std::io::Error::last_os_error();
        err.raw_os_error() != Some(libc::ESRCH)
    }
}

/// control docs 加载结果
pub struct ControlDocuments {
    pub author_intent: String,
    pub current_focus: String,
    pub style_guide: String,
    pub story_dir: PathBuf,
}

fn write_if_missing(path: &Path, content: &str) -> Result<(), AppError> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
            .map_err(|_| AppError::file_write_error(path.display().to_string()))?;
    }
    Ok(())
}

fn default_author_intent(language: Language) -> String {
    match language {
        Language::Zh => "# 作者意图\n\n（在这里描述这本书的长期创作方向。）\n".into(),
        Language::En => "# Author Intent\n\n(Describe the long-horizon vision for this book here.)\n".into(),
    }
}

fn default_current_focus(language: Language) -> String {
    match language {
        Language::Zh => "# 当前聚焦\n\n## 当前重点\n\n（描述接下来 1-3 章最需要优先推进的内容。）\n".into(),
        Language::En => "# Current Focus\n\n## Active Focus\n\n(Describe what the next 1-3 chapters should prioritize.)\n".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pid_from_lock_content() {
        assert_eq!(extract_lock_pid("pid:1234 ts:1700000000000"), Some(1234));
        assert_eq!(extract_lock_pid("pid:0 ts:0"), Some(0));
        assert_eq!(extract_lock_pid("ts:1234"), None);
        assert_eq!(extract_lock_pid(""), None);
    }

    #[test]
    fn acquires_and_releases_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let book_dir = tmp.path().join("test-book");
        std::fs::create_dir_all(&book_dir).unwrap();

        let book_id = "test-book-1".to_string();
        let guard = StateManager::acquire_book_lock(&book_id, &book_dir);
        assert!(guard.is_ok(), "first acquire should succeed");

        assert!(book_dir.join(".write.lock").exists());

        drop(guard);

        assert!(!book_dir.join(".write.lock").exists());
    }

    #[test]
    fn rejects_double_acquire_same_process() {
        let tmp = tempfile::tempdir().unwrap();
        let book_dir = tmp.path().join("test-book-2");
        std::fs::create_dir_all(&book_dir).unwrap();

        let book_id = "test-book-2".to_string();
        let guard = StateManager::acquire_book_lock(&book_id, &book_dir);
        assert!(guard.is_ok());

        let guard2 = StateManager::acquire_book_lock(&book_id, &book_dir);
        assert!(guard2.is_err());

        drop(guard);
    }
}
