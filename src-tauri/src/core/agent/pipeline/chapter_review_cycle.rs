//! S6.4: 章 review cycle 编排（审计 → 修订 → 重审循环 + 最佳快照回退）。
//!
//! 移植自 inkos `pipeline/chapter-review-cycle.ts`。核心机制：
//! - **预审计长度归一化**：硬区间漂移时调用 LengthNormalizerAgent 修正
//! - **parse_failed 守卫**：审计 LLM 输出无法解析时跳过自动修稿，避免误改正文
//! - **快照数组**：每一轮 (内容, 字数, 审计结果, 分数, 长度是否在区间内)
//! - **PASS_SCORE_THRESHOLD = 85** + **NET_IMPROVEMENT_EPSILON = 3**
//! - **最佳快照回退**：修订让事情变糟时回退到最高分版本
//! - **退出条件**：达到通过线 / 无净提升 / 修订产出空 / 达到最大轮次

use crate::shared::errors::AppError;
use crate::features::story::AuditResult;
use crate::core::agent::base::AgentContext;
use crate::core::agent::reviser::{ReviserAgent, ReviseMode};
use crate::core::agent::continuity::ContinuityAuditor;
use crate::core::agent::length_normalizer::LengthNormalizerAgent;
use crate::core::agent::length_metrics::{
    LengthSpec, LengthCheck, count_chapter_length, resolve_length_counting_mode,
};
use crate::core::agent::pipeline::chapter_persistence::save_chapter_file;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::state_store::gc::utils;

/// 通过线：分数 >= 此值且 length_in_range 且 audit.passed 即视为通过。
const PASS_SCORE_THRESHOLD: f64 = 85.0;
/// 净提升阈值：下一轮分数必须比当前高至少此值才继续循环。
const NET_IMPROVEMENT_EPSILON: f64 = 3.0;
/// 默认最大修稿轮次（inkos 默认 1 轮自动修稿）。
pub const DEFAULT_MAX_REVIEW_ITERATIONS: u32 = 1;

pub struct ReviewCycleResult {
    pub final_content: String,
    pub final_word_count: u32,
    pub revised: bool,
    pub audit_result: AuditResult,
    /// 修稿实际执行轮次（不含审计本身）。
    pub post_revise_count: u32,
    /// 是否触发了长度归一化（pre-audit）。
    pub normalize_applied: bool,
    /// 最终采用版本的审计分数。
    pub final_score: f64,
    /// 是否回退到了某个更早的快照。
    pub rolled_back: bool,
}

/// 一轮评估的快照。用于最佳快照回退。
struct Snapshot {
    content: String,
    word_count: u32,
    audit_result: AuditResult,
    score: f64,
    length_in_range: bool,
}

/// 评估结果（内部用）。
struct Assessment {
    audit_result: AuditResult,
    score: f64,
    length_in_range: bool,
}

/// 运行章 review cycle：审计 → 修订 → 重审循环 + 最佳快照回退。
///
/// 流程：
/// 1. 预审计长度归一化：如果 `target_words` 给定且当前字数硬区间漂移，
///    调用 LengthNormalizerAgent 修正
/// 2. 评估初始内容：LLM 审计 + 长度检查
/// 3. parse_failed 守卫：审计解析失败则跳过修稿
/// 4. 修稿循环：
///    - 调用 ReviserAgent（Auto 模式）
///    - 重新评估
///    - 达到通过线 → 退出
///    - 无净提升 → 退出
///    - 修订产出空 → 退出
/// 5. 最佳快照回退：若最终版本不是最高分，回退到最高分版本
pub async fn run_chapter_review_cycle(
    auditor_ctx: &AgentContext,
    reviser_ctx: &AgentContext,
    normalizer_ctx: &AgentContext,
    book_dir: &std::path::Path,
    chapter_number: u32,
    initial_content: &str,
    initial_title: &str,
    target_words: Option<u32>,
    max_iterations: u32,
    data_dir: &DataDir,
) -> Result<ReviewCycleResult, AppError> {
    let language = utils::read_book_language_from_dir(book_dir).unwrap_or_else(|| "zh".to_string());
    let counting_mode = resolve_length_counting_mode(&language);
    let length_spec = target_words.map(|w| LengthSpec::build(w, &language));

    let mut final_content = initial_content.to_string();
    let mut final_word_count = count_chapter_length(&final_content, counting_mode);
    let mut normalize_applied = false;

    // ── 1. 预审计长度归一化 ───────────────────────────────────
    if let Some(spec) = &length_spec {
        normalizer_ctx.tool_guardrails.lock().await.reset_for_turn();
        let (normalized, applied) = normalize_if_hard_drift(
            normalizer_ctx, &final_content, &language, spec,
        ).await?;
        if applied {
            final_content = normalized;
            final_word_count = count_chapter_length(&final_content, counting_mode);
            normalize_applied = true;
            // 持久化归一化后的草稿（与 inkos 行为一致：每次内容变更都落盘）
            save_chapter_file(book_dir, chapter_number, initial_title, &final_content)?;
        }
    }

    // ── 2. 初始评估 ────────────────────────────────────────────
    let auditor = ContinuityAuditor::new();
    auditor_ctx.tool_guardrails.lock().await.reset_for_turn();
    let initial_assessment = assess(
        &auditor, auditor_ctx, book_dir, chapter_number, &final_content, &length_spec, data_dir,
    ).await?;

    // ── 3. parse_failed 守卫 ──────────────────────────────────
    if initial_assessment.audit_result.parse_failed {
        tracing::warn!(
            chapter = chapter_number,
            "Audit output parsing failed; skipping automatic repair to avoid rewriting valid prose from an unreliable audit"
        );
        return Ok(ReviewCycleResult {
            final_content,
            final_word_count,
            revised: false,
            audit_result: initial_assessment.audit_result,
            post_revise_count: 0,
            normalize_applied,
            final_score: initial_assessment.score,
            rolled_back: false,
        });
    }

    let mut snapshots: Vec<Snapshot> = vec![Snapshot {
        content: final_content.clone(),
        word_count: final_word_count,
        audit_result: initial_assessment.audit_result.clone(),
        score: initial_assessment.score,
        length_in_range: initial_assessment.length_in_range,
    }];

    let mut current = initial_assessment;
    let mut post_revise_count: u32 = 0;
    let mut revised = false;

    // ── 4. 修稿循环 ───────────────────────────────────────────
    if !is_passed(&current) {
        let reviser = ReviserAgent::new();
        for iteration in 0..max_iterations {
            // 检查迭代预算
            if !reviser_ctx.iteration_budget.consume() {
                tracing::warn!(
                    iteration,
                    budget_used = reviser_ctx.iteration_budget.used(),
                    "Iteration budget exhausted during revision, stopping"
                );
                break;
            }

            // 重置工具守卫（每个修稿轮次独立）
            reviser_ctx.tool_guardrails.lock().await.reset_for_turn();

            tracing::info!(
                chapter = chapter_number,
                iteration,
                current_score = current.score,
                "Stage: Revise (review cycle)"
            );

            let revise_output = reviser.revise_chapter(
                reviser_ctx, book_dir, chapter_number,
                &final_content, &current.audit_result, ReviseMode::Auto,
                data_dir,
            ).await?;

            // 修订产出空或与原文相同 → 退出
            if revise_output.content.is_empty() || revise_output.content == final_content {
                tracing::warn!(
                    iteration,
                    "Repair iteration produced no new content, exiting loop"
                );
                break;
            }

            // 持久化修稿结果
            save_chapter_file(book_dir, chapter_number, initial_title, &revise_output.content)?;
            final_content = revise_output.content;
            final_word_count = revise_output.word_count;
            revised = true;
            post_revise_count += 1;

            // 重新评估（重置工具守卫）
            auditor_ctx.tool_guardrails.lock().await.reset_for_turn();
            let next = assess(
                &auditor, auditor_ctx, book_dir, chapter_number,
                &final_content, &length_spec, data_dir,
            ).await?;

            snapshots.push(Snapshot {
                content: final_content.clone(),
                word_count: final_word_count,
                audit_result: next.audit_result.clone(),
                score: next.score,
                length_in_range: next.length_in_range,
            });

            // 达到通过线 → 退出
            if is_passed(&next) {
                tracing::info!(
                    chapter = chapter_number,
                    score = next.score,
                    "Repair reached pass threshold, exiting loop"
                );
                current = next;
                break;
            }

            // 无净提升 → 退出
            if next.score < current.score + NET_IMPROVEMENT_EPSILON {
                tracing::warn!(
                    iteration,
                    prev_score = current.score,
                    next_score = next.score,
                    "Repair iteration no net improvement, exiting loop"
                );
                // 不更新 current — 下一行 best_snapshot 回退会处理
                break;
            }

            // 净提升达标，继续下一轮
            current = next;
        }
    }

    // ── 5. 最佳快照回退 ───────────────────────────────────────
    let best = pick_best_snapshot(&snapshots);
    let rolled_back = best.content != final_content && (
        (best.length_in_range && !current.length_in_range)
        || best.score >= current.score + NET_IMPROVEMENT_EPSILON
    );

    if rolled_back {
        tracing::warn!(
            chapter = chapter_number,
            best_score = best.score,
            current_score = current.score,
            "Rolling back to highest-scoring version"
        );
        final_content = best.content.clone();
        final_word_count = best.word_count;
        current = Assessment {
            audit_result: best.audit_result.clone(),
            score: best.score,
            length_in_range: best.length_in_range,
        };
        // 持久化回退版本
        save_chapter_file(book_dir, chapter_number, initial_title, &final_content)?;
    }

    Ok(ReviewCycleResult {
        final_content,
        final_word_count,
        revised,
        audit_result: current.audit_result,
        post_revise_count,
        normalize_applied,
        final_score: current.score,
        rolled_back,
    })
}

/// 评估一章内容：LLM 审计 + 长度检查。
async fn assess(
    auditor: &ContinuityAuditor,
    ctx: &AgentContext,
    book_dir: &std::path::Path,
    chapter_number: u32,
    content: &str,
    length_spec: &Option<LengthSpec>,
    data_dir: &DataDir,
) -> Result<Assessment, AppError> {
    let audit_result = auditor.audit_chapter(ctx, book_dir, chapter_number, data_dir).await?;

    let length_in_range = if let Some(spec) = length_spec {
        let wc = count_chapter_length(content, spec.counting_mode);
        !matches!(spec.check(wc), LengthCheck::TooShort | LengthCheck::TooLong)
    } else {
        true
    };

    let score = audit_result.score;
    Ok(Assessment {
        audit_result,
        score,
        length_in_range,
    })
}

/// 判断评估是否通过：passed + 分数达标 + 长度在区间内。
fn is_passed(a: &Assessment) -> bool {
    a.audit_result.passed && a.score >= PASS_SCORE_THRESHOLD && a.length_in_range
}

/// 长度硬区间漂移时调用 LengthNormalizerAgent 修正。
///
/// 返回 (内容, 是否应用了归一化)。
async fn normalize_if_hard_drift(
    ctx: &AgentContext,
    content: &str,
    language: &str,
    spec: &LengthSpec,
) -> Result<(String, bool), AppError> {
    let wc = count_chapter_length(content, spec.counting_mode);
    if !matches!(spec.check(wc), LengthCheck::TooShort | LengthCheck::TooLong) {
        return Ok((content.to_string(), false));
    }

    tracing::info!(
        current_words = wc,
        target_words = spec.target,
        "Length hard-range drift detected, invoking LengthNormalizerAgent"
    );
    let normalizer = LengthNormalizerAgent::new();
    let output = normalizer.normalize(ctx, content, spec, language).await?;
    Ok((output.content, output.applied))
}

/// 从快照数组中选择最佳版本。
///
/// 选择规则（移植自 inkos）：
/// 1. length_in_range 优先：有 in-range 的快照时排除 out-of-range 的
/// 2. 同 in-range 状态下，分数高出 NET_IMPROVEMENT_EPSILON 才胜出
fn pick_best_snapshot(snapshots: &[Snapshot]) -> &Snapshot {
    let mut best = &snapshots[0];
    for snap in &snapshots[1..] {
        if snap.length_in_range != best.length_in_range {
            if snap.length_in_range {
                best = snap;
            }
            // else：当前 best 在 range 内，snap 不在，跳过
        } else if snap.score >= best.score + NET_IMPROVEMENT_EPSILON {
            best = snap;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::{AuditIssue, AuditResult, AuditSeverity};

    fn make_snapshot(content: &str, score: f64, in_range: bool) -> Snapshot {
        Snapshot {
            content: content.to_string(),
            word_count: content.chars().count() as u32,
            audit_result: AuditResult {
                passed: score >= PASS_SCORE_THRESHOLD,
                score,
                issues: vec![],
                summary: String::new(),
                parse_failed: false,
            },
            score,
            length_in_range: in_range,
        }
    }

    fn make_audit(passed: bool, score: f64, parse_failed: bool) -> AuditResult {
        AuditResult {
            passed, score,
            issues: vec![],
            summary: String::new(),
            parse_failed,
        }
    }

    fn make_issue(severity: AuditSeverity) -> AuditIssue {
        AuditIssue {
            severity,
            category: "cat".to_string(),
            description: "desc".to_string(),
            suggestion: String::new(),
            repair_scope: None,
        }
    }

    // ── pick_best_snapshot ─────────────────────────────────────

    #[test]
    fn test_pick_best_single_snapshot_returns_itself() {
        let snaps = vec![make_snapshot("a", 70.0, true)];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "a");
    }

    #[test]
    fn test_pick_best_higher_score_wins_within_range() {
        let snaps = vec![
            make_snapshot("a", 70.0, true),
            make_snapshot("b", 90.0, true),
        ];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "b");
    }

    #[test]
    fn test_pick_best_in_range_beats_higher_score_out_of_range() {
        let snaps = vec![
            make_snapshot("low_in_range", 70.0, true),
            make_snapshot("high_out_of_range", 95.0, false),
        ];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "low_in_range");
    }

    #[test]
    fn test_pick_best_small_score_diff_no_change() {
        // 净提升 < EPSILON 不替换
        let snaps = vec![
            make_snapshot("a", 70.0, true),
            make_snapshot("b", 72.0, true), // 72 - 70 = 2 < 3
        ];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "a");
    }

    #[test]
    fn test_pick_best_three_snapshots_picks_highest_in_range() {
        let snaps = vec![
            make_snapshot("v1", 60.0, true),
            make_snapshot("v2", 80.0, false),
            make_snapshot("v3", 90.0, true),
        ];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "v3");
    }

    #[test]
    fn test_pick_best_all_out_of_range_picks_highest() {
        let snaps = vec![
            make_snapshot("v1", 60.0, false),
            make_snapshot("v2", 80.0, false),
            make_snapshot("v3", 70.0, false),
        ];
        let best = pick_best_snapshot(&snaps);
        assert_eq!(best.content, "v2");
    }

    // ── is_passed ──────────────────────────────────────────────

    #[test]
    fn test_is_passed_all_conditions_met() {
        let a = Assessment {
            audit_result: make_audit(true, 90.0, false),
            score: 90.0,
            length_in_range: true,
        };
        assert!(is_passed(&a));
    }

    #[test]
    fn test_is_passed_fails_when_audit_not_passed() {
        let a = Assessment {
            audit_result: make_audit(false, 90.0, false),
            score: 90.0,
            length_in_range: true,
        };
        assert!(!is_passed(&a));
    }

    #[test]
    fn test_is_passed_fails_below_threshold() {
        let a = Assessment {
            audit_result: make_audit(true, 80.0, false),
            score: 80.0,
            length_in_range: true,
        };
        assert!(!is_passed(&a));
    }

    #[test]
    fn test_is_passed_fails_when_length_out_of_range() {
        let a = Assessment {
            audit_result: make_audit(true, 90.0, false),
            score: 90.0,
            length_in_range: false,
        };
        assert!(!is_passed(&a));
    }

    #[test]
    fn test_is_passed_at_exact_threshold() {
        let a = Assessment {
            audit_result: make_audit(true, 85.0, false),
            score: 85.0,
            length_in_range: true,
        };
        assert!(is_passed(&a)); // 85 >= 85
    }

    // ── LengthSpec integration ─────────────────────────────────

    #[test]
    fn test_length_spec_in_range_returns_true() {
        let spec = LengthSpec::build(3000, "zh");
        let a = Assessment {
            audit_result: make_audit(true, 90.0, false),
            score: 90.0,
            length_in_range: !matches!(spec.check(3000), LengthCheck::TooShort | LengthCheck::TooLong),
        };
        assert!(is_passed(&a));
    }

    #[test]
    fn test_length_spec_hard_drift_returns_false() {
        let spec = LengthSpec::build(3000, "zh");
        let a = Assessment {
            audit_result: make_audit(true, 90.0, false),
            score: 90.0,
            length_in_range: !matches!(spec.check(100), LengthCheck::TooShort | LengthCheck::TooLong),
        };
        assert!(!is_passed(&a));
    }

    // ── AuditIssue severity sanity ─────────────────────────────

    #[test]
    fn test_audit_issue_severity_critical_indicates_blocking() {
        let issue = make_issue(AuditSeverity::Critical);
        assert_eq!(issue.severity, AuditSeverity::Critical);
    }
}
