use std::path::Path;
use crate::shared::errors::AppError;

/// snapshot 文件的垃圾回收。
/// 根据年龄和数量上限移除陈旧的 snapshot。
pub struct SnapshotGc {
    max_snapshots: usize,
    max_age_days: u64,
}

impl SnapshotGc {
    pub fn new(max_snapshots: usize, max_age_days: u64) -> Self {
        Self { max_snapshots, max_age_days }
    }

    pub fn default_config() -> Self {
        Self::new(100, 90) // 保留 100 个 snapshot，最长 90 天
    }

    /// 对 snapshot 目录运行 GC。
    /// 返回移除的文件数。
    pub fn run(&self, snapshots_dir: &Path) -> Result<usize, AppError> {
        if !snapshots_dir.exists() {
            return Ok(0);
        }

        let mut entries: Vec<_> = std::fs::read_dir(snapshots_dir)
            .map_err(|e| AppError::internal(format!("Failed to read snapshots dir: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();

        // 按修改时间排序（最旧优先）
        entries.sort_by_key(|e| {
            e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        let mut removed = 0;
        let now = std::time::SystemTime::now();
        let max_age = std::time::Duration::from_secs(self.max_age_days * 86400);

        // 按年龄移除
        for entry in &entries {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            let _ = std::fs::remove_file(entry.path());
                            removed += 1;
                        }
                    }
                }
            }
        }

        // 按年龄移除后重新读取
        let remaining: Vec<_> = std::fs::read_dir(snapshots_dir)
            .map_err(|e| AppError::internal(format!("Failed to read snapshots dir: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
            .collect();

        // 按数量移除多余项（最旧优先）
        if remaining.len() > self.max_snapshots {
            let to_remove = remaining.len() - self.max_snapshots;
            let mut sorted = remaining;
            sorted.sort_by_key(|e| {
                e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            for entry in sorted.into_iter().take(to_remove) {
                let _ = std::fs::remove_file(entry.path());
                removed += 1;
            }
        }

        Ok(removed)
    }
}

/// 跨 agent 去重公共工具函数。
pub mod utils {
    /// 按语言感知方式统计文本字数。
    /// English：按空白分隔的单词。Chinese：非 ASCII 字符 + ASCII 单词。
    pub fn count_words(text: &str, language: &str) -> u32 {
        if language == "en" {
            text.split_whitespace().count() as u32
        } else {
            let mut non_ascii = 0u32;
            for ch in text.chars() {
                if !ch.is_ascii() && !ch.is_whitespace() {
                    non_ascii += 1;
                }
            }
            let ascii_words: u32 = text.split_whitespace()
                .filter(|w| w.bytes().all(|b| b.is_ascii()))
                .count() as u32;
            non_ascii + ascii_words
        }
    }

    /// 统计字数（默认 English）。
    pub fn count_words_en(text: &str) -> u32 {
        text.split_whitespace().count() as u32
    }

    /// 从 config 读取 book 语言（通过 project_root + book_id）。
    pub fn read_book_language(project_root: &std::path::Path, book_id: &str) -> String {
        let book_dir = project_root.join("books").join(book_id);
        read_book_language_from_dir(&book_dir).unwrap_or_else(|| "zh".to_string())
    }

    /// 从 book 目录读取 book 语言（通过 book_dir 路径）。
    pub fn read_book_language_from_dir(book_dir: &std::path::Path) -> Option<String> {
        let config_path = book_dir.join("book.json");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                return config.get("language").and_then(|v| v.as_str()).map(|s| s.to_string());
            }
        }
        Some("zh".to_string())
    }

    /// 检查 book 是否为 English。
    pub fn is_english_book(book_dir: &std::path::Path) -> bool {
        let config_path = book_dir.join("book.json");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                return config.get("language").and_then(|v| v.as_str()) == Some("en");
            }
        }
        false
    }

    /// 对文件名进行清理以实现安全的文件系统使用。
    pub fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// 检查文本是否主要为 English。
    pub fn is_english_text(text: &str) -> bool {
        let ascii_count = text.chars().filter(|c| c.is_ascii() && c.is_alphabetic()).count();
        let total_count = text.chars().filter(|c| c.is_alphabetic()).count();
        if total_count == 0 { return false; }
        ascii_count as f64 / total_count as f64 > 0.7
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_words() {
        assert_eq!(utils::count_words("hello world", "en"), 2);
        assert_eq!(utils::count_words("", "en"), 0);
        assert_eq!(utils::count_words("你好世界", "zh"), 4);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(utils::sanitize_filename("hello/world"), "hello_world");
        assert_eq!(utils::sanitize_filename("file:name"), "file_name");
    }

    #[test]
    fn test_is_english_text() {
        assert!(utils::is_english_text("This is English text"));
        assert!(!utils::is_english_text("这是中文文本"));
    }
}
