// StateManager。
//
// 职责：
// 1. 确保书籍目录结构 + control docs（author_intent.md / current_focus.md / style_guide.md）
// 2. 加载 control docs
// 3. book.json 读写
// 4. 书籍写锁（防并发写入同一本书）
// 5. chapter index 读写（chapters.json）
//
// 简化说明：完整版本的 book lock 跨进程（基于 lock 文件 + stale 检测），
// Rust 版用 Mutex<HashMap<BookId, ()>> 做进程内互斥（Tauri 单进程多线程足够）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::super::types::{BookConfig, Language};
use crate::shared::error::AppError;

/// 全局书籍写锁注册表（进程级单例）
fn global_lock_registry() -> &'static Mutex<HashMap<String, ()>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, ()>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 书籍写锁 guard。Drop 时自动释放。
pub struct BookLockGuard {
    book_id: String,
}

impl Drop for BookLockGuard {
    fn drop(&mut self) {
        if let Ok(mut registry) = global_lock_registry().lock() {
            registry.remove(&self.book_id);
        }
    }
}

/// 状态管理器
pub struct StateManager;

impl StateManager {
    /// 尝试获取书籍写锁。若已被占用返回 Err。
    pub fn acquire_book_lock(book_id: &str) -> Result<BookLockGuard, AppError> {
        let mut registry = global_lock_registry()
            .lock()
            .map_err(|e| AppError::internal(format!("lock registry poisoned: {}", e)))?;
        if registry.contains_key(book_id) {
            return Err(AppError::agent_busy());
        }
        registry.insert(book_id.to_string(), ());
        Ok(BookLockGuard {
            book_id: book_id.to_string(),
        })
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

        // author_intent.md
        let author_intent_path = story_dir.join("author_intent.md");
        let author_intent_content = match author_intent {
            Some(text) if !text.trim().is_empty() => format!("{}\n", text.trim_end()),
            _ => default_author_intent(language),
        };
        write_if_missing(&author_intent_path, &author_intent_content)?;

        // current_focus.md
        let current_focus_path = story_dir.join("current_focus.md");
        write_if_missing(&current_focus_path, &default_current_focus(language))?;

        // state/ 目录（用于 runtime-state-store 的 JSON）
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
