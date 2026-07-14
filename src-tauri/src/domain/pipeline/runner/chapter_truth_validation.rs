// Chapter Truth Validation —— 章节真相文件持久化校验。
//
// 职责：章节写完后，校验 truth 文件（current_state.md / pending_hooks.md）持久化前的状态连续性。
// 校验失败时触发重试结算层（retry_settlement），重试仍失败则标记章节为 state-degraded。
//
// 流程：
// 1. 调用 state_validator 校验 updated_state/updated_hooks 与 old_state/old_hooks 的矛盾
// 2. 校验通过 → 返回正常结果（chapter_status = None）
// 3. 校验失败 → 调用 retry_settlement 重试结算层
// 4. 重试成功 → 用新的 ValidationResult 返回（chapter_status = None）
// 5. 重试仍失败 → 标记 state-degraded，注入降级问题（chapter_status = "state-degraded"）
//
// 实现差异：
// - WriterAgent 已提供独立 `settle_chapter_state`（delta 模式），但 retry_settlement 需要
//   完整 truth 文件（updated_state/updated_hooks markdown）用于 state_validator 对比。
//   Rust 端尚缺 markdown projection 模块（从 snapshot 渲染 markdown），因此 retry_settlement
//   暂保留独立的 full-truth-file prompt（=== UPDATED_STATE === / === UPDATED_HOOKS ===）。
//   待 projection 模块落地后，迁移 retry_settlement 调用 writer::settle_chapter_state +
//   reducer + projection，统一 settler 入口。
// - 返回值简化为 TruthValidationResult（validation + chapter_status + degraded_issues），
//   调用方负责根据 chapter_status 决定持久化策略。
// - 简化实现不注入 governed artifacts，仅以旧 truth 文件 + 校验反馈作为 settler 输入。

#![allow(unused_imports)]

use std::path::Path;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::super::agents::continuity::{AuditResult, AuditIssue};
use super::super::agents::state_validator::{self, ValidationResult};
use super::super::governance::input::GovernedArtifacts;
use super::super::types::{BookConfig, Language};
use super::chapter_state_recovery::{
    build_state_degraded_issues, build_state_validation_feedback, SettlementRetryResult,
};

// ── 类型 ─────────────────────────────────────────────────────

/// 真相文件持久化前的校验结果
///
/// 简化版：返回值额外包含 persistenceOutput 与 auditResult，
/// Rust 端将这两者的处理职责交给调用方（chapter persistence 模块）。
#[derive(Debug, Clone)]
pub struct TruthValidationResult {
    pub validation: ValidationResult,
    /// 状态降级时为 "state-degraded"，否则 None
    pub chapter_status: Option<String>,
    /// 降级时的注入问题（仅 chapter_status = "state-degraded" 时非空）
    pub degraded_issues: Vec<AuditIssue>,
}

/// 旧真相文件（用于校验对比）
///
/// 旧真相文件字段：
/// - old_state: 结算前的 current_state.md
/// - old_hooks: 结算前的 pending_hooks.md
/// - old_ledger: 结算前的 chapter_summaries.md（降级时回滚使用）
#[derive(Debug, Clone, Default)]
pub struct PreviousTruth {
    pub old_state: String,
    pub old_hooks: String,
    pub old_ledger: String,
}

// ── 主函数 ───────────────────────────────────────────────────

/// 校验章节真相文件持久化。
///
/// # 参数
/// - `engine`: Agent 引擎，用于调用 state_validator 与 settler 重试
/// - `book`: 书籍配置（ settler prompt 需要标题与目标章数）
/// - `book_dir`: 书籍目录（保留参数以对齐签名，简化实现未使用）
/// - `chapter_number`: 章节号
/// - `title`: 章节标题
/// - `content`: 章节正文
/// - `updated_state`: 结算后的 current_state.md 内容
/// - `updated_hooks`: 结算后的 pending_hooks.md 内容
/// - `previous_truth`: 结算前的 truth 文件（用于矛盾对比与降级回滚）
/// - `language`: 输出语言（影响降级 issue 文案）
///
/// # 返回
/// `TruthValidationResult`：校验结果 + 降级状态 + 降级问题
#[allow(clippy::too_many_arguments)]
pub async fn validate_chapter_truth_persistence(
    engine: &AgentEngine,
    book: &BookConfig,
    book_dir: &Path,
    chapter_number: u32,
    title: &str,
    content: &str,
    updated_state: &str,
    updated_hooks: &str,
    previous_truth: &PreviousTruth,
    language: Language,
) -> Result<TruthValidationResult, AppError> {
    // ── 第一次校验：检查 updated_state/updated_hooks 与 old_state/old_hooks 的矛盾 ──
    let validation = state_validator::validate_state(
        engine,
        content,
        chapter_number,
        &previous_truth.old_state,
        updated_state,
        &previous_truth.old_hooks,
        updated_hooks,
    )
    .await?;

    if !validation.warnings.is_empty() {
        tracing::warn!(
            "状态校验：第 {} 章发现 {} 条警告",
            chapter_number,
            validation.warnings.len()
        );
        for warning in &validation.warnings {
            tracing::warn!("  [{}] {}", warning.category, warning.description);
        }
    }

    // 校验通过 → 返回正常结果
    if validation.passed {
        return Ok(TruthValidationResult {
            validation,
            chapter_status: None,
            degraded_issues: Vec::new(),
        });
    }

    // ── 校验失败 → 重试结算层 ──
    let recovery = retry_settlement(
        engine,
        book,
        book_dir,
        chapter_number,
        title,
        content,
        previous_truth,
        &validation,
        language,
    )
    .await?;

    match recovery {
        SettlementRetryResult::Recovered {
            validation: retry_validation,
        } => {
            // 重试成功 → 用新 validation 结果，章节正常
            Ok(TruthValidationResult {
                validation: retry_validation,
                chapter_status: None,
                degraded_issues: Vec::new(),
            })
        }
        SettlementRetryResult::Degraded { issues } => {
            // 重试仍失败 → 标记 state-degraded，注入降级问题
            Ok(TruthValidationResult {
                validation,
                chapter_status: Some(r###"state-degraded"###.to_string()),
                degraded_issues: issues,
            })
        }
    }
}

// ── 重试结算层 ─────────────────────────────────────────────

/// 重试结算层。
///
/// 当前实现：直接调用 engine.prompt_once 触发 full-truth-file settler prompt，
/// 解析 === UPDATED_STATE === / === UPDATED_HOOKS === 得到重试后的完整 truth 文件。
///
/// 注意：writer.rs 已提供 `settle_chapter_state`（delta 模式），但本函数需要完整
/// truth 文件用于 state_validator 对比，且 Rust 端尚缺 markdown projection 模块，
/// 因此暂保留独立 prompt。待 projection 落地后迁移为调用 settle_chapter_state。
///
/// 流程：
/// 1. 用原始 validation 的 warnings 构建校验反馈
/// 2. 调用 engine.prompt_once 触发 full-truth-file settler prompt（含旧 truth + 反馈）
/// 3. 解析 === UPDATED_STATE === 与 === UPDATED_HOOKS === 区块
/// 4. 用新的 truth 文件重新校验
/// 5. 通过 → Recovered；仍失败 → Degraded（携带降级问题）
#[allow(clippy::too_many_arguments)]
async fn retry_settlement(
    engine: &AgentEngine,
    book: &BookConfig,
    _book_dir: &Path,
    chapter_number: u32,
    title: &str,
    content: &str,
    previous_truth: &PreviousTruth,
    original_validation: &ValidationResult,
    language: Language,
) -> Result<SettlementRetryResult, AppError> {
    tracing::warn!(
        "状态校验失败，正在仅重试结算层（第 {} 章）",
        chapter_number
    );

    // 1. 构建校验反馈文本
    let validation_feedback =
        build_state_validation_feedback(&original_validation.warnings, language);

    // 2. 触发 settler 重试
    let system_prompt = build_retry_settler_system_prompt(book);
    let user_message = build_retry_settler_user_message(
        chapter_number,
        title,
        content,
        &previous_truth.old_state,
        &previous_truth.old_hooks,
        &validation_feedback,
    );
    let response = engine.prompt_once(&system_prompt, &user_message).await?;

    // 3. 解析重试后的 truth 文件
    let retry_state = extract_section(&response, r###"UPDATED_STATE"###)
        .ok_or_else(|| AppError::invalid_format(r###"UPDATED_STATE 区块缺失"###))?;
    let retry_hooks = extract_section(&response, r###"UPDATED_HOOKS"###)
        .ok_or_else(|| AppError::invalid_format(r###"UPDATED_HOOKS 区块缺失"###))?;

    // 4. 用新 truth 文件重新校验
    let retry_validation = state_validator::validate_state(
        engine,
        content,
        chapter_number,
        &previous_truth.old_state,
        &retry_state,
        &previous_truth.old_hooks,
        &retry_hooks,
    )
    .await?;

    if !retry_validation.warnings.is_empty() {
        tracing::warn!(
            "状态校验重试后，第 {} 章仍有 {} 条警告",
            chapter_number,
            retry_validation.warnings.len()
        );
        for warning in &retry_validation.warnings {
            tracing::warn!("  [{}] {}", warning.category, warning.description);
        }
    }

    // 5. 通过 → Recovered；仍失败 → Degraded
    if retry_validation.passed {
        return Ok(SettlementRetryResult::Recovered {
            validation: retry_validation,
        });
    }

    Ok(SettlementRetryResult::Degraded {
        issues: build_state_degraded_issues(&retry_validation.warnings, language),
    })
}

// ── Prompt 构建 ─────────────────────────────────────────────

/// 构建重试结算的 system prompt（简化版，对应 writer.rs 的 build_settler_system_prompt）
///
/// 保留 settler 的核心规则：增量更新、严格以正文为准、=== TAG === 输出格式。
/// 简化部分：不注入伏笔追踪规则细节（依赖 settler 通用能力）。
fn build_retry_settler_system_prompt(book: &BookConfig) -> String {
    format!(
        r###"你是状态结算专家。给定章节正文、旧 truth 文件和上一次校验反馈，请重新结算状态。

## 书籍信息
- 标题：{title}
- 目标章数：{target_chapters}章

## 工作模式

你不是在写作。你的任务是：
1. 仔细阅读正文，提取所有状态变化
2. 基于"旧 truth 文件"做增量更新（不是重写）
3. 严格以正文为准，修正校验反馈中提到的矛盾
4. 严格按照 === TAG === 格式输出

## 输出格式（必须严格遵循）

=== UPDATED_STATE ===
（更新后的完整 current_state.md 内容）

=== UPDATED_HOOKS ===
（更新后的完整 pending_hooks.md 内容）

## 铁律

1. 输出完整的更新后内容，不要只输出增量
2. 所有章节号字段必须是整数
3. 只记录正文中实际发生的事，不要推断
4. 严格以正文为准，修正校验反馈中提到的矛盾"###,
        title = book.title,
        target_chapters = book.target_chapters,
    )
}

/// 构建重试结算的 user message
///
/// 包含：章节号 + 标题 + 旧状态卡 + 旧伏笔池 + 校验反馈 + 章节正文
fn build_retry_settler_user_message(
    chapter_number: u32,
    title: &str,
    content: &str,
    old_state: &str,
    old_hooks: &str,
    validation_feedback: &str,
) -> String {
    format!(
        r###"请为第 {chapter_number} 章「{title}」重新结算状态。

## 旧状态卡（结算前）
{old_state}

## 旧伏笔池（结算前）
{old_hooks}

## 上一次校验反馈
{validation_feedback}

## 章节正文
{content}

基于以上信息，输出 UPDATED_STATE 和 UPDATED_HOOKS。"###,
        chapter_number = chapter_number,
        title = title,
        old_state = old_state,
        old_hooks = old_hooks,
        validation_feedback = validation_feedback,
        content = content,
    )
}

/// 从 === TAG === 格式中提取区块内容
///
/// 对应 writer.rs 的 extract_section（私有函数，此处复制以避免修改 writer 模块）。
/// 找到 `=== TAG ===` 标记后，提取到下一个 `=== TAG ===` 或文本结尾的内容。
fn extract_section(content: &str, tag: &str) -> Option<String> {
    let marker = format!(r###"=== {tag} ==="###, tag = tag);
    let start = content.find(&marker)?;
    let content_start = start + marker.len();

    // 找下一个 === TAG === 或文本结尾
    let remaining = &content[content_start..];
    let next_marker = r###"
=== "###;
    let end = remaining
        .find(next_marker)
        .map(|pos| content_start + pos)
        .unwrap_or(content.len());

    Some(content[content_start..end].trim().to_string())
}

// ── 测试 ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_validation(passed: bool, warning_count: usize) -> ValidationResult {
        let warnings: Vec<_> = (0..warning_count)
            .map(|i: usize| state_validator::ValidationWarning {
                category: format!(r###"类别{}"###, i),
                description: format!(r###"描述{}"###, i),
            })
            .collect();
        ValidationResult {
            passed,
            warnings,
        }
    }

    #[test]
    fn truth_validation_result_passes() {
        // 校验通过 → chapter_status = None, degraded_issues 为空
        let result = TruthValidationResult {
            validation: make_validation(true, 0),
            chapter_status: None,
            degraded_issues: Vec::new(),
        };
        assert!(result.validation.passed);
        assert!(result.chapter_status.is_none());
        assert!(result.degraded_issues.is_empty());
    }

    #[test]
    fn truth_validation_result_degraded() {
        // 降级场景 → chapter_status = "state-degraded", degraded_issues 非空
        let degraded_issues = vec![AuditIssue {
            severity: super::super::super::agents::continuity::IssueSeverity::Warning,
            repair_scope: None,
            category: r###"state-validation"###.to_string(),
            description: r###"状态结算重试后仍未通过校验。"###.to_string(),
            suggestion: r###"请先基于已保存正文修复本章 state"###.to_string(),
        }];
        let result = TruthValidationResult {
            validation: make_validation(false, 1),
            chapter_status: Some(r###"state-degraded"###.to_string()),
            degraded_issues,
        };
        assert!(!result.validation.passed);
        assert_eq!(
            result.chapter_status.as_deref(),
            Some(r###"state-degraded"###)
        );
        assert_eq!(result.degraded_issues.len(), 1);
    }

    #[test]
    fn previous_truth_default() {
        // Default 实现：所有字段为空字符串
        let truth = PreviousTruth::default();
        assert!(truth.old_state.is_empty());
        assert!(truth.old_hooks.is_empty());
        assert!(truth.old_ledger.is_empty());
    }

    #[test]
    fn extract_section_finds_tag() {
        // 正常提取：两个区块（中间区块到下个标记为止，最后区块到文本结尾）
        let content = r###"=== UPDATED_STATE ===
这是状态内容。
=== UPDATED_HOOKS ===
这是伏笔内容。"###;
        let state = extract_section(content, r###"UPDATED_STATE"###).expect(r###"应找到 UPDATED_STATE"###);
        assert_eq!(state, r###"这是状态内容。"###);

        let hooks = extract_section(content, r###"UPDATED_HOOKS"###).expect(r###"应找到 UPDATED_HOOKS"###);
        assert_eq!(hooks, r###"这是伏笔内容。"###);
    }

    #[test]
    fn extract_section_returns_none_when_missing() {
        // 缺失标签 → None
        let content = r###"无标签内容"###;
        assert!(extract_section(content, r###"UPDATED_STATE"###).is_none());
    }

    #[test]
    fn extract_section_returns_content_until_end() {
        // 最后一个区块：提取到文本结尾
        let content = r###"=== UPDATED_STATE ===
最后一行内容
没有后续标签"###;
        let state = extract_section(content, r###"UPDATED_STATE"###).expect(r###"应找到"###);
        assert_eq!(state, r###"最后一行内容
没有后续标签"###);
    }
}
