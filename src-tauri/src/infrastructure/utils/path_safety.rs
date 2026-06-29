//! Path safety utilities.

use std::path::{Path, PathBuf};

/// Resolve a child path safely, preventing traversal
pub fn safe_child_path(base: &Path, child: &str) -> Result<PathBuf, crate::shared::errors::AppError> {
    let base_canonical = base.canonicalize()
        .map_err(|e| crate::shared::errors::AppError::internal(format!("Failed to resolve base path: {}", e)))?;

    let child_path = base.join(child);
    let child_canonical = child_path.canonicalize()
        .unwrap_or_else(|_| child_path.clone());

    if !child_canonical.starts_with(&base_canonical) {
        return Err(crate::shared::errors::AppError::forbidden("Path traversal not allowed"));
    }

    Ok(child_path)
}

/// Check if a path is within a base directory
pub fn is_within_directory(base: &Path, path: &Path) -> bool {
    let base_canonical = base.canonicalize().ok();
    let path_canonical = path.canonicalize().ok();

    match (base_canonical, path_canonical) {
        (Some(base), Some(path)) => path.starts_with(&base),
        _ => false,
    }
}

/// Assert a safe truth file name
pub fn assert_safe_truth_file_name(name: &str) -> Result<String, crate::shared::errors::AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") || trimmed.contains('\0') {
        return Err(crate::shared::errors::AppError::bad_request(format!("Invalid truth file name: {}", name)));
    }
    Ok(if trimmed.ends_with(".md") { trimmed.to_string() } else { format!("{}.md", trimmed) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_safe_truth_file_name() {
        assert!(assert_safe_truth_file_name("test.md").is_ok());
        assert!(assert_safe_truth_file_name("test").is_ok());
        assert!(assert_safe_truth_file_name("../etc/passwd").is_err());
        assert!(assert_safe_truth_file_name("a/b").is_err());
    }
}
