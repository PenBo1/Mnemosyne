use crate::shared::errors::AppError;

/// Project-level tools for file operations
pub struct ProjectTools;

impl ProjectTools {
    /// Read a file from the project
    pub fn read_file(project_root: &str, path: &str) -> Result<String, AppError> {
        let full_path = if Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            std::path::PathBuf::from(project_root).join(path)
        };

        // Guard against path traversal
        let canonical = full_path.canonicalize()
            .map_err(|e| AppError::internal(format!("Failed to resolve path: {}", e)))?;
        let project_canonical = std::path::PathBuf::from(project_root).canonicalize()
            .map_err(|e| AppError::internal(format!("Failed to resolve project root: {}", e)))?;

        if !canonical.starts_with(&project_canonical) {
            return Err(AppError::forbidden("Path traversal not allowed"));
        }

        std::fs::read_to_string(&canonical)
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))
    }

    /// Write a file to the project
    pub fn write_file(project_root: &str, path: &str, content: &str) -> Result<(), AppError> {
        let full_path = if Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            std::path::PathBuf::from(project_root).join(path)
        };

        let canonical = full_path.canonicalize()
            .map_err(|e| AppError::internal(format!("Failed to resolve path: {}", e)))?;
        let project_canonical = std::path::PathBuf::from(project_root).canonicalize()
            .map_err(|e| AppError::internal(format!("Failed to resolve project root: {}", e)))?;

        if !canonical.starts_with(&project_canonical) {
            return Err(AppError::forbidden("Path traversal not allowed"));
        }

        if let Some(parent) = canonical.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::internal(format!("Failed to create directory: {}", e)))?;
        }

        std::fs::write(&canonical, content)
            .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))
    }

    /// List files in a directory
    pub fn list_dir(project_root: &str, path: &str) -> Result<Vec<String>, AppError> {
        let full_path = std::path::PathBuf::from(project_root).join(path);
        let entries = std::fs::read_dir(&full_path)
            .map_err(|e| AppError::internal(format!("Failed to read directory: {}", e)))?;

        let files: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        Ok(files)
    }

    /// Search files using grep-like pattern
    pub fn grep(project_root: &str, pattern: &str, path: &str) -> Result<Vec<GrepResult>, AppError> {
        let search_path = std::path::PathBuf::from(project_root).join(path);
        let re = regex::Regex::new(pattern)
            .map_err(|e| AppError::internal(format!("Invalid regex: {}", e)))?;

        let mut results = Vec::new();
        let entries = std::fs::read_dir(&search_path)
            .map_err(|e| AppError::internal(format!("Failed to read directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (i, line) in content.lines().enumerate() {
                        if re.is_match(line) {
                            results.push(GrepResult {
                                file: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                                line: i + 1,
                                content: line.to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResult {
    pub file: String,
    pub line: usize,
    pub content: String,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_dir() {
        let result = ProjectTools::list_dir("/nonexistent", ".");
        assert!(result.is_err());
    }
}
