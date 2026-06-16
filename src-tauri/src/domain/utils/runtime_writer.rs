//! Runtime writer utilities for writing state changes to truth files.

use std::path::Path;

/// Write a truth file with backup
pub fn write_truth_file_with_backup(
    path: &Path,
    content: &str,
    backup_dir: Option<&Path>,
) -> Result<(), crate::errors::AppError> {
    // Create backup if requested
    if let Some(backup) = backup_dir {
        if path.exists() {
            std::fs::create_dir_all(backup)
                .map_err(|e| crate::errors::AppError::internal(format!("Failed to create backup dir: {}", e)))?;
            let backup_path = backup.join(path.file_name().unwrap_or_default());
            std::fs::copy(path, &backup_path)
                .map_err(|e| crate::errors::AppError::internal(format!("Failed to backup: {}", e)))?;
        }
    }

    // Write the file
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::errors::AppError::internal(format!("Failed to create dir: {}", e)))?;
    }
    std::fs::write(path, content)
        .map_err(|e| crate::errors::AppError::internal(format!("Failed to write: {}", e)))?;

    Ok(())
}

/// Append content to a file
pub fn append_to_file(path: &Path, content: &str) -> Result<(), crate::errors::AppError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| crate::errors::AppError::internal(format!("Failed to open: {}", e)))?;
    write!(file, "{}", content)
        .map_err(|e| crate::errors::AppError::internal(format!("Failed to write: {}", e)))?;
    Ok(())
}
