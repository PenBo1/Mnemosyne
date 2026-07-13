use std::path::{Path, PathBuf};
use crate::shared::error::AppError;

const MAX_PATH_LENGTH: usize = 4096;

#[derive(Debug, Clone)]
pub struct CanonicalPath(PathBuf);

impl CanonicalPath {
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for CanonicalPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl std::fmt::Display for CanonicalPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

pub fn validate_path(path: &Path, base: Option<&Path>) -> Result<CanonicalPath, AppError> {
    let path_str = path.to_string_lossy();

    if path_str.is_empty() {
        return Err(AppError::invalid_input("Path is empty"));
    }

    if path_str.len() > MAX_PATH_LENGTH {
        return Err(AppError::value_out_of_range(format!(
            "Path exceeds maximum length of {} characters",
            MAX_PATH_LENGTH
        )));
    }

    if path_str.contains("\r") || path_str.contains("\n") {
        return Err(AppError::invalid_input("Path contains CRLF characters"));
    }

    if path_str.chars().any(|c| c.is_control()) {
        return Err(AppError::invalid_input("Path contains control characters"));
    }

    if path_str.contains("..") {
        return Err(AppError::path_traversal());
    }

    let canonical = path.canonicalize().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::file_not_found(path_str.to_string())
        } else {
            AppError::invalid_path(path_str.to_string())
        }
    })?;

    if let Some(base_dir) = base {
        let canonical_base = base_dir.canonicalize().map_err(|e| {
            AppError::internal(format!("Failed to canonicalize base directory: {}", e))
        })?;

        if !canonical.starts_with(&canonical_base) {
            return Err(AppError::path_traversal());
        }
    }

    Ok(CanonicalPath(canonical))
}

pub fn validate_path_with_base(path: &Path, base: &Path) -> Result<CanonicalPath, AppError> {
    validate_path(path, Some(base))
}

pub fn validate_path_for_creation(path: &Path, base: &Path) -> Result<CanonicalPath, AppError> {
    let path_str = path.to_string_lossy();

    if path_str.is_empty() {
        return Err(AppError::invalid_input("Path is empty"));
    }

    if path_str.len() > MAX_PATH_LENGTH {
        return Err(AppError::value_out_of_range(format!(
            "Path exceeds maximum length of {} characters",
            MAX_PATH_LENGTH
        )));
    }

    if path_str.contains("\r") || path_str.contains("\n") {
        return Err(AppError::invalid_input("Path contains CRLF characters"));
    }

    if path_str.chars().any(|c| c.is_control()) {
        return Err(AppError::invalid_input("Path contains control characters"));
    }

    if path_str.contains("..") {
        return Err(AppError::path_traversal());
    }

    let canonical_base = base.canonicalize().map_err(|e| {
        AppError::internal(format!("Failed to canonicalize base directory: {}", e))
    })?;

    let normalized = if path.is_absolute() {
        path.to_path_buf()
    } else {
        canonical_base.join(path)
    };

    let mut resolved = PathBuf::new();
    for component in normalized.components() {
        match component {
            std::path::Component::ParentDir { .. } => {
                if !resolved.pop() {
                    return Err(AppError::path_traversal());
                }
            }
            std::path::Component::CurDir => {}
            _ => {
                resolved.push(component);
            }
        }
    }

    if !resolved.starts_with(&canonical_base) {
        return Err(AppError::path_traversal());
    }

    Ok(CanonicalPath(resolved))
}

pub fn check_symlink_target(path: &Path, allowed_base: &Path) -> Result<(), AppError> {
    let metadata = path.symlink_metadata().map_err(|e| {
        AppError::internal(format!("Failed to read symlink metadata: {}", e))
    })?;

    if metadata.file_type().is_symlink() {
        let target = path.read_link().map_err(|e| {
            AppError::internal(format!("Failed to read symlink target: {}", e))
        })?;

        let canonical_target = if target.is_absolute() {
            target.canonicalize().map_err(|e| {
                AppError::internal(format!("Failed to canonicalize symlink target: {}", e))
            })?
        } else {
            path.parent()
                .ok_or_else(|| AppError::invalid_input("Invalid symlink parent directory"))?
                .join(&target)
                .canonicalize()
                .map_err(|e| {
                    AppError::internal(format!("Failed to canonicalize symlink target: {}", e))
                })?
        };

        let canonical_base = allowed_base.canonicalize().map_err(|e| {
            AppError::internal(format!("Failed to canonicalize allowed base: {}", e))
        })?;

        if !canonical_target.starts_with(&canonical_base) {
            return Err(AppError::forbidden(format!(
                "Symlink target escapes allowed directory: {}",
                canonical_target.display()
            )));
        }
    }

    Ok(())
}

pub fn is_within_base(path: &Path, base: &Path) -> Result<bool, AppError> {
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(false);
        }
        Err(e) => {
            return Err(AppError::internal(format!("Failed to canonicalize path: {}", e)));
        }
    };

    let canonical_base = base.canonicalize().map_err(|e| {
        AppError::internal(format!("Failed to canonicalize base: {}", e))
    })?;

    Ok(canonical_path.starts_with(&canonical_base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_empty() {
        let result = validate_path(Path::new(""), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        let result = validate_path(Path::new("../../../etc/passwd"), None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "PATH_TRAVERSAL");
    }

    #[test]
    fn test_validate_path_control_chars() {
        let result = validate_path(Path::new("/tmp/test\x00file"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_crlf() {
        let result = validate_path(Path::new("/tmp/test\r\nfile"), None);
        assert!(result.is_err());
    }
}