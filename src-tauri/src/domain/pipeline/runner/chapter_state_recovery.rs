// Chapter State Recovery —— 状态校验失败恢复逻辑。
//
// 职责：状态校验失败后的恢复逻辑。
// - 构建校验反馈文本（供 settler 重试使用）
// - 构建降级问题列表（注入到 audit issues）
// - 构建/解析状态降级审查笔记（持久化到 chapter index）
// - 解析降级基础状态（ready-for-review / audit-failed）
//
// 实现说明：
// - retrySettlementAfterValidationFailure 移至 chapter_truth_validation.rs，
//   因其需要调用 AgentEngine + state_validator，与校验入口同文件更内聚。
// - 本文件仅保留纯函数（无 engine 依赖），便于单元测试。

#![allow(unused_imports)]

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::agents::continuity::{AuditIssue, IssueSeverity};
use super::super::agents::state_validator::{ValidationResult, ValidationWarning};
use super::super::types::{BookConfig, Language};

// ── 类型 ─────────────────────────────────────────────────────

/// 状态降级审查笔记
///
/// 持久化到 ChapterMeta.reviewNote，记录降级时的基础状态与注入问题，
/// 供后续 resolveStateDegradedBaseStatus 恢复真实章节状态使用。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateDegradedReviewNote {
    /// 始终为 "state-degraded"
    pub kind: String,
    /// "ready-for-review" | "audit-failed"
    pub base_status: String,
    /// 注入的降级问题（格式："[severity] description"）
    pub injected_issues: Vec<String>,
}

/// 结算重试结果
///
/// retry_settlement 返回：
/// - Recovered：重试后校验通过，携带新的 ValidationResult 与重试后的 truth 文件
/// - Degraded：重试仍失败，携带降级问题列表
pub enum SettlementRetryResult {
    /// 重试成功
    Recovered {
        validation: ValidationResult,
        /// 重试后的 current_state.md 内容（调用方需持久化）
        retry_state: String,
        /// 重试后的 pending_hooks.md 内容（调用方需持久化）
        retry_hooks: String,
    },
    /// 重试仍失败，降级
    Degraded {
        issues: Vec<AuditIssue>,
    },
}

// ── 辅助函数 ─────────────────────────────────────────────────

/// 将 IssueSeverity 转为小写字符串（与 JSON 序列化一致）
///
/// IssueSeverity 仅 derive 了 Deserialize，无法直接 serde::Serialize，
/// 因此用 match 显式映射，确保 review note 中 "[severity] description" 格式与 JSON 序列化一致。
fn severity_to_str(severity: &IssueSeverity) -> &'static str {
    match severity {
        IssueSeverity::Critical => r###"critical"###,
        IssueSeverity::Warning => r###"warning"###,
        IssueSeverity::Info => r###"info"###,
    }
}

// ── 核心函数 ─────────────────────────────────────────────────

/// 构建状态校验反馈文本
///
/// warnings 为空时返回通用提示（"上一次状态结算与正文矛盾..."），
/// 否则列出所有 warning（含 category + description），供 settler 重试时对照修正。
pub fn build_state_validation_feedback(
    warnings: &[ValidationWarning],
    language: Language,
) -> String {
    if warnings.is_empty() {
        return match language {
            Language::En => r###"The previous settlement contradicted the chapter text. Reconcile truth files strictly to the body."###.to_string(),
            Language::Zh => r###"上一次状态结算与正文矛盾。请严格以正文为准修正 truth files。"###.to_string(),
        };
    }

    match language {
        Language::En => {
            let mut lines: Vec<String> = vec![r###"The previous settlement failed validation. Fix these contradictions against the chapter body:"###.to_string()];
            for warning in warnings {
                lines.push(format!(
                    r###"- [{category}] {description}"###,
                    category = warning.category,
                    description = warning.description,
                ));
            }
            lines.join(r###"
"###)
        }
        Language::Zh => {
            let mut lines: Vec<String> = vec![r###"上一次状态结算未通过校验。请对照正文修正以下矛盾："###.to_string()];
            for warning in warnings {
                lines.push(format!(
                    r###"- [{category}] {description}"###,
                    category = warning.category,
                    description = warning.description,
                ));
            }
            lines.join(r###"
"###)
        }
    }
}

/// 构建状态降级问题列表
///
/// warnings 非空时：每条 warning 映射为 AuditIssue{severity: Warning, category: "state-validation", ...}
/// warnings 为空时：返回单条降级 issue（"状态结算重试后仍未通过校验。"）
///
/// 这些 issues 会被注入到 audit_result.issues 中，标记章节为 state-degraded。
pub fn build_state_degraded_issues(
    warnings: &[ValidationWarning],
    language: Language,
) -> Vec<AuditIssue> {
    let suggestion = match language {
        Language::En => r###"Repair chapter state from the persisted body before continuing."###,
        Language::Zh => r###"请先基于已保存正文修复本章 state，再继续后续章节。"###,
    };

    if !warnings.is_empty() {
        return warnings
            .iter()
            .map(|warning| AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: None,
                category: r###"state-validation"###.to_string(),
                description: warning.description.clone(),
                suggestion: suggestion.to_string(),
            })
            .collect();
    }

    let description = match language {
        Language::En => r###"State validation still failed after settlement retry."###,
        Language::Zh => r###"状态结算重试后仍未通过校验。"###,
    };

    vec![AuditIssue {
        severity: IssueSeverity::Warning,
        repair_scope: None,
        category: r###"state-validation"###.to_string(),
        description: description.to_string(),
        suggestion: suggestion.to_string(),
    }]
}

/// 构建状态降级审查笔记 JSON
///
/// 输出格式：{"kind":"state-degraded","baseStatus":...,"injectedIssues":[...]}
/// injectedIssues 每项格式："[severity] description"
///
/// 此 JSON 会被写入 ChapterMeta.reviewNote，供 resolve_state_degraded_base_status 解析。
pub fn build_state_degraded_review_note(
    base_status: &str,
    issues: &[AuditIssue],
) -> String {
    let note = StateDegradedReviewNote {
        kind: r###"state-degraded"###.to_string(),
        base_status: base_status.to_string(),
        injected_issues: issues
            .iter()
            .map(|issue| {
                format!(
                    r###"[{severity}] {description}"###,
                    severity = severity_to_str(&issue.severity),
                    description = issue.description,
                )
            })
            .collect(),
    };
    // 序列化 String/Vec<String> 字段理论上不会失败，但避免 expect/panic：
    // 失败时降级为仅含 baseStatus 的最小 JSON（保留 kind 标识，下游可识别）
    serde_json::to_string(&note).unwrap_or_else(|e| {
        tracing::error!(error = %e, "StateDegradedReviewNote 序列化失败，降级为最小 JSON");
        format!(
            r#"{{"kind":"state-degraded","baseStatus":"{}","injectedIssues":[]}}"#,
            base_status.replace('"', r#"\""#)
        )
    })
}

/// 解析状态降级审查笔记
///
/// 任何字段不合法或 JSON 解析失败均返回 None：
/// - kind 必须为 "state-degraded"
/// - baseStatus 必须为 "ready-for-review" 或 "audit-failed"
/// - injectedIssues 必须为数组（非字符串元素会被过滤）
pub fn parse_state_degraded_review_note(
    review_note: Option<&str>,
) -> Option<StateDegradedReviewNote> {
    let raw = review_note?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;

    let kind = value.get(r###"kind"###)?.as_str()?;
    if kind != r###"state-degraded"### {
        return None;
    }

    let base_status = value.get(r###"baseStatus"###)?.as_str()?;
    if base_status != r###"ready-for-review"### && base_status != r###"audit-failed"### {
        return None;
    }

    let injected_array = value.get(r###"injectedIssues"###)?.as_array()?;
    let injected_issues: Vec<String> = injected_array
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    Some(StateDegradedReviewNote {
        kind: r###"state-degraded"###.to_string(),
        base_status: base_status.to_string(),
        injected_issues,
    })
}

/// 解析状态降级基础状态
///
/// 优先从 review_note 解析 baseStatus；
/// 若 review_note 缺失或非法，则按 audit_issues 中是否含 "[critical]" 前缀判定：
/// - 含 [critical] → "audit-failed"
/// - 否则 → "ready-for-review"
pub fn resolve_state_degraded_base_status(
    review_note: Option<&str>,
    audit_issues: &[String],
) -> String {
    if let Some(note) = parse_state_degraded_review_note(review_note) {
        return note.base_status;
    }

    if audit_issues
        .iter()
        .any(|issue| issue.starts_with(r###"[critical]"###))
    {
        r###"audit-failed"###.to_string()
    } else {
        r###"ready-for-review"###.to_string()
    }
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_warning(category: &str, description: &str) -> ValidationWarning {
        ValidationWarning {
            category: category.to_string(),
            description: description.to_string(),
        }
    }

    fn make_issue(severity: IssueSeverity, description: &str) -> AuditIssue {
        AuditIssue {
            severity,
            repair_scope: None,
            category: r###"state-validation"###.to_string(),
            description: description.to_string(),
            suggestion: r###"请修复"###.to_string(),
        }
    }

    // ── build_state_validation_feedback ──────────────────────

    #[test]
    fn build_state_validation_feedback_empty_warnings() {
        // 空警告 → 通用提示
        let zh = build_state_validation_feedback(&[], Language::Zh);
        assert!(zh.contains(r###"上一次状态结算与正文矛盾"###));
        assert!(zh.contains(r###"truth files"###));

        let en = build_state_validation_feedback(&[], Language::En);
        assert!(en.contains(r###"The previous settlement contradicted the chapter text"###));
    }

    #[test]
    fn build_state_validation_feedback_with_warnings() {
        // 非空警告 → 列出每条 warning（含 category + description）
        let warnings = vec![
            make_warning(r###"Hook 异常"###, r###"H007 状态不一致"###),
            make_warning(r###"时间不可能性"###, r###"时长不合逻辑"###),
        ];

        let zh = build_state_validation_feedback(&warnings, Language::Zh);
        assert!(zh.contains(r###"上一次状态结算未通过校验"###));
        assert!(zh.contains(r###"[Hook 异常] H007 状态不一致"###));
        assert!(zh.contains(r###"[时间不可能性] 时长不合逻辑"###));

        let en = build_state_validation_feedback(&warnings, Language::En);
        assert!(en.contains(r###"The previous settlement failed validation"###));
        assert!(en.contains(r###"[Hook 异常] H007 状态不一致"###));
    }

    // ── build_state_degraded_issues ──────────────────────────

    #[test]
    fn build_state_degraded_issues_with_warnings() {
        // 非空警告 → 每条 warning 映射为 AuditIssue
        let warnings = vec![
            make_warning(r###"Hook 异常"###, r###"H007 状态不一致"###),
            make_warning(r###"时间不可能性"###, r###"时长不合逻辑"###),
        ];
        let issues = build_state_degraded_issues(&warnings, Language::Zh);

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].category, r###"state-validation"###);
        assert_eq!(issues[0].description, r###"H007 状态不一致"###);
        assert_eq!(issues[1].description, r###"时长不合逻辑"###);
        // 两条 suggestion 应一致
        assert_eq!(issues[0].suggestion, issues[1].suggestion);
    }

    #[test]
    fn build_state_degraded_issues_empty_warnings() {
        // 空警告 → 单条降级 issue
        let issues = build_state_degraded_issues(&[], Language::Zh);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].category, r###"state-validation"###);
        assert_eq!(issues[0].description, r###"状态结算重试后仍未通过校验。"###);

        // 英文版本
        let issues_en = build_state_degraded_issues(&[], Language::En);
        assert_eq!(issues_en.len(), 1);
        assert_eq!(
            issues_en[0].description,
            r###"State validation still failed after settlement retry."###
        );
    }

    // ── build_state_degraded_review_note ─────────────────────

    #[test]
    fn build_state_degraded_review_note_serializes_correctly() {
        let issues = vec![
            make_issue(IssueSeverity::Warning, r###"状态不一致"###),
            make_issue(IssueSeverity::Critical, r###"时间线断裂"###),
        ];
        let json = build_state_degraded_review_note(r###"ready-for-review"###, &issues);

        // 解析回 JSON 验证字段
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect(r###"JSON 应合法"###);
        assert_eq!(
            parsed.get(r###"kind"###).and_then(|v| v.as_str()),
            Some(r###"state-degraded"###)
        );
        assert_eq!(
            parsed.get(r###"baseStatus"###).and_then(|v| v.as_str()),
            Some(r###"ready-for-review"###)
        );
        let injected = parsed
            .get(r###"injectedIssues"###)
            .and_then(|v| v.as_array())
            .expect(r###"应为数组"###);
        assert_eq!(injected.len(), 2);
        assert_eq!(injected[0].as_str(), Some(r###"[warning] 状态不一致"###));
        assert_eq!(injected[1].as_str(), Some(r###"[critical] 时间线断裂"###));
    }

    // ── parse_state_degraded_review_note ─────────────────────

    #[test]
    fn parse_state_degraded_review_note_valid() {
        let json = r###"{"kind":"state-degraded","baseStatus":"audit-failed","injectedIssues":["[warning] 状态不一致"]}"###;
        let note = parse_state_degraded_review_note(Some(json));
        assert!(note.is_some());

        let note = note.unwrap();
        assert_eq!(note.kind, r###"state-degraded"###);
        assert_eq!(note.base_status, r###"audit-failed"###);
        assert_eq!(note.injected_issues.len(), 1);
        assert_eq!(note.injected_issues[0], r###"[warning] 状态不一致"###);
    }

    #[test]
    fn parse_state_degraded_review_note_invalid() {
        // None 输入
        assert!(parse_state_degraded_review_note(None).is_none());

        // 非 JSON
        assert!(parse_state_degraded_review_note(Some(r###"not json"###)).is_none());

        // kind 错误
        let bad_kind = r###"{"kind":"other","baseStatus":"ready-for-review","injectedIssues":[]}"###;
        assert!(parse_state_degraded_review_note(Some(bad_kind)).is_none());

        // baseStatus 错误
        let bad_status = r###"{"kind":"state-degraded","baseStatus":"unknown","injectedIssues":[]}"###;
        assert!(parse_state_degraded_review_note(Some(bad_status)).is_none());

        // injectedIssues 非数组
        let bad_array = r###"{"kind":"state-degraded","baseStatus":"ready-for-review","injectedIssues":"not-array"}"###;
        assert!(parse_state_degraded_review_note(Some(bad_array)).is_none());
    }

    #[test]
    fn parse_state_degraded_review_note_filters_non_strings() {
        // injectedIssues 包含非字符串元素，应被过滤掉
        let json = r###"{"kind":"state-degraded","baseStatus":"ready-for-review","injectedIssues":["[warning] ok", 42, null, "[critical] bad"]}"###;
        let note = parse_state_degraded_review_note(Some(json)).expect(r###"应解析成功"###);
        assert_eq!(note.injected_issues.len(), 2);
        assert_eq!(note.injected_issues[0], r###"[warning] ok"###);
        assert_eq!(note.injected_issues[1], r###"[critical] bad"###);
    }

    // ── resolve_state_degraded_base_status ───────────────────

    #[test]
    fn resolve_state_degraded_base_status_from_note() {
        // 有合法 review_note → 返回 note.baseStatus
        let json = r###"{"kind":"state-degraded","baseStatus":"audit-failed","injectedIssues":["[warning] x"]}"###;
        let status = resolve_state_degraded_base_status(Some(json), &[]);
        assert_eq!(status, r###"audit-failed"###);

        // 即使 audit_issues 含 [critical]，note 优先
        let json2 = r###"{"kind":"state-degraded","baseStatus":"ready-for-review","injectedIssues":[]}"###;
        let status2 = resolve_state_degraded_base_status(
            Some(json2),
            &[r###"[critical] xxx"###.to_string()],
        );
        assert_eq!(status2, r###"ready-for-review"###);
    }

    #[test]
    fn resolve_state_degraded_base_status_from_critical_issue() {
        // 无 note，audit_issues 含 [critical] → audit-failed
        let issues = vec![
            r###"[warning] 小问题"###.to_string(),
            r###"[critical] 严重问题"###.to_string(),
        ];
        let status = resolve_state_degraded_base_status(None, &issues);
        assert_eq!(status, r###"audit-failed"###);
    }

    #[test]
    fn resolve_state_degraded_base_status_no_critical() {
        // 无 note，audit_issues 不含 [critical] → ready-for-review
        let issues = vec![r###"[warning] 小问题"###.to_string()];
        let status = resolve_state_degraded_base_status(None, &issues);
        assert_eq!(status, r###"ready-for-review"###);

        // 空 issues → ready-for-review
        let status2 = resolve_state_degraded_base_status(None, &[]);
        assert_eq!(status2, r###"ready-for-review"###);
    }
}
