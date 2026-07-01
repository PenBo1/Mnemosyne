use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::features::story::AuditResult;
use crate::infrastructure::state_store::gc::utils;
use crate::infrastructure::file_storage::data_dir::DataDir;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::reviser_prompts;
use super::agent_identity::AgentIdentity;
use super::auto_routing::{AutoOutputMode, resolve_auto_output_mode};
use super::spot_fix_patches::{parse_spot_fix_patches, apply_spot_fix_patches};
use super::style_profile::{load_style_profile, build_deterministic_style_guide};

#[derive(Debug, Clone, PartialEq)]
pub enum ReviseMode {
    Auto,
    Polish,
    Rewrite,
    Rework,
    SpotFix,
    /// 去 AIGC 模式：基于 humanizer 33 模式清单做 draft → audit → final rewrite。
    /// 用于把章节正文里残留的 AI 写作痕迹改写成更自然的人类表达，
    /// 不改剧情、人物动机、关键事件，输出 4 段标记（DRAFT_REWRITE /
    /// STILL_AI_TELLS / FINAL_REWRITE / CHANGES_SUMMARY）。
    DeAigc,
}

impl Default for ReviseMode {
    fn default() -> Self {
        Self::Auto
    }
}

pub struct ReviserAgent;

impl Default for ReviserAgent {
    fn default() -> Self { Self }
}
impl ReviserAgent {
    pub fn new() -> Self { Self }

    /// Revise a chapter based on audit issues.
    ///
    /// Auto 模式会先调用 `resolve_auto_output_mode` 决定输出路由
    /// （PatchOnly / RewriteOnly / AllowFull），然后通过 prompt 约束 LLM
    /// 输出 PATCHES 或 REVISED_CONTENT。输出解析由 `parse_output` 处理：
    /// - PatchOnly：只解析 PATCHES，应用补丁，应用率 >= 50% 才算成功
    /// - RewriteOnly：只解析 REVISED_CONTENT，不回退到 PATCHES
    /// - AllowFull：先尝试 REVISED_CONTENT，再尝试 PATCHES
    pub async fn revise_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        chapter_content: &str,
        audit: &AuditResult,
        mode: ReviseMode,
        data_dir: &DataDir,
    ) -> Result<ReviseOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let identity = AgentIdentity::load(data_dir, "reviser");
        let task_query = format!("revise chapter {} based on audit feedback", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;

        let is_auto = matches!(mode, ReviseMode::Auto);
        let auto_output_mode = if is_auto {
            resolve_auto_output_mode(&audit.issues)
        } else {
            AutoOutputMode::AllowFull
        };

        // DeAigc 模式：若书级存在 style_profile.json，生成确定性文风指南
        // 作为 humanizer Voice Calibration 段的具体数据来源
        let style_guide: Option<String> = if matches!(mode, ReviseMode::DeAigc) {
            load_style_profile(book_dir).map(|p| {
                build_deterministic_style_guide(&p, &language, "DeAIGC voice calibration")
            })
        } else {
            None
        };

        let system = reviser_prompts::build_system_prompt(
            &mode, &language, Some(&identity_prefix), auto_output_mode,
            style_guide.as_deref(),
        );
        let user = reviser_prompts::build_user_message(
            chapter_number,
            chapter_content,
            audit,
            &language,
            is_auto,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let parsed = parse_output(
            &response.content,
            &mode,
            chapter_content,
            auto_output_mode,
        );

        let word_count = utils::count_words(&parsed.content, &language);
        Ok(ReviseOutput {
            chapter_number,
            content: parsed.content,
            word_count,
            fixed_issues: parsed.fixed_issues,
            applied: parsed.applied,
        })
    }
}

#[async_trait]
impl BaseAgent for ReviserAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Reviser
    }

    fn name(&self) -> &str {
        "reviser"
    }
}

pub struct ReviseOutput {
    pub chapter_number: u32,
    pub content: String,
    pub word_count: u32,
    pub fixed_issues: Vec<String>,
    /// 是否实际应用了修改。
    /// - Auto 模式：PATCHES 应用率 >= 50% 或 REVISED_CONTENT 非空才算 true
    /// - Legacy 模式：REVISED_CONTENT 非空或 PATCHES 应用成功才算 true
    /// - false 表示未修改原文（LLM 输出无法应用）
    pub applied: bool,
}

/// 解析 reviser LLM 输出的中间结果。
struct ParsedOutput {
    content: String,
    fixed_issues: Vec<String>,
    applied: bool,
}

/// 解析 reviser 输出，根据模式分流。
///
/// 对齐 inkos `ReviserAgent.parseOutput`：
/// - DeAigc：提取 FINAL_REWRITE 段
/// - Auto + PatchOnly：只解析 PATCHES，应用率 >= 50% 才算成功
/// - Auto + RewriteOnly：只解析 REVISED_CONTENT，不回退到 PATCHES
/// - Auto + AllowFull：先 REVISED_CONTENT，再 PATCHES
/// - SpotFix：只解析 PATCHES
/// - 其他 legacy：解析 REVISED_CONTENT
fn parse_output(
    content: &str,
    mode: &ReviseMode,
    original_chapter: &str,
    auto_output_mode: AutoOutputMode,
) -> ParsedOutput {
    let fixed_issues = extract_fixed_issues(content);

    // DeAigc 走独立 4 段标记工作流
    if matches!(mode, ReviseMode::DeAigc) {
        let final_rewrite = extract_final_rewrite(content);
        let applied = !final_rewrite.is_empty() && final_rewrite != original_chapter;
        return ParsedOutput {
            content: if final_rewrite.is_empty() { original_chapter.to_string() } else { final_rewrite },
            fixed_issues,
            applied,
        };
    }

    // Auto 模式：按路由分流
    if matches!(mode, ReviseMode::Auto) {
        return parse_auto_output(content, original_chapter, auto_output_mode, fixed_issues);
    }

    // Legacy spot-fix：只解析 PATCHES
    if matches!(mode, ReviseMode::SpotFix) {
        let patches_raw = extract_section(content, "PATCHES");
        let patches = parse_spot_fix_patches(&patches_raw);
        let result = apply_spot_fix_patches(original_chapter, &patches);
        return ParsedOutput {
            content: result.revised_content,
            fixed_issues: if result.applied { fixed_issues } else { Vec::new() },
            applied: result.applied,
        };
    }

    // Legacy polish/rewrite/rework：解析 REVISED_CONTENT
    let revised = extract_section(content, "REVISED_CONTENT");
    let applied = !revised.is_empty() && revised != original_chapter;
    ParsedOutput {
        content: if revised.is_empty() { original_chapter.to_string() } else { revised },
        fixed_issues: if applied { fixed_issues } else { Vec::new() },
        applied,
    }
}

/// Auto 模式输出解析：按 auto_output_mode 路由。
fn parse_auto_output(
    content: &str,
    original_chapter: &str,
    auto_output_mode: AutoOutputMode,
    fixed_issues: Vec<String>,
) -> ParsedOutput {
    match auto_output_mode {
        AutoOutputMode::PatchOnly => {
            let patches_raw = extract_section(content, "PATCHES");
            let patches = parse_spot_fix_patches(&patches_raw);
            if !patches.is_empty() {
                let result = apply_spot_fix_patches(original_chapter, &patches);
                // 应用率门槛 50%
                let application_rate = result.applied_patch_count as f64 / patches.len() as f64;
                if result.applied && application_rate >= 0.5 {
                    return ParsedOutput {
                        content: result.revised_content,
                        fixed_issues,
                        applied: true,
                    };
                }
            }
            // PATCHES 失败或未产出 —— 返回原文不改
            ParsedOutput {
                content: original_chapter.to_string(),
                fixed_issues: Vec::new(),
                applied: false,
            }
        }
        AutoOutputMode::RewriteOnly => {
            // 只接受 REVISED_CONTENT，不回退到 PATCHES
            let revised = extract_section(content, "REVISED_CONTENT");
            let applied = !revised.is_empty() && revised != original_chapter;
            ParsedOutput {
                content: if revised.is_empty() { original_chapter.to_string() } else { revised },
                fixed_issues: if applied { fixed_issues } else { Vec::new() },
                applied,
            }
        }
        AutoOutputMode::AllowFull => {
            // 先尝试 REVISED_CONTENT
            let revised = extract_section(content, "REVISED_CONTENT");
            if !revised.is_empty() && revised != original_chapter {
                return ParsedOutput {
                    content: revised,
                    fixed_issues,
                    applied: true,
                };
            }
            // 再尝试 PATCHES
            let patches_raw = extract_section(content, "PATCHES");
            if !patches_raw.is_empty() {
                let patches = parse_spot_fix_patches(&patches_raw);
                if !patches.is_empty() {
                    let result = apply_spot_fix_patches(original_chapter, &patches);
                    let application_rate = result.applied_patch_count as f64 / patches.len() as f64;
                    if result.applied && application_rate >= 0.5 {
                        return ParsedOutput {
                            content: result.revised_content,
                            fixed_issues,
                            applied: true,
                        };
                    }
                }
            }
            // 两者都失败 —— 返回原文不改
            ParsedOutput {
                content: original_chapter.to_string(),
                fixed_issues: Vec::new(),
                applied: false,
            }
        }
    }
}

/// 提取 `=== SECTION ===` 标记包裹的段内容。
///
/// 段结束于下一个 `=== ` 标记或文本末尾。
fn extract_section(content: &str, section: &str) -> String {
    let marker = format!("=== {} ===", section);
    let start = match content.find(&marker) {
        Some(s) => s + marker.len(),
        None => return String::new(),
    };
    let rest = &content[start..];

    // 段结束于下一个 === 标记
    let end = rest.find("\n===").or_else(|| rest.find("==="));
    match end {
        Some(e) => rest[..e].trim().to_string(),
        None => rest.trim().to_string(),
    }
}

/// 提取 FIXED_ISSUES 段并按行拆分。
fn extract_fixed_issues(content: &str) -> Vec<String> {
    let raw = extract_section(content, "FIXED_ISSUES");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 从 DeAigc 工作流的输出中提取 FINAL_REWRITE 段。
///
/// DeAigc 模式输出 4 段标记（DRAFT_REWRITE / STILL_AI_TELLS /
/// FINAL_REWRITE / CHANGES_SUMMARY），只取 FINAL_REWRITE 段作为最终章节内容。
/// 若找不到标记则回退到整段内容（防御性，不应发生）。
fn extract_final_rewrite(content: &str) -> String {
    let marker = "=== FINAL_REWRITE ===";
    let start = match content.find(marker) {
        Some(s) => s + marker.len(),
        None => return String::new(),
    };
    let after = &content[start..];
    // FINAL_REWRITE 结束于 === CHANGES_SUMMARY === 或末尾
    if let Some(end) = after.find("=== CHANGES_SUMMARY ===") {
        return after[..end].trim().to_string();
    }
    // 若没有 CHANGES_SUMMARY，取到末尾
    // 但要检查是否有其他 === 标记
    if let Some(end) = after.find("\n===") {
        return after[..end].trim().to_string();
    }
    after.trim().to_string()
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::{AuditIssue, AuditSeverity, RepairScope};

    fn make_issue(severity: AuditSeverity, category: &str, desc: &str, scope: Option<RepairScope>) -> AuditIssue {
        AuditIssue {
            severity,
            category: category.to_string(),
            description: desc.to_string(),
            suggestion: String::new(),
            repair_scope: scope,
        }
    }

    // ── extract_section ────────────────────────────────────────

    #[test]
    fn test_extract_section_basic() {
        let content = "前缀\n=== REVISED_CONTENT ===\n修订内容\n=== END ===\n后缀";
        assert_eq!(extract_section(content, "REVISED_CONTENT"), "修订内容");
    }

    #[test]
    fn test_extract_section_to_end() {
        let content = "=== REVISED_CONTENT ===\n修订内容到末尾";
        assert_eq!(extract_section(content, "REVISED_CONTENT"), "修订内容到末尾");
    }

    #[test]
    fn test_extract_section_missing_returns_empty() {
        let content = "没有标记的文本";
        assert_eq!(extract_section(content, "REVISED_CONTENT"), "");
    }

    #[test]
    fn test_extract_section_with_patches() {
        let content = "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\nx\nREPLACEMENT_TEXT:\ny\n--- END PATCH ---\n=== END ===";
        let patches_raw = extract_section(content, "PATCHES");
        assert!(patches_raw.contains("--- PATCH 1 ---"));
        assert!(patches_raw.contains("TARGET_TEXT"));
    }

    // ── extract_fixed_issues ──────────────────────────────────

    #[test]
    fn test_extract_fixed_issues_basic() {
        let content = "=== FIXED_ISSUES ===\n- 修复1\n- 修复2\n=== END ===";
        let issues = extract_fixed_issues(content);
        assert_eq!(issues, vec!["- 修复1".to_string(), "- 修复2".to_string()]);
    }

    #[test]
    fn test_extract_fixed_issues_empty() {
        let content = "=== FIXED_ISSUES ===\n\n=== END ===";
        let issues = extract_fixed_issues(content);
        assert!(issues.is_empty());
    }

    // ── extract_final_rewrite ─────────────────────────────────

    #[test]
    fn test_extract_final_rewrite_with_changes_summary() {
        let content = "=== DRAFT_REWRITE ===\n草稿\n=== STILL_AI_TELLS ===\n- 痕迹1\n=== FINAL_REWRITE ===\n最终改写内容\n=== CHANGES_SUMMARY ===\n摘要";
        assert_eq!(extract_final_rewrite(content), "最终改写内容");
    }

    #[test]
    fn test_extract_final_rewrite_without_changes_summary() {
        let content = "=== DRAFT_REWRITE ===\n草稿\n=== FINAL_REWRITE ===\n最终内容";
        assert_eq!(extract_final_rewrite(content), "最终内容");
    }

    #[test]
    fn test_extract_final_rewrite_missing_returns_empty() {
        let content = "没有标记的文本";
        assert_eq!(extract_final_rewrite(content), "");
    }

    // ── parse_output: DeAigc ──────────────────────────────────

    #[test]
    fn test_parse_output_de_aigc() {
        let content = "=== DRAFT_REWRITE ===\n草稿\n=== FINAL_REWRITE ===\n最终内容\n=== CHANGES_SUMMARY ===\n摘要";
        let result = parse_output(content, &ReviseMode::DeAigc, "原文", AutoOutputMode::AllowFull);
        assert_eq!(result.content, "最终内容");
        assert!(result.applied);
    }

    #[test]
    fn test_parse_output_de_aigc_missing_returns_original() {
        let content = "没有任何标记";
        let result = parse_output(content, &ReviseMode::DeAigc, "原文", AutoOutputMode::AllowFull);
        assert_eq!(result.content, "原文");
        assert!(!result.applied);
    }

    // ── parse_output: Auto + PatchOnly ───────────────────────

    #[test]
    fn test_parse_output_auto_patch_only_success() {
        let original = "他走了过去。然后拿了杯子。";
        let content = format!(
            "=== FIXED_ISSUES ===\n- 修复措辞\n=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n他走了过去。\nREPLACEMENT_TEXT:\n他踱到窗前。\n--- END PATCH ---"
        );
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::PatchOnly);
        assert!(result.applied);
        assert_eq!(result.content, "他踱到窗前。然后拿了杯子。");
        assert_eq!(result.fixed_issues, vec!["- 修复措辞"]);
    }

    #[test]
    fn test_parse_output_auto_patch_only_no_patches_returns_original() {
        let original = "原文";
        let content = "=== FIXED_ISSUES ===\n无法安全 patch\n=== PATCHES ===\n(空)";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::PatchOnly);
        assert!(!result.applied);
        assert_eq!(result.content, original);
        assert!(result.fixed_issues.is_empty());
    }

    #[test]
    fn test_parse_output_auto_patch_only_low_application_rate_fails() {
        // 2 个 patch 只命中 1 个（50%），刚好达到阈值 → 应成功
        // 3 个 patch 只命中 1 个（33%），低于阈值 → 应失败
        let original = "原文一。其他。其他。";
        let content = format!(
            "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n原文一\nREPLACEMENT_TEXT:\n替换一\n--- END PATCH ---\n--- PATCH 2 ---\nTARGET_TEXT:\n不存在A\nREPLACEMENT_TEXT:\n替换A\n--- END PATCH ---\n--- PATCH 3 ---\nTARGET_TEXT:\n不存在B\nREPLACEMENT_TEXT:\n替换B\n--- END PATCH ---"
        );
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::PatchOnly);
        // 1/3 = 33% < 50% → 失败
        assert!(!result.applied, "33% application rate should fail");
        assert_eq!(result.content, original);
    }

    #[test]
    fn test_parse_output_auto_patch_only_50_percent_threshold_passes() {
        // 2 个 patch 命中 1 个（50%），刚好达到阈值 → 应成功
        let original = "原文一。其他。";
        let content = format!(
            "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n原文一\nREPLACEMENT_TEXT:\n替换一\n--- END PATCH ---\n--- PATCH 2 ---\nTARGET_TEXT:\n不存在\nREPLACEMENT_TEXT:\n替换\n--- END PATCH ---"
        );
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::PatchOnly);
        assert!(result.applied, "50% application rate should pass");
        assert_eq!(result.content, "替换一。其他。");
    }

    // ── parse_output: Auto + RewriteOnly ─────────────────────

    #[test]
    fn test_parse_output_auto_rewrite_only_success() {
        let original = "原文";
        let content = "=== FIXED_ISSUES ===\n- 重写修复\n=== REVISED_CONTENT ===\n重写后的内容";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::RewriteOnly);
        assert!(result.applied);
        assert_eq!(result.content, "重写后的内容");
    }

    #[test]
    fn test_parse_output_auto_rewrite_only_empty_returns_original() {
        let original = "原文";
        let content = "=== FIXED_ISSUES ===\n无法安全重写\n=== REVISED_CONTENT ===\n";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::RewriteOnly);
        assert!(!result.applied);
        assert_eq!(result.content, original);
    }

    #[test]
    fn test_parse_output_auto_rewrite_only_does_not_fallback_to_patches() {
        // RewriteOnly 模式即使有 PATCHES 也不应用
        let original = "他走了过去。";
        let content = "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n他走了过去。\nREPLACEMENT_TEXT:\n他踱到窗前。\n--- END PATCH ---";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::RewriteOnly);
        assert!(!result.applied, "RewriteOnly must not fall back to patches");
        assert_eq!(result.content, original);
    }

    // ── parse_output: Auto + AllowFull ───────────────────────

    #[test]
    fn test_parse_output_auto_allow_full_prefers_revised_content() {
        let original = "原文";
        let content = "=== REVISED_CONTENT ===\n重写内容";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::AllowFull);
        assert!(result.applied);
        assert_eq!(result.content, "重写内容");
    }

    #[test]
    fn test_parse_output_auto_allow_full_falls_back_to_patches() {
        let original = "他走了过去。";
        let content = "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n他走了过去。\nREPLACEMENT_TEXT:\n他踱到窗前。\n--- END PATCH ---";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::AllowFull);
        assert!(result.applied);
        assert_eq!(result.content, "他踱到窗前。");
    }

    #[test]
    fn test_parse_output_auto_allow_full_both_empty_returns_original() {
        let original = "原文";
        let content = "=== FIXED_ISSUES ===\n无法修复";
        let result = parse_output(&content, &ReviseMode::Auto, original, AutoOutputMode::AllowFull);
        assert!(!result.applied);
        assert_eq!(result.content, original);
    }

    // ── parse_output: Legacy SpotFix ─────────────────────────

    #[test]
    fn test_parse_output_legacy_spot_fix_success() {
        let original = "他走了过去。";
        let content = "=== FIXED_ISSUES ===\n- 修复措辞\n=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n他走了过去。\nREPLACEMENT_TEXT:\n他踱到窗前。\n--- END PATCH ---";
        let result = parse_output(&content, &ReviseMode::SpotFix, original, AutoOutputMode::AllowFull);
        assert!(result.applied);
        assert_eq!(result.content, "他踱到窗前。");
    }

    // ── parse_output: Legacy Polish/Rewrite/Rework ───────────

    #[test]
    fn test_parse_output_legacy_polish_success() {
        let original = "原文";
        let content = "=== REVISED_CONTENT ===\n润色后内容";
        let result = parse_output(&content, &ReviseMode::Polish, original, AutoOutputMode::AllowFull);
        assert!(result.applied);
        assert_eq!(result.content, "润色后内容");
    }

    #[test]
    fn test_parse_output_legacy_rewrite_empty_returns_original() {
        let original = "原文";
        let content = "=== FIXED_ISSUES ===\n无法重写";
        let result = parse_output(&content, &ReviseMode::Rewrite, original, AutoOutputMode::AllowFull);
        assert!(!result.applied);
        assert_eq!(result.content, original);
    }

    // ── resolve_auto_output_mode 集成 ────────────────────────

    #[test]
    fn test_revise_chapter_auto_mode_routing_integration() {
        // 验证 resolve_auto_output_mode 与 parse_output 的端到端协作
        // 结构问题 → RewriteOnly → 只接受 REVISED_CONTENT
        let issues = vec![
            make_issue(AuditSeverity::Critical, "OOC", "人设崩", Some(RepairScope::Structural)),
        ];
        let mode = resolve_auto_output_mode(&issues);
        assert_eq!(mode, AutoOutputMode::RewriteOnly);

        // 模拟 LLM 输出 REVISED_CONTENT
        let content = "=== REVISED_CONTENT ===\n重写后的内容";
        let result = parse_output(content, &ReviseMode::Auto, "原文", mode);
        assert!(result.applied);
        assert_eq!(result.content, "重写后的内容");
    }

    #[test]
    fn test_revise_chapter_auto_mode_patch_only_integration() {
        // 局部问题 → PatchOnly → 只接受 PATCHES
        let issues = vec![
            make_issue(AuditSeverity::Warning, "段落等长", "均匀", Some(RepairScope::Local)),
        ];
        let mode = resolve_auto_output_mode(&issues);
        assert_eq!(mode, AutoOutputMode::PatchOnly);

        // 模拟 LLM 输出 PATCHES
        let original = "原文段落。";
        let content = "=== PATCHES ===\n--- PATCH 1 ---\nTARGET_TEXT:\n原文段落。\nREPLACEMENT_TEXT:\n替换段落。\n--- END PATCH ---";
        let result = parse_output(content, &ReviseMode::Auto, original, mode);
        assert!(result.applied);
        assert_eq!(result.content, "替换段落。");
    }
}
