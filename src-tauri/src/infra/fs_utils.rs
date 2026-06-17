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

        // Overwrite
        atomic_write(&path, b"updated").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "updated");

        let _ = fs::remove_dir_all(&dir);
    }
}
