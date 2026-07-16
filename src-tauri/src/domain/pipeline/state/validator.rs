// RuntimeState 校验器。
//
// 校验 snapshot 一致性：重复 hook_id、重复 summary chapter、currentState 超前于 manifest。
// Rust 版用 serde 反序列化替代 zod parse，解析失败即视为 issue。

use super::types::*;

/// 校验 issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub path: String,
}

/// 校验 snapshot 一致性。
/// 注意：完整版本接受 unknown 然后用 zod parse；Rust 版本接收已反序列化的 Snapshot，
/// 因此 "invalid_xxx" 类 issue（反序列化失败）在 load 阶段已以 Err 返回，此处只做结构级校验。
pub fn validate_runtime_state(snapshot: &RuntimeStateSnapshot) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    // 重复 hook_id
    let mut seen_hooks = std::collections::HashSet::new();
    for hook in &snapshot.hooks.hooks {
        if !seen_hooks.insert(&hook.hook_id) {
            issues.push(ValidationIssue {
                code: "duplicate_hook_id".into(),
                message: format!("duplicate hook id: {}", hook.hook_id),
                path: format!("hooks.{}", hook.hook_id),
            });
        }
    }

    // hook_id 引用完整性：depends_on 引用的 hook_id 必须存在于已知集合
    for hook in &snapshot.hooks.hooks {
        if let Some(deps) = &hook.depends_on {
            for dep_id in deps {
                if !seen_hooks.contains(dep_id) {
                    issues.push(ValidationIssue {
                        code: "dangling_hook_dependency".into(),
                        message: format!(
                            "hook '{}' depends on missing hook '{}'",
                            hook.hook_id, dep_id
                        ),
                        path: format!("hooks.{}.depends_on", hook.hook_id),
                    });
                }
            }
        }
    }

    // 重复 summary chapter
    let mut seen_summaries = std::collections::HashSet::new();
    for row in &snapshot.chapter_summaries.rows {
        if !seen_summaries.insert(row.chapter) {
            issues.push(ValidationIssue {
                code: "duplicate_summary_chapter".into(),
                message: format!("duplicate summary chapter: {}", row.chapter),
                path: format!("chapterSummaries.{}", row.chapter),
            });
        }
    }

    // currentState 超前于 manifest
    if snapshot.current_state.chapter > snapshot.manifest.last_applied_chapter {
        issues.push(ValidationIssue {
            code: "current_state_ahead_of_manifest".into(),
            message: format!(
                "current state chapter {} exceeds manifest {}",
                snapshot.current_state.chapter, snapshot.manifest.last_applied_chapter
            ),
            path: "currentState.chapter".into(),
        });
    }

    issues
}

/// 将 issues 拼接为单个错误消息（reducer 调用失败时使用）
pub fn issues_to_error(issues: &[ValidationIssue]) -> String {
    issues
        .iter()
        .map(|i| format!("{}: {}", i.code, i.message))
        .collect::<Vec<_>>()
        .join("; ")
}
