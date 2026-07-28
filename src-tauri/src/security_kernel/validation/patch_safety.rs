//! ═══════════════════════════════════════════════════════════════════════════
//! patch_safety - 补丁安全检查模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ── 决策类型 ────────────────────────────────────────────────────

/// Patch safety 决策结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchSafetyDecision {
    /// 自动批准（受限 + 有沙箱）。
    AutoApprove,
    /// 询问用户（受限 + 无沙箱）。
    AskUser,
    /// 拒绝（路径逃逸或 hardlink 攻击）。
    Reject { reason: String },
}

impl PatchSafetyDecision {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::AutoApprove)
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Reject { .. })
    }
}

// ── 路径规范化 ──────────────────────────────────────────────────

/// 规范化路径：解析 `..` / `.` 并返回绝对路径。
///
/// 对照 codex `canonicalize_path_for_safety`。不解析符号链接（避免 TOCTOU），
/// 仅做词法规范化（`Path::lexical` 风格）。
///
/// - 若 path 是相对路径，基于 `base` 解析为绝对路径
/// - 解析 `.` 和 `..` 组件
/// - 不访问文件系统（纯词法操作）
pub fn normalize_path_lexical(path: &Path, base: &Path) -> PathBuf {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };

    // 词法规范化：逐组件处理 `.` 和 `..`
    let mut components: Vec<std::path::Component<'_>> = Vec::new();
    for comp in abs.components() {
        match comp {
            std::path::Component::CurDir => { /* 跳过 `.` */ }
            std::path::Component::ParentDir => {
                // 弹出上一个正常组件（若有）
                match components.last() {
                    Some(std::path::Component::Normal(_)) => {
                        components.pop();
                    }
                    _ => {
                        // 根目录的 `..` 或空栈的 `..` 跳过（与 std 规范化一致）
                    }
                }
            }
            other => {
                components.push(other);
            }
        }
    }

    components.iter().collect()
}

/// 检查 path 是否在 writable_roots 内（词法比较，不解析符号链接）。
///
/// 对照 codex `is_path_within_writable_roots`。规范化 path 与每个 root，
/// 检查 path 是否以某个 root 开头。
pub fn is_path_in_writable_roots(path: &Path, writable_roots: &[PathBuf]) -> bool {
    if writable_roots.is_empty() {
        return false;
    }
    let normalized_path = normalize_path_lexical(path, Path::new("."));
    writable_roots.iter().any(|root| {
        let normalized_root = normalize_path_lexical(root, Path::new("."));
        normalized_path.starts_with(&normalized_root)
    })
}

// ── 写补丁约束检查 ──────────────────────────────────────────────

/// 写补丁中的单个变更路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPathChange {
    /// 变更路径（可能是相对或绝对）。
    pub path: PathBuf,
    /// 是否为新建文件（true 时检查父目录权限）。
    pub is_new_file: bool,
}

/// 检查写补丁是否所有变更路径都约束在 writable_roots 内。
///
/// 对照 codex `is_write_patch_constrained_to_writable_paths`。
///
/// 返回 `Ok(())` 表示所有路径都在 writable_roots 内；
/// 返回 `Err(reason)` 表示有路径逃逸，reason 描述哪个路径逃逸。
pub fn is_write_patch_constrained_to_writable_paths(
    changes: &[PatchPathChange],
    writable_roots: &[PathBuf],
    base: &Path,
) -> Result<(), String> {
    for change in changes {
        let normalized = normalize_path_lexical(&change.path, base);

        // 检查是否在 writable_roots 内
        let in_roots = writable_roots.iter().any(|root| {
            let normalized_root = normalize_path_lexical(root, base);
            normalized.starts_with(&normalized_root)
        });

        if !in_roots {
            return Err(format!(
                "path {:?} is not within writable_roots",
                change.path
            ));
        }

        // 新建文件：检查父目录是否在 writable_roots 内
        if change.is_new_file {
            if let Some(parent) = normalized.parent() {
                let parent_in_roots = writable_roots.iter().any(|root| {
                    let normalized_root = normalize_path_lexical(root, base);
                    parent.starts_with(&normalized_root)
                });
                if !parent_in_roots {
                    return Err(format!(
                        "parent directory of new file {:?} is not within writable_roots",
                        change.path
                    ));
                }
            }
        }
    }
    Ok(())
}

// ── Hardlink 攻击防护 ───────────────────────────────────────────

/// 检查文件是否为 hardlink（link count > 1）。
///
/// 对照 codex `is_hardlink_attack`。攻击场景：攻击者在 writable_roots 外
/// 创建一个指向敏感文件的 hardlink，诱导补丁写入该 hardlink，从而修改
/// writable_roots 外的文件。
///
/// 返回 `Ok(false)` 表示安全（不是 hardlink 或文件不存在）；
/// 返回 `Ok(true)` 表示检测到 hardlink 攻击；
/// 返回 `Err` 表示无法检查（权限不足等）。
pub fn is_hardlink_attack(path: &Path) -> Result<bool, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // 文件不存在，不是攻击
            return Ok(false);
        }
        Err(e) => {
            return Err(format!("failed to stat {:?}: {}", path, e));
        }
    };

    // 符号链接不视为 hardlink 攻击（符号链接有单独的解析逻辑）
    if metadata.file_type().is_symlink() {
        return Ok(false);
    }

    // 检查 link count（仅 Unix 支持；Windows 的 std::fs::Metadata 不暴露 nlink）
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let hard_link_count = metadata.nlink();
        if hard_link_count > 1 {
            tracing::warn!(
                path = ?path,
                link_count = hard_link_count,
                "potential hardlink attack detected"
            );
            return Ok(true);
        }
    }
    #[cfg(not(unix))]
    {
        // Windows: std::fs::Metadata 不暴露 link count，跳过 hardlink 检查。
        // 如需更严格防护，可通过 GetFileInformationByHandle Win32 API 获取 nNumberOfLinks。
        tracing::trace!(
            path = ?path,
            "hardlink check skipped on non-Unix platform"
        );
    }

    Ok(false)
}

/// 检查多个路径是否存在 hardlink 攻击。
pub fn check_hardlink_attacks(paths: &[PathBuf]) -> Result<(), String> {
    for path in paths {
        if is_hardlink_attack(path)? {
            return Err(format!(
                "hardlink attack detected: {:?} has link count > 1",
                path
            ));
        }
    }
    Ok(())
}

// ── 决策矩阵 ────────────────────────────────────────────────────

/// 评估写补丁的安全性并给出决策。
///
/// 决策矩阵（对照 codex `evaluate_patch_safety`）：
/// 1. 先检查 hardlink 攻击 → 若检测到，Reject
/// 2. 检查路径约束 → 若有逃逸，Reject
/// 3. 受限 + 有沙箱（is_sandboxed=true）→ AutoApprove
/// 4. 受限 + 无沙箱 → AskUser
pub fn evaluate_patch_safety(
    changes: &[PatchPathChange],
    writable_roots: &[PathBuf],
    base: &Path,
    is_sandboxed: bool,
) -> PatchSafetyDecision {
    // 1. 检查 hardlink 攻击（仅检查已存在的文件）
    let existing_paths: Vec<PathBuf> = changes
        .iter()
        .filter_map(|c| {
            let normalized = normalize_path_lexical(&c.path, base);
            if normalized.exists() {
                Some(normalized)
            } else {
                None
            }
        })
        .collect();

    if let Err(reason) = check_hardlink_attacks(&existing_paths) {
        return PatchSafetyDecision::Reject { reason };
    }

    // 2. 检查路径约束
    if let Err(reason) = is_write_patch_constrained_to_writable_paths(changes, writable_roots, base)
    {
        return PatchSafetyDecision::Reject { reason };
    }

    // 3. 受限 + 有沙箱 → AutoApprove
    // 4. 受限 + 无沙箱 → AskUser
    if is_sandboxed {
        PatchSafetyDecision::AutoApprove
    } else {
        PatchSafetyDecision::AskUser
    }
}

// ── 单元测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_lexical_resolves_dot_dot() {
        let base = Path::new("/workspace/project");
        let path = Path::new("../other/file.txt");
        let normalized = normalize_path_lexical(path, base);
        assert_eq!(normalized, PathBuf::from("/workspace/other/file.txt"));
    }

    #[test]
    fn test_normalize_path_lexical_resolves_dot() {
        let base = Path::new("/workspace");
        let path = Path::new("./src/main.rs");
        let normalized = normalize_path_lexical(path, base);
        assert_eq!(normalized, PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn test_normalize_path_lexical_absolute_unchanged() {
        let base = Path::new("/workspace");
        let path = Path::new("/etc/passwd");
        let normalized = normalize_path_lexical(path, base);
        assert_eq!(normalized, PathBuf::from("/etc/passwd"));
    }

    #[test]
    fn test_normalize_path_lexical_handles_root_dot_dot() {
        // 根目录的 `..` 应保留（保守）
        let base = Path::new("/");
        let path = Path::new("../../etc/passwd");
        let normalized = normalize_path_lexical(path, base);
        // 根目录的 .. 会被保留，最终仍是 /etc/passwd
        assert_eq!(normalized, PathBuf::from("/etc/passwd"));
    }

    #[test]
    fn test_is_path_in_writable_roots_match() {
        let roots = vec![PathBuf::from("/workspace/project")];
        assert!(is_path_in_writable_roots(
            &Path::new("/workspace/project/src/main.rs"),
            &roots
        ));
    }

    #[test]
    fn test_is_path_in_writable_roots_no_match() {
        let roots = vec![PathBuf::from("/workspace/project")];
        assert!(!is_path_in_writable_roots(
            &Path::new("/etc/passwd"),
            &roots
        ));
    }

    #[test]
    fn test_is_path_in_writable_roots_empty_roots() {
        assert!(!is_path_in_writable_roots(
            &Path::new("/workspace/file"),
            &[]
        ));
    }

    #[test]
    fn test_is_write_patch_constrained_all_paths_within() {
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/workspace/src/a.rs"), is_new_file: false },
            PatchPathChange { path: PathBuf::from("/workspace/src/b.rs"), is_new_file: true },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let result = is_write_patch_constrained_to_writable_paths(&changes, &roots, Path::new("/"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_write_patch_constrained_path_escape() {
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/workspace/src/a.rs"), is_new_file: false },
            PatchPathChange { path: PathBuf::from("/etc/passwd"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let result = is_write_patch_constrained_to_writable_paths(&changes, &roots, Path::new("/"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("/etc/passwd"));
    }

    #[test]
    fn test_is_write_patch_constrained_relative_path() {
        // 相对路径基于 base 解析
        let changes = vec![
            PatchPathChange { path: PathBuf::from("src/a.rs"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace/project")];
        let result = is_write_patch_constrained_to_writable_paths(
            &changes,
            &roots,
            Path::new("/workspace/project"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_write_patch_constrained_relative_path_escape() {
        // 相对路径 `../escape` 逃逸出 writable_roots
        let changes = vec![
            PatchPathChange { path: PathBuf::from("../escape.rs"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace/project")];
        let result = is_write_patch_constrained_to_writable_paths(
            &changes,
            &roots,
            Path::new("/workspace/project"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_is_write_patch_constrained_new_file_parent_check() {
        // 新建文件路径等于 writable_root 本身时，父目录不在 writable_roots 内
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/workspace"), is_new_file: true },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let result = is_write_patch_constrained_to_writable_paths(&changes, &roots, Path::new("/"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("parent directory"));
    }

    #[test]
    fn test_is_hardlink_attack_nonexistent_file() {
        let result = is_hardlink_attack(&Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_ok());
        assert!(!result.unwrap()); // 文件不存在 → 不是攻击
    }

    #[test]
    fn test_evaluate_patch_safety_auto_approve() {
        // 受限 + 有沙箱 → AutoApprove
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/workspace/src/a.rs"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let decision = evaluate_patch_safety(
            &changes,
            &roots,
            Path::new("/"),
            true, // is_sandboxed
        );
        assert_eq!(decision, PatchSafetyDecision::AutoApprove);
    }

    #[test]
    fn test_evaluate_patch_safety_ask_user() {
        // 受限 + 无沙箱 → AskUser
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/workspace/src/a.rs"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let decision = evaluate_patch_safety(
            &changes,
            &roots,
            Path::new("/"),
            false, // not sandboxed
        );
        assert_eq!(decision, PatchSafetyDecision::AskUser);
    }

    #[test]
    fn test_evaluate_patch_safety_reject_on_escape() {
        // 路径逃逸 → Reject
        let changes = vec![
            PatchPathChange { path: PathBuf::from("/etc/passwd"), is_new_file: false },
        ];
        let roots = vec![PathBuf::from("/workspace")];
        let decision = evaluate_patch_safety(
            &changes,
            &roots,
            Path::new("/"),
            true, // is_sandboxed
        );
        assert!(decision.is_rejected());
    }

    #[test]
    fn test_patch_safety_decision_helpers() {
        assert!(PatchSafetyDecision::AutoApprove.is_approved());
        assert!(!PatchSafetyDecision::AutoApprove.is_rejected());
        assert!(!PatchSafetyDecision::AskUser.is_approved());
        assert!(!PatchSafetyDecision::AskUser.is_rejected());
        let reject = PatchSafetyDecision::Reject { reason: "test".to_string() };
        assert!(!reject.is_approved());
        assert!(reject.is_rejected());
    }
}
