//! ═══════════════════════════════════════════════════════════════════════════
//! path - 路径验证模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Component, Path, PathBuf};
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
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Path is empty",
            "path_validation: rejected empty path"
        );
        return Err(AppError::invalid_input("Path is empty"));
    }

    if path_str.len() > MAX_PATH_LENGTH {
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Path exceeds maximum length",
            path_length = path_str.len(),
            max_length = MAX_PATH_LENGTH,
            "path_validation: rejected path too long"
        );
        return Err(AppError::value_out_of_range(format!(
            "Path exceeds maximum length of {} characters",
            MAX_PATH_LENGTH
        )));
    }

    if path_str.contains("\r") || path_str.contains("\n") {
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Path contains CRLF characters",
            "path_validation: rejected CRLF in path"
        );
        return Err(AppError::invalid_input("Path contains CRLF characters"));
    }

    if path_str.chars().any(|c| c.is_control()) {
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Path contains control characters",
            "path_validation: rejected control chars in path"
        );
        return Err(AppError::invalid_input("Path contains control characters"));
    }

    // High 8: 用 Component::ParentDir 精确检测路径遍历,
    // 避免 contains("..") 误判含 ".." 的合法文件名（如 "..bar.txt"）
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Path traversal detected",
            "path_validation: rejected path traversal"
        );
        return Err(AppError::path_traversal());
    }

    let canonical = path.canonicalize().map_err(|e| {
        tracing::error!(
            operation = "validate_path",
            decision = "Deny",
            reason = "Failed to canonicalize path",
            error = ?e,
            "path_validation: canonicalize failed"
        );
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::file_not_found(path_str.to_string())
        } else {
            AppError::invalid_path(path_str.to_string())
        }
    })?;

    if let Some(base_dir) = base {
        let canonical_base = base_dir.canonicalize().map_err(|e| {
            tracing::error!(
                operation = "validate_path",
                decision = "Deny",
                reason = "Failed to canonicalize base directory",
                error = ?e,
                "path_validation: base canonicalize failed"
            );
            AppError::internal(format!("Failed to canonicalize base directory: {}", e))
        })?;

        if !canonical.starts_with(&canonical_base) {
            tracing::error!(
                operation = "validate_path",
                decision = "Deny",
                reason = "Path escapes base directory",
                "path_validation: rejected path escape"
            );
            return Err(AppError::path_traversal());
        }
    }

    tracing::warn!(
        operation = "validate_path",
        decision = "Allow",
        has_base = base.is_some(),
        "path_validation: path validated"
    );

    Ok(CanonicalPath(canonical))
}

pub fn validate_path_with_base(path: &Path, base: &Path) -> Result<CanonicalPath, AppError> {
    validate_path(path, Some(base))
}

pub fn validate_path_for_creation(path: &Path, base: &Path) -> Result<CanonicalPath, AppError> {
    let path_str = path.to_string_lossy();

    if path_str.is_empty() {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Path is empty",
            "path_validation: rejected empty path for creation"
        );
        return Err(AppError::invalid_input("Path is empty"));
    }

    if path_str.len() > MAX_PATH_LENGTH {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Path exceeds maximum length",
            path_length = path_str.len(),
            max_length = MAX_PATH_LENGTH,
            "path_validation: rejected path too long for creation"
        );
        return Err(AppError::value_out_of_range(format!(
            "Path exceeds maximum length of {} characters",
            MAX_PATH_LENGTH
        )));
    }

    if path_str.contains("\r") || path_str.contains("\n") {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Path contains CRLF characters",
            "path_validation: rejected CRLF in path for creation"
        );
        return Err(AppError::invalid_input("Path contains CRLF characters"));
    }

    if path_str.chars().any(|c| c.is_control()) {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Path contains control characters",
            "path_validation: rejected control chars in path for creation"
        );
        return Err(AppError::invalid_input("Path contains control characters"));
    }

    // High 8: 用 Component::ParentDir 精确检测路径遍历,
    // 避免 contains("..") 误判含 ".." 的合法文件名（如 "..bar.txt"）
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Path traversal detected",
            "path_validation: rejected path traversal for creation"
        );
        return Err(AppError::path_traversal());
    }

    let canonical_base = base.canonicalize().map_err(|e| {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Failed to canonicalize base directory",
            error = ?e,
            "path_validation: base canonicalize failed for creation"
        );
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
            Component::ParentDir => {
                if !resolved.pop() {
                    tracing::error!(
                        operation = "validate_path_for_creation",
                        decision = "Deny",
                        reason = "Path traversal detected during normalization",
                        "path_validation: rejected traversal during normalization"
                    );
                    return Err(AppError::path_traversal());
                }
            }
            Component::CurDir => {}
            _ => {
                resolved.push(component);
            }
        }
    }

    if !resolved.starts_with(&canonical_base) {
        tracing::error!(
            operation = "validate_path_for_creation",
            decision = "Deny",
            reason = "Normalized path escapes base directory",
            "path_validation: rejected escaped path for creation"
        );
        return Err(AppError::path_traversal());
    }

    tracing::warn!(
        operation = "validate_path_for_creation",
        decision = "Allow",
        "path_validation: path validated for creation"
    );

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