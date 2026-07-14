// Chapter Review Cycle —— 章节评审循环。
//
// 职责：Audit↔Revise 评分循环，带最佳快照回退。
// 流程：normalize（硬漂移）→ assess（audit + score）→ revise（auto）→ re-assess → 选最佳快照。
//
// 实现差异：
// - analyzeAITells / analyzeSensitiveWords / runPostWriteChecks 已移植到 Rust
//   （ai_tells.rs / sensitive_words.rs / post_write_checks.rs），在 assess 中合并。
// - normalizePostWriteSurface / assertChapterContentNotEmpty 已引入（post_write_checks.rs）。
// - logWarn / logStage 用 tracing::warn! / tracing::info! 替代。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::agents::continuity::{
    audit_chapter, AuditIssue, AuditResult, AuditorContext, IssueSeverity, RepairScope,
};
use super::super::agents::length_normalizer::normalize_chapter;
use super::super::agents::reviser::{revise_chapter, ReviserContext, ReviseMode};
use super::super::governance::input::GovernedArtifacts;
use super::super::governance::length::{
    count_chapter_length, is_outside_hard_range, LengthSpec,
};
use super::super::types::BookConfig;
use super::ai_tells::analyze_ai_tells;
use super::post_write_checks::{
    assert_chapter_content_not_empty, normalize_post_write_surface, run_post_write_checks,
};
use super::sensitive_words::analyze_sensitive_words;

// ── 常量 ─────────────────────────────────────────────────────

const DEFAULT_MAX_REVIEW_ITERATIONS: usize = 1;
const PASS_SCORE_THRESHOLD: f32 = 85.0;
const NET_IMPROVEMENT_EPSILON: f32 = 3.0;

// ── 结果类型 ─────────────────────────────────────────────────

/// Token 用量汇总
#[derive(Debug, Clone, Default)]
pub struct CycleUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Chapter review cycle 结果
#[derive(Debug, Clone)]
pub struct CycleResult {
    pub final_content: String,
    pub final_word_count: u32,
    pub pre_audit_normalized_word_count: u32,
    pub revised: bool,
    pub audit_result: AuditResult,
    pub total_usage: CycleUsage,
    pub post_revise_count: u32,
    pub normalize_applied: bool,
}

/// 字数归一化结果（normalize_if_hard_drift 内部使用）
struct NormalizeResult {
    content: String,
    #[allow(dead_code)]
    word_count: u32,
    applied: bool,
    token_usage: Option<CycleUsage>,
}

// ── 内部类型 ─────────────────────────────────────────────────

/// 评审快照
/// 每一轮 assess 后保存一份，用于最终选择最佳版本
struct ReviewSnapshot {
    content: String,
    word_count: u32,
    audit_result: AuditResult,
    score: f32,
    length_in_range: bool,
}

/// 评估结果（assess 输出）
struct Assessment {
    audit_result: AuditResult,
    score: f32,
    length_in_range: bool,
}

// ── 主函数 ───────────────────────────────────────────────────

/// 运行章节评审循环：audit → revise → re-audit，带最佳快照回退。
#[allow(clippy::too_many_arguments)]
pub async fn run_chapter_review_cycle(
    engine: &AgentEngine,
    book: &BookConfig,
    book_dir: &std::path::Path,
    chapter_number: u32,
    initial_content: &str,
    initial_word_count: u32,
    length_spec: &LengthSpec,
    initial_usage: CycleUsage,
    governed_artifacts: Option<&GovernedArtifacts>,
    max_review_iterations: Option<usize>,
) -> Result<CycleResult, AppError> {
    let mut total_usage = initial_usage;
    let mut normalize_applied = false;
    let mut final_content = initial_content.to_string();
    // initial_word_count 会被 normalize 后的重计算覆盖；保留参数以对齐签名
    let _ = initial_word_count;

    // ── 字数归一化（pre-audit）：仅当超出硬边界时调用 length_normalizer ──
    // 长度不混入 reviser 的 issues——normalize 作为独立步骤处理。
    let normalized = normalize_if_hard_drift(engine, &final_content, length_spec).await?;
    total_usage = add_usage(total_usage, normalized.token_usage);
    final_content = normalized.content;
    let mut final_word_count = count_chapter_length(&final_content, length_spec.counting_mode);
    if normalized.applied {
        normalize_applied = true;
    }

    // ── 初始评估 ──
    tracing::info!(chapter = chapter_number, "审计草稿");
    let initial = assess(
        engine,
        book,
        book_dir,
        chapter_number,
        &final_content,
        length_spec,
        governed_artifacts,
    )
    .await?;

    let mut snapshots: Vec<ReviewSnapshot> = vec![ReviewSnapshot {
        content: final_content.clone(),
        word_count: final_word_count,
        audit_result: initial.audit_result.clone(),
        score: initial.score,
        length_in_range: initial.length_in_range,
    }];

    // 解析失败：跳过自动修稿，避免误改正文
    if initial.audit_result.parse_failed {
        tracing::warn!("审稿输出解析失败，跳过自动修稿以避免误改正文");
        return Ok(CycleResult {
            final_content,
            final_word_count,
            pre_audit_normalized_word_count: final_word_count,
            revised: false,
            audit_result: initial.audit_result,
            total_usage,
            post_revise_count: 0,
            normalize_applied,
        });
    }

    let mut current_audit = Assessment {
        audit_result: initial.audit_result.clone(),
        score: initial.score,
        length_in_range: initial.length_in_range,
    };
    let mut post_revise_count: u32 = 0;

    // ── 评分循环：assess → revise → assess ──
    // 默认一次自动修复轮次；项目可调高以接受更慢但更持久的修复。
    let max_iterations = max_review_iterations.unwrap_or(DEFAULT_MAX_REVIEW_ITERATIONS);

    if !is_passed(&current_audit) {
        for iteration in 0..max_iterations {
            tracing::info!(
                "修复轮次 {}/{}（当前 {} 分）",
                iteration + 1,
                max_iterations,
                current_audit.score
            );

            let reviser_ctx = build_reviser_context(book_dir, governed_artifacts);
            let revise_output = revise_chapter(
                engine,
                book,
                chapter_number,
                &final_content,
                &current_audit.audit_result.issues,
                ReviseMode::Auto,
                &reviser_ctx,
            )
            .await?;

            // 修稿输出为空或与原文相同 → 退出循环
            if revise_output.revised_content.is_empty()
                || revise_output.revised_content == final_content
            {
                tracing::warn!(
                    "修复轮次 {} 未产出新内容，退出循环",
                    iteration + 1
                );
                break;
            }

            let revised_content = revise_output.revised_content;
            let revised_word_count =
                count_chapter_length(&revised_content, length_spec.counting_mode);

            // 重新评估修订内容（temperature=0，对应 re-assess）
            // 若修订内容字数漂移，length_in_range 为 false → isPassed 失败 →
            // bestSnapshot 会选择较早的 in-range 版本。循环内无需 normalize。
            let next_assessment = assess(
                engine,
                book,
                book_dir,
                chapter_number,
                &revised_content,
                length_spec,
                governed_artifacts,
            )
            .await?;

            snapshots.push(ReviewSnapshot {
                content: revised_content.clone(),
                word_count: revised_word_count,
                audit_result: next_assessment.audit_result.clone(),
                score: next_assessment.score,
                length_in_range: next_assessment.length_in_range,
            });

            // 通过判定：score >= 85 AND audit passed AND length in range
            if is_passed(&next_assessment) {
                tracing::info!(
                    "修复后达到通过线（{} 分），退出循环",
                    next_assessment.score
                );
                final_content = revised_content;
                final_word_count = revised_word_count;
                post_revise_count = revised_word_count;
                current_audit = Assessment {
                    audit_result: next_assessment.audit_result.clone(),
                    score: next_assessment.score,
                    length_in_range: next_assessment.length_in_range,
                };
                break;
            }

            // 净提升判定：score >= current + epsilon 才接受
            if next_assessment.score >= current_audit.score + NET_IMPROVEMENT_EPSILON {
                final_content = revised_content;
                final_word_count = revised_word_count;
                post_revise_count = revised_word_count;
                current_audit = Assessment {
                    audit_result: next_assessment.audit_result.clone(),
                    score: next_assessment.score,
                    length_in_range: next_assessment.length_in_range,
                };
                // 继续下一轮
            } else {
                tracing::warn!(
                    "修复轮次 {} 未净提升（{} → {}），退出循环",
                    iteration + 1,
                    current_audit.score,
                    next_assessment.score
                );
                break;
            }
        }
    }

    // ── 选择最佳快照 ──
    let best_snapshot = pick_best_snapshot(&snapshots);

    let should_restore_best = match best_snapshot {
        Some(best) => {
            best.content != final_content
                && ((best.length_in_range && !current_audit.length_in_range)
                    || best.score >= current_audit.score + NET_IMPROVEMENT_EPSILON)
        }
        None => false,
    };

    if should_restore_best {
        if let Some(best) = best_snapshot {
            tracing::warn!(
                "回退到最高分版本（{} 分 vs 当前 {} 分）",
                best.score,
                current_audit.score
            );
            final_content = best.content.clone();
            final_word_count = best.word_count;
            current_audit = Assessment {
                audit_result: best.audit_result.clone(),
                score: best.score,
                length_in_range: best.length_in_range,
            };
        }
    }

    let revised = snapshots.len() > 1 && final_content != initial_content;

    Ok(CycleResult {
        final_content,
        final_word_count,
        // 在此处返回 finalWordCount（已知偏差，保留以对齐）
        pre_audit_normalized_word_count: final_word_count,
        revised,
        audit_result: current_audit.audit_result,
        total_usage,
        post_revise_count,
        normalize_applied,
    })
}

// ── 辅助函数 ─────────────────────────────────────────────────

/// 评估章节：audit + 规则检测 + 计算 score + 判定 length_in_range
///
/// 合并 LLM audit 结果与 analyzeAITells / analyzeSensitiveWords / runPostWriteChecks
/// 的规则检测结果。Critical 级别问题强制 passed=false 并扣 10 分，Warning 扣 3 分，
/// Info 扣 1 分。
async fn assess(
    engine: &AgentEngine,
    book: &BookConfig,
    book_dir: &std::path::Path,
    chapter_number: u32,
    content: &str,
    length_spec: &LengthSpec,
    governed_artifacts: Option<&GovernedArtifacts>,
) -> Result<Assessment, AppError> {
    let language = book.language.unwrap_or_default();
    let normalized = normalize_post_write_surface(content, language);

    // 章节内容非空断言（normalize 后）：空内容跳过 LLM audit，直接返回 0 分
    if let Err(e) = assert_chapter_content_not_empty(&normalized) {
        tracing::warn!("章节内容为空，跳过 LLM audit");
        let mut audit_result = AuditResult::default();
        audit_result.passed = false;
        audit_result.overall_score = Some(0.0);
        audit_result.parse_failed = true;
        audit_result.issues.push(AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Structural),
            category: "章节内容为空".to_string(),
            description: e.message,
            suggestion: "章节内容不能为空，请检查写作输出".to_string(),
        });
        let word_count = count_chapter_length(content, length_spec.counting_mode);
        let length_in_range = !is_outside_hard_range(word_count, length_spec);
        return Ok(Assessment {
            audit_result,
            score: 0.0,
            length_in_range,
        });
    }

    // 调用 continuity auditor
    let mut audit_result = audit_chapter(
        engine,
        book,
        chapter_number,
        "", // chapter_title — auditor 可从内容推断
        content,
        &build_auditor_context(book_dir, governed_artifacts),
    )
    .await?;

    // 运行规则检测（analyzeAITells + analyzeSensitiveWords + runPostWriteChecks）
    // 在原始 content 上检测（em-dash 等检查需看到原始字符）
    let mut additional_issues: Vec<AuditIssue> = Vec::new();
    additional_issues.extend(analyze_ai_tells(content, language));
    additional_issues.extend(analyze_sensitive_words(content));
    additional_issues.extend(run_post_write_checks(content, language));

    // 合并 issues 并重新计算 score / passed
    let mut score = audit_result.overall_score.unwrap_or(0.0);
    let mut passed = audit_result.passed;
    for issue in &additional_issues {
        match issue.severity {
            IssueSeverity::Critical => {
                score = (score - 10.0).max(0.0);
                passed = false;
            }
            IssueSeverity::Warning => {
                score = (score - 3.0).max(0.0);
            }
            IssueSeverity::Info => {
                score = (score - 1.0).max(0.0);
            }
        }
    }
    audit_result.issues.extend(additional_issues);
    audit_result.overall_score = Some(score);
    audit_result.passed = passed;

    let word_count = count_chapter_length(content, length_spec.counting_mode);
    let length_in_range = !is_outside_hard_range(word_count, length_spec);

    Ok(Assessment {
        audit_result,
        score,
        length_in_range,
    })
}

/// 构建 AuditorContext：从 truth files 读取 + governed_artifacts 注入 memo
fn build_auditor_context(
    book_dir: &std::path::Path,
    governed_artifacts: Option<&GovernedArtifacts>,
) -> AuditorContext {
    let story_dir = book_dir.join("story");
    let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();

    let chapter_memo = governed_artifacts
        .map(|g| g.plan.memo.body.clone())
        .unwrap_or_default();

    AuditorContext {
        current_state: read_safe("current_state.md"),
        pending_hooks: read_safe("pending_hooks.md"),
        chapter_summaries: read_safe("chapter_summaries.md"),
        volume_map: read_safe("outline/volume_map.md"),
        story_frame: read_safe("outline/story_frame.md"),
        book_rules: read_safe("book_rules.md"),
        style_guide: read_safe("style_guide.md"),
        chapter_memo,
        previous_chapter: String::new(), // 由 caller 在需要时填充
        parent_canon: read_safe("parent_canon.md"),
        fanfic_canon: read_safe("fanfic_canon.md"),
    }
}

/// 构建 ReviserContext：从 truth files 读取 + governed_artifacts 注入 memo
fn build_reviser_context(
    book_dir: &std::path::Path,
    governed_artifacts: Option<&GovernedArtifacts>,
) -> ReviserContext {
    let story_dir = book_dir.join("story");
    let read_safe = |name: &str| std::fs::read_to_string(story_dir.join(name)).unwrap_or_default();

    let chapter_memo = governed_artifacts
        .map(|g| g.plan.memo.body.clone())
        .unwrap_or_default();

    ReviserContext {
        current_state: read_safe("current_state.md"),
        pending_hooks: read_safe("pending_hooks.md"),
        chapter_summaries: read_safe("chapter_summaries.md"),
        volume_map: read_safe("outline/volume_map.md"),
        story_frame: read_safe("outline/story_frame.md"),
        book_rules: read_safe("book_rules.md"),
        style_guide: read_safe("style_guide.md"),
        chapter_memo,
    }
}

/// 字数归一化：仅当超出硬边界时调用 length_normalizer agent
async fn normalize_if_hard_drift(
    engine: &AgentEngine,
    content: &str,
    length_spec: &LengthSpec,
) -> Result<NormalizeResult, AppError> {
    let word_count = count_chapter_length(content, length_spec.counting_mode);
    if !is_outside_hard_range(word_count, length_spec) {
        return Ok(NormalizeResult {
            content: content.to_string(),
            word_count,
            applied: false,
            token_usage: None,
        });
    }

    // 调用 length_normalizer agent
    let normalize_output = normalize_chapter(
        engine,
        content,
        length_spec.target,
        length_spec.soft_min,
        length_spec.soft_max,
    )
    .await?;

    let new_count =
        count_chapter_length(&normalize_output.normalized_content, length_spec.counting_mode);
    Ok(NormalizeResult {
        content: normalize_output.normalized_content,
        word_count: new_count,
        applied: normalize_output.applied,
        token_usage: None, // length_normalizer 暂未返回 token usage
    })
}

/// 通过判定：passed AND score >= 85 AND length in range
fn is_passed(assessment: &Assessment) -> bool {
    assessment.audit_result.passed
        && assessment.score >= PASS_SCORE_THRESHOLD
        && assessment.length_in_range
}

/// 选择最佳快照（reduce 逻辑）
/// - length_in_range 不同的：true 优先
/// - 同 length_in_range 状态：score >= best + epsilon 才换
fn pick_best_snapshot(snapshots: &[ReviewSnapshot]) -> Option<&ReviewSnapshot> {
    snapshots.iter().reduce(|best, snap| {
        if snap.length_in_range != best.length_in_range {
            // length_in_range = true 的优先
            if snap.length_in_range {
                snap
            } else {
                best
            }
        } else if snap.score >= best.score + NET_IMPROVEMENT_EPSILON {
            snap
        } else {
            best
        }
    })
}

/// 累加 token 用量
fn add_usage(left: CycleUsage, right: Option<CycleUsage>) -> CycleUsage {
    match right {
        Some(r) => CycleUsage {
            prompt_tokens: left.prompt_tokens + r.prompt_tokens,
            completion_tokens: left.completion_tokens + r.completion_tokens,
            total_tokens: left.total_tokens + r.total_tokens,
        },
        None => left,
    }
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_assessment(passed: bool, score: f32, length_in_range: bool) -> Assessment {
        Assessment {
            audit_result: AuditResult {
                passed,
                overall_score: Some(score),
                ..Default::default()
            },
            score,
            length_in_range,
        }
    }

    fn make_snapshot(content: &str, score: f32, length_in_range: bool) -> ReviewSnapshot {
        ReviewSnapshot {
            content: content.to_string(),
            word_count: 0,
            audit_result: AuditResult::default(),
            score,
            length_in_range,
        }
    }

    #[test]
    fn is_passed_when_all_conditions_met() {
        let a = make_assessment(true, 90.0, true);
        assert!(is_passed(&a));
    }

    #[test]
    fn is_passed_when_score_below_threshold() {
        let a = make_assessment(true, 80.0, true);
        assert!(!is_passed(&a));
    }

    #[test]
    fn is_passed_when_length_out_of_range() {
        let a = make_assessment(true, 90.0, false);
        assert!(!is_passed(&a));
    }

    #[test]
    fn is_passed_when_audit_not_passed() {
        let a = make_assessment(false, 90.0, true);
        assert!(!is_passed(&a));
    }

    #[test]
    fn pick_best_snapshot_prefers_in_range() {
        let snapshots = [
            make_snapshot("snap1", 90.0, false),
            make_snapshot("snap2", 85.0, true),
        ];
        let best = pick_best_snapshot(&snapshots).expect("应返回 Some");
        assert_eq!(best.content, "snap2");
        assert!(best.length_in_range);
    }

    #[test]
    fn pick_best_snapshot_prefers_higher_score() {
        let snapshots = [
            make_snapshot("snap1", 85.0, true),
            make_snapshot("snap2", 90.0, true),
        ];
        let best = pick_best_snapshot(&snapshots).expect("应返回 Some");
        // 90 >= 85 + 3 → 换
        assert_eq!(best.content, "snap2");
    }

    #[test]
    fn pick_best_snapshot_keeps_best_when_no_improvement() {
        let snapshots = [
            make_snapshot("snap1", 90.0, true),
            make_snapshot("snap2", 92.0, true),
        ];
        let best = pick_best_snapshot(&snapshots).expect("应返回 Some");
        // 92 < 90 + 3 → 不换
        assert_eq!(best.content, "snap1");
    }

    #[test]
    fn add_usage_accumulates() {
        let left = CycleUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let right = CycleUsage {
            prompt_tokens: 200,
            completion_tokens: 100,
            total_tokens: 300,
        };
        let result = add_usage(left, Some(right));
        assert_eq!(result.prompt_tokens, 300);
        assert_eq!(result.completion_tokens, 150);
        assert_eq!(result.total_tokens, 450);
    }

    #[test]
    fn add_usage_handles_none() {
        let left = CycleUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let result = add_usage(left, None);
        assert_eq!(result.prompt_tokens, 100);
        assert_eq!(result.completion_tokens, 50);
        assert_eq!(result.total_tokens, 150);
    }
}
