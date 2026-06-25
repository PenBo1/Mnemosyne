use std::path::Path;
use crate::errors::AppError;

/// Atomic file write using temp-and-rename pattern.
/// Prevents corruption if the process crashes mid-write.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let dir = path.parent()
        .ok_or_else(|| AppError::internal("Cannot determine parent directory"))?;

    // Create temp file in same directory (same filesystem for atomic rename)
    let temp_name = format!("{}.tmp.{}", path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file"),
        std::process::id()
    );
    let temp_path = dir.join(&temp_name);

    std::fs::write(&temp_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write temp file: {}", e)))?;

    // Atomic rename
    std::fs::rename(&temp_path, path)
        .map_err(|e| {
            // Clean up temp file on failure
            let _ = std::fs::remove_file(&temp_path);
            AppError::internal(format!("Failed to rename temp file: {}", e))
        })?;

    Ok(())
}

/// Atomic JSON write with pretty printing.
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| AppError::internal(format!("Failed to serialize JSON: {}", e)))?;
    atomic_write(path, json.as_bytes())
}

/// Read file content with error mapping.
pub fn read_file(path: &Path) -> Result<String, AppError> {
    std::fs::read_to_string(path)
        .map_err(|e| AppError::internal(format!("Failed to read {}: {}", path.display(), e)))
}

/// Read JSON from file.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AppError> {
    let content = read_file(path)?;
    serde_json::from_str(&content)
        .map_err(|e| AppError::internal(format!("Failed to parse JSON from {}: {}", path.display(), e)))
}

/// Ensure directory exists.
pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(path)
        .map_err(|e| AppError::internal(format!("Failed to create directory {}: {}", path.display(), e)))
}

/// Validate that a user-supplied component (book_id, chapter file prefix, etc.)
/// contains no path separators or traversal sequences.
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

/// Validate that a resolved path is within an allowed root directory.
/// Returns the canonicalized path on success.
pub fn validate_path_within_root(
    path: &Path,
    root: &Path,
    _field_name: &str,
) -> Result<std::path::PathBuf, AppError> {
    let canonical_root = root.canonicalize()
        .map_err(|e| AppError::internal(format!("Failed to canonicalize root: {}", e)))?;
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !canonical_path.starts_with(&canonical_root) {
        return Err(AppError::path_traversal());
    }
    Ok(canonical_path)
}

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
}
