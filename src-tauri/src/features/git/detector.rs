/// Detector for the system `git` executable.
///
/// Runs `git --version` via `tokio::process::Command` and parses the version
/// string. Returns `None` when git is not installed or the command fails —
/// never panics.

/// Returns the git version string (e.g. "git version 2.43.0") if git is
/// installed and reachable on PATH, otherwise `None`.
pub async fn detect_git() -> Option<String> {
    let output = tokio::process::Command::new(git_executable())
        .arg("--version")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        tracing::debug!(exit = ?output.status.code(), "git --version exited non-zero");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Returns the git executable name. Hardcoded to `"git"` for now; future
/// configurations may allow overriding this (e.g. absolute path).
pub fn git_executable() -> String {
    "git".to_string()
}

/// Extracts the bare version number from a `git --version` output line.
/// Input example: `"git version 2.43.0.windows.1"` → `Some("2.43.0.windows.1")`.
pub fn parse_version(version_output: &str) -> Option<String> {
    let trimmed = version_output.trim();
    let prefix = "git version ";
    if let Some(rest) = trimmed.strip_prefix(prefix) {
        let v = rest.trim();
        if v.is_empty() {
            None
        } else {
            Some(v.to_string())
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_standard() {
        assert_eq!(
            parse_version("git version 2.43.0"),
            Some("2.43.0".to_string())
        );
    }

    #[test]
    fn test_parse_version_windows() {
        assert_eq!(
            parse_version("git version 2.43.0.windows.1"),
            Some("2.43.0.windows.1".to_string())
        );
    }

    #[test]
    fn test_parse_version_invalid() {
        assert_eq!(parse_version("not a git output"), None);
    }

    #[test]
    fn test_parse_version_empty() {
        assert_eq!(parse_version("git version "), None);
    }

    #[test]
    fn test_git_executable_is_git() {
        assert_eq!(git_executable(), "git");
    }
}
