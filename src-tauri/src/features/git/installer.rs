use super::detector::{detect_git, parse_version};
use super::models::InstallResult;

/// Attempts to install git on the current platform using the platform's
/// native package manager. After installation, re-runs `detect_git()` to
/// verify success and return the version.
///
/// Platform behaviour:
/// - **Windows**: `winget install --id Git.Git -e --source winget`
/// - **macOS**: `brew install git` (fails if Homebrew is not installed)
/// - **Linux**: `sudo apt-get install -y git` (may fail without sudo rights)
///
/// All commands run via `tokio::process::Command`, capturing stdout/stderr.
pub async fn install_git() -> InstallResult {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "windows") {
        ("winget", vec!["install", "--id", "Git.Git", "-e", "--source", "winget", "--accept-source-agreements", "--accept-package-agreements"])
    } else if cfg!(target_os = "macos") {
        ("brew", vec!["install", "git"])
    } else {
        ("sudo", vec!["apt-get", "install", "-y", "git"])
    };

    tracing::info!(platform = std::env::consts::OS, program, "Attempting git installation");

    let output = match tokio::process::Command::new(program)
        .args(&args)
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, program, "Installer executable not found");
            return InstallResult {
                success: false,
                message: format!(
                    "Installer '{}' not available. Please install git manually.",
                    program
                ),
                version: None,
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("'{}' exited with code {:?}", program, output.status.code())
        };
        tracing::warn!(program, exit = ?output.status.code(), detail = %detail, "Git installer failed");
        return InstallResult {
            success: false,
            message: format!(
                "Installation failed: {}. Please install git manually.",
                detail
            ),
            version: None,
        };
    }

    // Verify installation succeeded by re-detecting.
    match detect_git().await {
        Some(version_str) => {
            let parsed = parse_version(&version_str);
            tracing::info!(version = ?parsed, "Git installed successfully");
            InstallResult {
                success: true,
                message: "Git installed successfully".to_string(),
                version: parsed,
            }
        }
        None => {
            tracing::warn!("Installer exited successfully but git not detected on PATH");
            InstallResult {
                success: false,
                message: "Installer reported success but git is not on PATH. \
                          Please restart your shell or add git to PATH manually."
                    .to_string(),
                version: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_result_default_fields() {
        let r = InstallResult {
            success: false,
            message: "noop".to_string(),
            version: None,
        };
        assert!(!r.success);
        assert!(r.version.is_none());
    }
}
