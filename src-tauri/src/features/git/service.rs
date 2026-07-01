use std::path::Path;

use crate::shared::errors::AppError;

use super::detector::git_executable;
use super::models::{
    Commit, Diff, FileChange, FileDiff, GitConfig, GitInitResult, GitStatus, RollbackMode,
};

/// Core git business operations.
///
/// All methods run `git` directly via `tokio::process::Command` and never go
/// through the agent bash sandbox — the workspace path is supplied by the
/// IPC layer which has already validated it.
pub struct GitService;

const MESSAGE_PLACEHOLDER: &str = "{message}";

impl GitService {
    /// Initialize a git repository at `path` if one does not already exist.
    /// Returns `initialized: false` when `path/.git` already exists.
    pub async fn init(path: &Path) -> Result<GitInitResult, AppError> {
        if path.join(".git").exists() {
            return Ok(GitInitResult {
                initialized: false,
                path: path.to_string_lossy().to_string(),
            });
        }
        run_git(path, &["init"]).await?;
        tracing::info!(path = %path.display(), "Git repository initialized");
        Ok(GitInitResult {
            initialized: true,
            path: path.to_string_lossy().to_string(),
        })
    }

    /// Snapshot the working tree state (branch, staged/unstaged/untracked).
    pub async fn status(path: &Path) -> Result<GitStatus, AppError> {
        let output = run_git(path, &["status", "--porcelain=v1", "-b"]).await?;
        Ok(parse_status(&output))
    }

    /// Return the most recent `limit` commits (newest first).
    pub async fn log(path: &Path, limit: u32) -> Result<Vec<Commit>, AppError> {
        let limit_clamped = limit.clamp(1, 1000);
        let limit_arg = format!("-n{}", limit_clamped);
        // ASCII unit separator (\x1f) is unlikely to appear in author names or
        // commit subjects, so it is a safe field delimiter.
        let pretty_arg = "--pretty=format:%H\x1f%h\x1f%an\x1f%ae\x1f%aI\x1f%s";
        let args: [&str; 3] = ["log", limit_arg.as_str(), pretty_arg];
        let output = run_git(path, &args).await?;
        Ok(parse_log(&output))
    }

    /// Compute a diff. When `commit_hash` is `None`, shows all uncommitted
    /// changes against `HEAD`. When `Some(hash)`, shows the changes introduced
    /// by that commit (handles the root commit gracefully via `git show`).
    pub async fn diff(path: &Path, commit_hash: Option<&str>) -> Result<Diff, AppError> {
        let numstat = if let Some(hash) = commit_hash {
            run_git(path, &["show", "--numstat", "--pretty=format:", hash]).await?
        } else {
            run_git(path, &["diff", "--numstat", "HEAD"]).await?
        };

        let patch = if let Some(hash) = commit_hash {
            let full = run_git(path, &["show", hash]).await?;
            // `git show` prepends commit metadata; cut everything before the
            // first per-file diff so the parser only sees the unified diff.
            match full.find("diff --git") {
                Some(idx) => full[idx..].to_string(),
                None => String::new(),
            }
        } else {
            run_git(path, &["diff", "HEAD"]).await?
        };

        Ok(parse_diff(&numstat, &patch))
    }

    /// Stage one or more paths. Pass `["."]` to stage everything.
    pub async fn stage(path: &Path, files: &[String]) -> Result<(), AppError> {
        if files.is_empty() {
            return Err(AppError::invalid_input("No files to stage"));
        }
        let mut args: Vec<String> = vec!["add".to_string(), "--".to_string()];
        args.extend(files.iter().cloned());
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_git(path, &arg_refs).await?;
        Ok(())
    }

    /// Create a commit. If `GitConfig.auto_stage` is enabled, runs `git add -A`
    /// first. If a `commit_message_template` is configured, the `{message}`
    /// placeholder inside it is replaced with `message`.
    /// Returns the new commit's full hash.
    pub async fn commit(path: &Path, message: &str) -> Result<String, AppError> {
        let config = Self::get_config(path).await?;

        if config.auto_stage {
            run_git(path, &["add", "-A"]).await?;
        }

        let actual_message = match &config.commit_message_template {
            Some(template) if template.contains(MESSAGE_PLACEHOLDER) => {
                template.replace(MESSAGE_PLACEHOLDER, message)
            }
            Some(template) if !template.is_empty() => template.clone(),
            _ => message.to_string(),
        };

        run_git(path, &["commit", "-m", &actual_message]).await?;

        let hash = run_git(path, &["rev-parse", "HEAD"]).await?;
        let hash = hash.trim().to_string();
        tracing::info!(hash = %hash, "Commit created");
        Ok(hash)
    }

    /// Reset `HEAD` to `commit_hash` using the given `mode`.
    /// `Hard` mode discards working-tree changes — the IPC layer is
    /// responsible for confirming with the user before invoking this.
    pub async fn rollback(
        path: &Path,
        commit_hash: &str,
        mode: RollbackMode,
    ) -> Result<(), AppError> {
        validate_commit_hash(commit_hash)?;

        let flag = match mode {
            RollbackMode::Soft => "--soft",
            RollbackMode::Hard => {
                tracing::warn!(
                    commit_hash,
                    "Performing hard rollback — working tree changes will be discarded"
                );
                "--hard"
            }
        };

        run_git(path, &["reset", flag, commit_hash]).await?;
        Ok(())
    }

    /// Read the per-repository git configuration. Custom keys are namespaced
    /// under `mnemosyne.*` so they do not collide with git's built-ins.
    pub async fn get_config(path: &Path) -> Result<GitConfig, AppError> {
        let user_name = run_git_optional(path, &["config", "user.name"]).await?;
        let user_email = run_git_optional(path, &["config", "user.email"]).await?;

        let auto_stage = run_git_optional(path, &["config", "--bool", "mnemosyne.autoStage"])
            .await?
            .map(|v| v.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let commit_message_template =
            run_git_optional(path, &["config", "mnemosyne.commitMessageTemplate"]).await?;

        let enable_remote = run_git_optional(path, &["config", "--bool", "mnemosyne.enableRemote"])
            .await?
            .map(|v| v.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(GitConfig {
            user_name,
            user_email,
            auto_stage,
            commit_message_template,
            enable_remote,
        })
    }

    /// Persist the per-repository git configuration. Writes only to the local
    /// repo (no `--global`).
    pub async fn set_config(path: &Path, config: &GitConfig) -> Result<(), AppError> {
        if let Some(name) = &config.user_name {
            run_git(path, &["config", "user.name", name]).await?;
        }
        if let Some(email) = &config.user_email {
            run_git(path, &["config", "user.email", email]).await?;
        }

        let auto_stage_str = if config.auto_stage { "true" } else { "false" };
        run_git(path, &["config", "mnemosyne.autoStage", auto_stage_str]).await?;

        match &config.commit_message_template {
            Some(template) if !template.is_empty() => {
                run_git(path, &["config", "mnemosyne.commitMessageTemplate", template]).await?;
            }
            _ => {
                // Unset silently if it doesn't exist.
                let _ = run_git_optional(path, &["config", "--unset", "mnemosyne.commitMessageTemplate"])
                    .await;
            }
        }

        let enable_remote_str = if config.enable_remote { "true" } else { "false" };
        run_git(path, &["config", "mnemosyne.enableRemote", enable_remote_str]).await?;

        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Run a git command in `path` and return its stdout. Non-zero exit becomes
/// `AppError::internal` with the (sanitized) stderr — never exposes absolute
/// paths from the workspace root.
async fn run_git(path: &Path, args: &[&str]) -> Result<String, AppError> {
    let output = tokio::process::Command::new(git_executable())
        .current_dir(path)
        .args(args)
        .output()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to spawn git");
            AppError::internal("Failed to execute git command")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = pick_first_nonempty(&[&stderr, &stdout]);
        tracing::warn!(
            args = ?args,
            exit = ?output.status.code(),
            stderr = %detail,
            "Git command failed"
        );
        // Avoid echoing absolute paths back to the frontend verbatim.
        let sanitized = detail.lines().next().unwrap_or("git error").to_string();
        return Err(AppError::internal(format!("Git command failed: {}", sanitized)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Like `run_git` but treats a non-zero exit (e.g. missing config key) as
/// `Ok(None)` instead of an error. Empty stdout also maps to `None`.
async fn run_git_optional(path: &Path, args: &[&str]) -> Result<Option<String>, AppError> {
    let output = tokio::process::Command::new(git_executable())
        .current_dir(path)
        .args(args)
        .output()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to spawn git");
            AppError::internal("Failed to execute git command")
        })?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn pick_first_nonempty<'a>(candidates: &[&'a str]) -> &'a str {
    for c in candidates {
        let trimmed = c.trim();
        if !trimmed.is_empty() {
            return c;
        }
    }
    ""
}

/// Rejects anything that is not a hex string of length 4..=40 — short
/// hashes (e.g. `abc1234`) and full SHA-1 hashes (`40` hex chars) are allowed.
fn validate_commit_hash(hash: &str) -> Result<(), AppError> {
    if hash.is_empty() {
        return Err(AppError::invalid_input("Commit hash cannot be empty"));
    }
    if hash.len() < 4 || hash.len() > 40 {
        return Err(AppError::invalid_input("Invalid commit hash length"));
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::invalid_input("Commit hash must be hexadecimal"));
    }
    Ok(())
}

/// Map a porcelain status code character to a stable status name.
fn parse_status_char(c: char) -> &'static str {
    match c {
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        'U' => "unmerged",
        _ => "modified",
    }
}

/// Parse `git status --porcelain=v1 -b` output into a `GitStatus`.
fn parse_status(output: &str) -> GitStatus {
    let mut branch = String::new();
    let mut staged: Vec<FileChange> = Vec::new();
    let mut unstaged: Vec<FileChange> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // "## main" or "## main...origin/main [ahead 1]" or "## No commits yet on main"
            let after = rest.strip_prefix("No commits yet on ").unwrap_or(rest);
            let name = after
                .split("...")
                .next()
                .unwrap_or(after)
                .split(' ')
                .next()
                .unwrap_or(after);
            branch = name.to_string();
            continue;
        }

        if let Some(rest) = line.strip_prefix("?? ") {
            untracked.push(rest.to_string());
            continue;
        }

        let bytes = line.as_bytes();
        if bytes.len() < 3 {
            continue;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path_part = &line[3..];

        // Renames in porcelain v1 look like "old -> new"; keep the new name.
        let resolved = match path_part.find(" -> ") {
            Some(idx) => path_part[idx + 4..].to_string(),
            None => path_part.to_string(),
        };

        if x != ' ' && x != '?' {
            staged.push(FileChange {
                path: resolved.clone(),
                status: parse_status_char(x).to_string(),
                staged: true,
            });
        }
        if y != ' ' && y != '?' {
            unstaged.push(FileChange {
                path: resolved,
                status: parse_status_char(y).to_string(),
                staged: false,
            });
        }
    }

    let is_clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();

    GitStatus {
        branch,
        staged,
        unstaged,
        untracked,
        is_clean,
    }
}

/// Parse `git log --pretty=format:` output delimited by ASCII unit separator.
fn parse_log(output: &str) -> Vec<Commit> {
    let mut commits = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(6, '\x1f').collect();
        if parts.len() < 6 {
            continue;
        }
        commits.push(Commit {
            hash: parts[0].to_string(),
            short_hash: parts[1].to_string(),
            author: parts[2].to_string(),
            email: parts[3].to_string(),
            date: parts[4].to_string(),
            message: parts[5].to_string(),
        });
    }
    commits
}

/// Parse `git diff --numstat` and the matching unified diff into per-file
/// `FileDiff` records, attaching each file's patch slice to its entry.
fn parse_diff(numstat: &str, patch: &str) -> Diff {
    // numstat format: "additions\tdeletions\tpath"  (renames: "0\t0\told => new")
    let mut files: Vec<FileDiff> = Vec::new();
    for line in numstat.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let additions = parts[0].parse::<u32>().unwrap_or(0);
        let deletions = parts[1].parse::<u32>().unwrap_or(0);
        let raw_path = parts[2];
        let path = match raw_path.find("=>") {
            Some(idx) => raw_path[idx + 2..].trim().trim_matches('"').to_string(),
            None => raw_path.to_string(),
        };
        files.push(FileDiff {
            path,
            additions,
            deletions,
            patch: String::new(),
        });
    }

    // Walk the unified diff, slicing per-file patches at "diff --git" boundaries.
    let mut current_path: Option<String> = None;
    let mut current_patch = String::new();
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            if let Some(p) = current_path.take() {
                if let Some(fd) = files.iter_mut().find(|f| f.path == p) {
                    fd.patch = std::mem::take(&mut current_patch);
                }
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Standard format: `diff --git a/<src> b/<dst>` — parts[3] is the
            // b-side path. Fall back to parts[2] for non-standard headers.
            let b_part = parts.get(3).or_else(|| parts.get(2));
            if let Some(b_raw) = b_part {
                let b_path = b_raw.strip_prefix("b/").unwrap_or(b_raw);
                current_path = Some(b_path.to_string());
            }
        }
        if current_path.is_some() {
            current_patch.push_str(line);
            current_patch.push('\n');
        }
    }
    if let Some(p) = current_path.take() {
        if let Some(fd) = files.iter_mut().find(|f| f.path == p) {
            fd.patch = current_patch;
        }
    }

    Diff { files }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_commit_hash_valid_short() {
        assert!(validate_commit_hash("abc1234").is_ok());
    }

    #[test]
    fn test_validate_commit_hash_valid_full() {
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert!(validate_commit_hash(hash).is_ok());
    }

    #[test]
    fn test_validate_commit_hash_too_short() {
        assert!(validate_commit_hash("ab").is_err());
    }

    #[test]
    fn test_validate_commit_hash_too_long() {
        let hash = "0123456789abcdef0123456789abcdef0123456789";
        assert!(validate_commit_hash(hash).is_err());
    }

    #[test]
    fn test_validate_commit_hash_non_hex() {
        assert!(validate_commit_hash("xyz1234").is_err());
    }

    #[test]
    fn test_validate_commit_hash_empty() {
        assert!(validate_commit_hash("").is_err());
    }

    #[test]
    fn test_parse_status_clean() {
        let out = "## main\n";
        let s = parse_status(out);
        assert_eq!(s.branch, "main");
        assert!(s.is_clean);
    }

    #[test]
    fn test_parse_status_with_changes() {
        let out = "## main\nM  staged_modified.txt\n M unstaged_modified.txt\nA  staged_added.txt\nD  staged_deleted.txt\n?? untracked.txt\n";
        let s = parse_status(out);
        assert_eq!(s.branch, "main");
        assert!(!s.is_clean);
        assert_eq!(s.staged.len(), 3);
        assert_eq!(s.unstaged.len(), 1);
        assert_eq!(s.untracked.len(), 1);
        assert_eq!(s.untracked[0], "untracked.txt");
        assert_eq!(s.unstaged[0].path, "unstaged_modified.txt");
        assert_eq!(s.unstaged[0].status, "modified");
    }

    #[test]
    fn test_parse_status_no_commits_yet() {
        let out = "## No commits yet on main\n\
                   ?? new_file.txt\n";
        let s = parse_status(out);
        assert_eq!(s.branch, "main");
        assert_eq!(s.untracked.len(), 1);
    }

    #[test]
    fn test_parse_status_rename() {
        let out = "## main\nR  old_name.txt -> new_name.txt\n";
        let s = parse_status(out);
        assert_eq!(s.staged.len(), 1);
        assert_eq!(s.staged[0].path, "new_name.txt");
        assert_eq!(s.staged[0].status, "renamed");
    }

    #[test]
    fn test_parse_status_branch_with_upstream() {
        let out = "## main...origin/main [ahead 1]\n";
        let s = parse_status(out);
        assert_eq!(s.branch, "main");
    }

    #[test]
    fn test_parse_log_basic() {
        let out = "abc123def456789abc123def456789abc123def4\x1fabc123d\x1fAlice\x1falice@example.com\x1f2024-01-15T10:30:00+08:00\x1fInitial commit";
        let commits = parse_log(out);
        assert_eq!(commits.len(), 1);
        let c = &commits[0];
        assert_eq!(c.short_hash, "abc123d");
        assert_eq!(c.author, "Alice");
        assert_eq!(c.email, "alice@example.com");
        assert_eq!(c.message, "Initial commit");
    }

    #[test]
    fn test_parse_log_message_with_separator() {
        // Message containing the unit separator should still parse via splitn.
        let out = "hash\x1fshort\x1fBob\x1fbob@example.com\x1f2024-01-15T10:30:00+08:00\x1fFix bug | important";
        let commits = parse_log(out);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "Fix bug | important");
    }

    #[test]
    fn test_parse_log_multiple_commits() {
        let out = "h1\x1fs1\x1fA\x1fa@x\x1f2024-01-01T00:00:00+08:00\x1fone\n\
                   h2\x1fs2\x1fB\x1fb@x\x1f2024-01-02T00:00:00+08:00\x1ftwo";
        let commits = parse_log(out);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "one");
        assert_eq!(commits[1].message, "two");
    }

    #[test]
    fn test_parse_diff_numstat_only() {
        let numstat = "5\t2\tsrc/main.rs\n3\t0\tREADME.md\n";
        let patch = "";
        let d = parse_diff(numstat, patch);
        assert_eq!(d.files.len(), 2);
        assert_eq!(d.files[0].path, "src/main.rs");
        assert_eq!(d.files[0].additions, 5);
        assert_eq!(d.files[0].deletions, 2);
        assert_eq!(d.files[1].path, "README.md");
        assert_eq!(d.files[1].additions, 3);
        assert_eq!(d.files[1].deletions, 0);
    }

    #[test]
    fn test_parse_diff_with_patch() {
        let numstat = "1\t1\tsrc/main.rs\n";
        let patch = "diff --git a/src/main.rs b/src/main.rs\nindex 123..456 789\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,2 +1,2 @@\n-old line\n+new line\n";
        let d = parse_diff(numstat, patch);
        assert_eq!(d.files.len(), 1);
        let f = &d.files[0];
        assert_eq!(f.path, "src/main.rs");
        assert!(f.patch.contains("diff --git a/src/main.rs b/src/main.rs"));
        assert!(f.patch.contains("+new line"));
        assert!(f.patch.contains("-old line"));
    }

    #[test]
    fn test_parse_diff_rename_in_numstat() {
        let numstat = "0\t0\told_name.txt => new_name.txt\n";
        let d = parse_diff(numstat, "");
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].path, "new_name.txt");
    }

    #[test]
    fn test_parse_diff_binary_file() {
        // Binary files produce "-\t-\tpath" in numstat.
        let numstat = "-\t-\timage.png\n";
        let d = parse_diff(numstat, "");
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].additions, 0);
        assert_eq!(d.files[0].deletions, 0);
    }

    #[test]
    fn test_parse_status_char_mapping() {
        assert_eq!(parse_status_char('M'), "modified");
        assert_eq!(parse_status_char('A'), "added");
        assert_eq!(parse_status_char('D'), "deleted");
        assert_eq!(parse_status_char('R'), "renamed");
        assert_eq!(parse_status_char('C'), "copied");
        assert_eq!(parse_status_char('U'), "unmerged");
        assert_eq!(parse_status_char('X'), "modified"); // fallback
    }
}
