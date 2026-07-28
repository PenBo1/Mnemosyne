//! ═══════════════════════════════════════════════════════════════════════════
//! 文件系统工具
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供：
//! - 日志初始化
//! - 原子写入
//! - 文件读取（带大小限制）
//! - 目录创建
//! - ID 组件校验
//! - 路径穿越防护

use std::path::Path;
use crate::shared::error::AppError;

// ── 日志初始化 ──────────────────────────────────────────────────────────────

/// 初始化日志系统
pub fn init_logging(logs_dir: &Path) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, fmt, EnvFilter};

    let _ = std::fs::create_dir_all(logs_dir);

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .max_log_files(5)
        .filename_prefix("mnemosyne")
        .filename_suffix("log")
        .build(logs_dir)
        .expect("Failed to create rolling file appender");

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| {
            #[cfg(debug_assertions)]
            { EnvFilter::try_new("debug") }
            #[cfg(not(debug_assertions))]
            { EnvFilter::try_new("info") }
        })
        .expect("Failed to create EnvFilter");

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_thread_names(false)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true)
                .with_level(true)
        )
        .init();
}

// ── 原子写入 ────────────────────────────────────────────────────────────────

/// 原子写入文件
///
/// 先写入临时文件，再重命名为目标文件，避免写入过程中断导致数据损坏。
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let dir = path.parent()
        .ok_or_else(|| AppError::internal("Cannot determine parent directory"))?;

    let temp_name = format!("{}.tmp.{}", path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file"),
        std::process::id()
    );
    let temp_path = dir.join(&temp_name);

    std::fs::write(&temp_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write temp file: {}", e)))?;

    std::fs::rename(&temp_path, path)
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            AppError::internal(format!("Failed to rename temp file: {}", e))
        })?;

    Ok(())
}

/// 原子写入 JSON 文件
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| AppError::internal(format!("Failed to serialize JSON: {}", e)))?;
    atomic_write(path, json.as_bytes())
}

// ── 文件读取 ────────────────────────────────────────────────────────────────

/// 读取文件的大小上限：10MB
pub const MAX_READ_SIZE: usize = 10 * 1024 * 1024;

/// 读取文件内容
///
/// 超过 MAX_READ_SIZE 的文件返回错误，避免 OOM。
pub fn read_file(path: &Path) -> Result<String, AppError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| AppError::internal(format!("Failed to get metadata for {}: {}", path.display(), e)))?;
    if metadata.len() > MAX_READ_SIZE as u64 {
        return Err(AppError::invalid_input(format!(
            "File too large ({} bytes > {} max): {}",
            metadata.len(), MAX_READ_SIZE, path.display()
        )));
    }
    std::fs::read_to_string(path)
        .map_err(|e| AppError::internal(format!("Failed to read {}: {}", path.display(), e)))
}

/// 读取 JSON 文件
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AppError> {
    let content = read_file(path)?;
    serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse JSON from {}: {}", path.display(), e)))
}

// ── 目录操作 ────────────────────────────────────────────────────────────────

/// 确保目录存在
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(path)
        .map_err(|e| AppError::internal(format!("Failed to create directory {}: {}", path.display(), e)))
}

// ── 路径校验 ────────────────────────────────────────────────────────────────

/// 校验 ID 组件
///
/// 检查是否为空、是否过长、是否包含路径分隔符或穿越字符。
pub fn validate_id_component(component: &str, field_name: &str) -> Result<(), AppError> {
    if component.is_empty() {
        return Err(AppError::invalid_input(format!("{} cannot be empty", field_name)));
    }
    if component.len() > 255 {
        return Err(AppError::invalid_input(format!("{} too long (max 255 chars)", field_name)));
    }
    if component.contains('/') || component.contains('\\') || component.contains("..") {
        return Err(AppError::path_traversal());
    }
    Ok(())
}

/// 校验路径是否在根目录内
///
/// 防止路径穿越攻击：
/// - 拒绝任何含 `..` 组件的路径
/// - 使用 canonicalize 校验最终路径
pub fn validate_path_within_root(
    path: &Path,
    root: &Path,
    _field_name: &str,
) -> Result<std::path::PathBuf, AppError> {
    use std::path::Component;

    // 防御层 1：拒绝任何含 `..` 组件的路径
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::path_traversal());
    }

    let canonical_root = root.canonicalize()
        .map_err(|e| AppError::internal(format!("Failed to canonicalize root: {}", e)))?;

    // 文件已存在：直接 canonicalize 后与 root 比对
    if let Ok(canonical_path) = path.canonicalize() {
        if !canonical_path.starts_with(&canonical_root) {
            return Err(AppError::path_traversal());
        }
        return Ok(canonical_path);
    }

    // 文件不存在：canonicalize 最近存在的祖先目录
    let mut suffix: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        match current.canonicalize() {
            Ok(canonical_ancestor) => {
                if !canonical_ancestor.starts_with(&canonical_root) {
                    return Err(AppError::path_traversal());
                }
                let mut result = canonical_ancestor;
                for part in suffix.into_iter().rev() {
                    result.push(part);
                }
                return Ok(result);
            }
            Err(_) => {
                match current.file_name().map(std::ffi::OsString::from) {
                    Some(name) => {
                        suffix.push(name);
                        match current.parent() {
                            Some(parent) => current = parent.to_path_buf(),
                            None => break,
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Err(AppError::path_traversal())
}

// ── 测试模块 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_write() {
        let dir = std::env::temp_dir().join("mnemosyne_test_atomic");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.txt");

        atomic_write(&path, b"hello world").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");

        atomic_write(&path, b"updated").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "updated");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_id_component_ok() {
        assert!(validate_id_component("abc-123", "test").is_ok());
        assert!(validate_id_component("uuid-v4-format", "test").is_ok());
        assert!(validate_id_component("a", "test").is_ok());
    }

    #[test]
    fn test_validate_id_component_empty() {
        assert!(validate_id_component("", "test").is_err());
    }

    #[test]
    fn test_validate_id_component_too_long() {
        let long = "a".repeat(256);
        assert!(validate_id_component(&long, "test").is_err());
    }

    #[test]
    fn test_validate_id_component_slash() {
        assert!(validate_id_component("a/b", "test").is_err());
    }

    #[test]
    fn test_validate_id_component_backslash() {
        assert!(validate_id_component("a\\b", "test").is_err());
    }

    #[test]
    fn test_validate_id_component_dotdot() {
        assert!(validate_id_component("a..b", "test").is_err());
        assert!(validate_id_component("../etc/passwd", "test").is_err());
    }

    #[test]
    fn test_validate_path_within_root_ok() {
        let dir = std::env::temp_dir().join("mnemosyne_test_pathval");
        let _ = fs::create_dir_all(&dir);
        let sub = dir.join("subdir");
        let _ = fs::create_dir_all(&sub);

        let result = validate_path_within_root(&sub, &dir, "test");
        assert!(result.is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_path_within_root_traversal() {
        let dir = std::env::temp_dir().join("mnemosyne_test_pathval2");
        let _ = fs::create_dir_all(&dir);
        let outside = dir.join("..").join("other");

        let result = validate_path_within_root(&outside, &dir, "test");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_path_within_root_nonexistent_inside() {
        let dir = std::env::temp_dir().join("mnemosyne_test_pathval3");
        let _ = fs::create_dir_all(&dir);
        let nonexistent = dir.join("subdir").join("file.txt");

        let result = validate_path_within_root(&nonexistent, &dir, "test");
        assert!(result.is_ok(), "non-existent path inside root should pass");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_path_within_root_rejects_parent_dir_component() {
        let dir = std::env::temp_dir().join("mnemosyne_test_pathval4");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("a"));
        let _ = fs::create_dir_all(dir.join("b"));
        let tricky = dir.join("a").join("..").join("b");

        let result = validate_path_within_root(&tricky, &dir, "test");
        assert!(result.is_err(), "path with `..` component must be rejected");

        let _ = fs::remove_dir_all(&dir);
    }
}