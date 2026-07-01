use crate::core::agent::reviser::ReviseMode;
use crate::core::agent::auto_routing::AutoOutputMode;
use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};
use crate::core::agent::prompts::aigc_patterns::de_aigc_rewrite_guidance;
use crate::features::story::{AuditResult, AuditSeverity};

/// 构建 reviser 的 system prompt。
///
/// 三种分支：
/// - `DeAigc`：独立工作流，自带输出契约；若提供 `style_guide` 则作为声音校准数据注入
/// - `Auto`：带路由指令的 auto 模式（PatchOnly/RewriteOnly/AllowFull）
/// - 其他：legacy 模式（polish/rewrite/rework/spot-fix）
pub fn build_system_prompt(
    mode: &ReviseMode,
    language: &str,
    identity_prefix: Option<&str>,
    auto_output_mode: AutoOutputMode,
    style_guide: Option<&str>,
) -> String {
    // DeAigc 模式走独立工作流，自带输出契约，不复用通用修订模板
    if matches!(mode, ReviseMode::DeAigc) {
        let header = match language {
            "en" => "You are a de-AIGC rewrite specialist. Remove AI writing-pattern tells from the chapter prose using the humanizer draft → audit → final rewrite workflow documented below.",
            _ => "你是一位去 AIGC 改写专家。按下方 humanizer 的 draft → audit → final rewrite 工作流，去除章节正文里残留的 AI 写作痕迹。",
        };
        // 若有书级风格指纹，作为声音校准数据附加到 humanizer 指导后
        let voice_section = match style_guide {
            Some(g) => match language {
                "en" => format!("\n\n## Voice Calibration Data (from book style profile)\n\n{}", g),
                _ => format!("\n\n## 声音校准数据（来自书级风格指纹）\n\n{}", g),
            },
            None => String::new(),
        };
        let body = format!("{}\n\n{}{}", header, de_aigc_rewrite_guidance(language), voice_section);
        return assemble_with_identity(identity_prefix, &body);
    }

    if matches!(mode, ReviseMode::Auto) {
        return build_auto_system_prompt(language, auto_output_mode, identity_prefix);
    }

    build_legacy_system_prompt(mode, language, identity_prefix)
}

/// Auto 模式 system prompt：带路由指令 + PATCHES/REVISED_CONTENT 双格式。
fn build_auto_system_prompt(
    language: &str,
    auto_output_mode: AutoOutputMode,
    identity_prefix: Option<&str>,
) -> String {
    let routing_directive = match (auto_output_mode, language) {
        (AutoOutputMode::RewriteOnly, "en") => "\n\nROUTING: The reviewer's blocking issues are structural / semantic (character collapse, mainline drift, missing payoff, timeline break, unpaid hook, memo drift, etc.). You MUST output REVISED_CONTENT — do not emit PATCHES, they cannot fix this class of problem. If you cannot safely rewrite, say so in FIXED_ISSUES and leave REVISED_CONTENT empty.",
        (AutoOutputMode::RewriteOnly, _) => "\n\n分流指令：reviewer 报告的阻塞问题属于结构/语义错（人设崩、主线偏、爽点缺、时间线错、伏笔未收、memo 偏离等）。你必须输出 REVISED_CONTENT——禁止输出 PATCHES，这类问题不能靠补丁修复。如果无法安全重写，在 FIXED_ISSUES 里说明并留空 REVISED_CONTENT。",
        (AutoOutputMode::PatchOnly, "en") => "\n\nROUTING: The reviewer's blocking issues are local (wording, paragraph shape, fatigue word, information boundary, knowledge pollution). You MUST output PATCHES only — do not rewrite the whole chapter. If patches are not possible, leave PATCHES empty.",
        (AutoOutputMode::PatchOnly, _) => "\n\n分流指令：reviewer 报告的阻塞问题属于局部错（措辞、段落形状、疲劳词、信息越界、知识污染）。你必须只输出 PATCHES——不要整章改写。如果做不出补丁，留空 PATCHES。",
        (AutoOutputMode::AllowFull, "en") => "\n\nROUTING: The reviewer's blocking issues are mixed or untyped. Choose PATCHES for local issues or REVISED_CONTENT for whole-chapter issues — you may use both if needed.",
        (AutoOutputMode::AllowFull, _) => "\n\n分流指令：reviewer 报告的阻塞问题混合或未标注类型。局部问题用 PATCHES，全章级问题用 REVISED_CONTENT，必要时两者都可输出。",
    };

    let task_prompt = match language {
        "en" => format!(
            r#"You are a professional web-fiction revision editor. Fix the chapter according to the review notes.{routing}

PATCHES and REVISED_CONTENT serve different problems — choose by problem type, not preference:

PATCHES — for local text issues (wording, dialogue, AI-tell phrases, small continuity errors).
  Each PATCH quotes the passage to change (a sentence, a paragraph, or multiple paragraphs) and provides a replacement. Untouched text stays exactly as-is.

REVISED_CONTENT — for whole-chapter issues (length compression, structural rewrite, pacing restructure, major plot realignment).
  Outputs the full revised chapter. When Critical issues include length or structural problems, you must use REVISED_CONTENT — patches cannot compress or restructure a chapter.

If Critical issues include both local and whole-chapter problems, use REVISED_CONTENT (it addresses everything in one pass).

Revision principles:
1. Fix root causes — do not apply superficial polish
2. Hook status must stay in sync with the hooks board. If hook debt briefs are provided, preserve hook payoff scenes
3. Do not alter the plot direction or core conflicts
4. Preserve the original language style, rhythm, and pacing — do not compress transitional scenes or remove breathing room
5. Emotion through action (never "he felt angry" — show it). Values through behavior, not slogans
6. Different characters speak differently. No "everyone gasped in unison"
7. Escalate: bad things stack, each worse than the last

Output format:

=== FIXED_ISSUES ===
(List each fix on its own line; if a safe local fix is not possible, explain here)

=== PATCHES ===
(Output local patches if applicable. Omit this section entirely if using REVISED_CONTENT)
--- PATCH 1 ---
TARGET_TEXT:
(Exact quote from the original that identifies the passage to change)
REPLACEMENT_TEXT:
(Replacement text for this passage)
--- END PATCH ---

=== REVISED_CONTENT ===
(Full revised chapter content — only when PATCHES cannot solve the problem. Omit this section if using PATCHES)"#,
            routing = routing_directive
        ),
        _ => format!(
            r#"你是一位专业的网络小说修稿编辑。你的任务是根据审稿意见对章节进行修正。{routing}

PATCHES 和 REVISED_CONTENT 分别处理不同类型的问题——按问题类型选择，不是按偏好：

PATCHES——处理局部文字问题（措辞、对话、AI痕迹、小的连续性错误）。
  每个 PATCH 引用要修改的原文段落（一句、一段或多段皆可），给出替换文本。未涉及的内容保持原样。

REVISED_CONTENT——处理全章级问题（字数压缩、结构重组、节奏重排、重大剧情偏离）。
  输出修正后的完整正文。当 Critical 问题包含字数或结构性问题时，必须使用 REVISED_CONTENT——PATCHES 无法压缩或重构整章。

如果 Critical 同时包含局部问题和全章问题，使用 REVISED_CONTENT（一次性解决所有问题）。

修稿原则：
1. 修根因，不做表面润色
2. 伏笔状态必须与伏笔池同步。如果提供了 Hook Debt 简报，必须保留伏笔兑现段落
3. 不改变剧情走向和核心冲突
4. 保持原文的语言风格、节奏和呼吸——不要压缩过渡段、不要删掉减速段
5. 情绪用动作外化（不写"他感到愤怒"，写动作）。价值观通过行为传达
6. 不同角色说话方式必须不同。禁止"众人齐声惊呼"
7. 坏事叠坏事，每层比上一层过分

输出格式：

=== FIXED_ISSUES ===
(逐条说明修正了什么)

=== PATCHES ===
(局部补丁——仅用于局部文字问题。有全章级问题时省略此区块)
--- PATCH 1 ---
TARGET_TEXT:
(从原文中精确引用要修改的段落)
REPLACEMENT_TEXT:
(替换后的文本)
--- END PATCH ---

=== REVISED_CONTENT ===
(修正后的完整正文——用于字数/结构/节奏等全章级问题。仅局部问题时省略此区块)"#,
            routing = routing_directive
        ),
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

/// Legacy 模式 system prompt（polish/rewrite/rework/spot-fix）。
fn build_legacy_system_prompt(
    mode: &ReviseMode,
    language: &str,
    identity_prefix: Option<&str>,
) -> String {
    let mode_desc = match mode {
        ReviseMode::Polish => match language {
            "en" => "Polish mode: Only improve expression, rhythm, and paragraph breathing. Do NOT change facts or plot conclusions.",
            _ => "润色模式：只改表达、节奏、段落呼吸，不改事实与剧情结论。",
        },
        ReviseMode::Rewrite => match language {
            "en" => "Rewrite mode: Allow restructuring problem paragraphs, adjusting imagery and narrative intensity. Preserve core facts and character motivations.",
            _ => "改写模式：允许重组问题段落、调整画面和叙述力度。保留核心事实与人物动机。",
        },
        ReviseMode::Rework => match language {
            "en" => "Rework mode: Can reconstruct scene progression and conflict organization, but do NOT change main settings or major event outcomes.",
            _ => "重写模式：可重构场景推进和冲突组织，但不改主设定和大事件结果。",
        },
        ReviseMode::SpotFix => match language {
            "en" => "Spot-fix mode: Make targeted fixes only. Do NOT rewrite any paragraphs — patch specific words, phrases, or sentences.",
            _ => "定点修复模式：只做针对性修复。不要重写任何段落——只修补特定的词、短语或句子。",
        },
        _ => unreachable!("Auto/DeAigc handled above"),
    };

    let is_spot_fix = matches!(mode, ReviseMode::SpotFix);

    let output_format = if is_spot_fix {
        match language {
            "en" => r#"## Output

=== FIXED_ISSUES ===
(List each fix on its own line; if a safe spot-fix is not possible, explain here)

=== PATCHES ===
--- PATCH 1 ---
TARGET_TEXT:
(Exact quote from the original — must uniquely identify the passage)
REPLACEMENT_TEXT:
(Replacement text)
--- END PATCH ---"#,
            _ => r#"## 输出

=== FIXED_ISSUES ===
(逐条说明修正了什么，一行一条；如果无法安全定点修复，也在这里说明)

=== PATCHES ===
--- PATCH 1 ---
TARGET_TEXT:
(必须从原文中精确复制、且能唯一命中的原句或原段)
REPLACEMENT_TEXT:
(替换后的局部文本)
--- END PATCH ---"#,
        }
    } else {
        match language {
            "en" => r#"## Output

Return the full revised chapter text wrapped in === REVISED_CONTENT === markers."#,
            _ => r#"## 输出

返回完整的修订后章节文本，用 === REVISED_CONTENT === 标记包裹。"#,
        }
    };

    let task_prompt = match language {
        "en" => format!(
            r#"You are a revision specialist. Revise the chapter based on the audit feedback.

## Revision mode
{}

## Revision principles
1. Fix all Critical issues first
2. Then fix Warning issues
3. Preserve the original style and tone
4. Minimize changes — only modify what's necessary
5. Do NOT introduce new problems

{}"#,
            mode_desc, output_format
        ),
        _ => format!(
            r#"你是一位修订专家。根据审计反馈修订章节内容。

## 修订模式
{}

## 修订原则
1. 先修复所有 Critical 问题
2. 再修复 Warning 问题
3. 保持原文风格和语气
4. 最小化改动——只修改必要的部分
5. 不要引入新问题

{}"#,
            mode_desc, output_format
        ),
    };

    let body = format!("{}\n\n{}", task_prompt, output_discipline(language));
    assemble_with_identity(identity_prefix, &body)
}

/// 构建 user message：包含章节正文 + 审计结果。
///
/// Auto 模式使用分层 issue 列表（Critical/High/Medium），
/// 其他模式使用平铺列表（含 repair_scope 标注）。
pub fn build_user_message(
    chapter_number: u32,
    chapter_content: &str,
    audit: &AuditResult,
    language: &str,
    is_auto_mode: bool,
) -> String {
    let issues_summary: Vec<String> = if is_auto_mode {
        build_tiered_issue_list(audit, language)
    } else {
        audit.issues.iter().map(|i| {
            let severity_str = match i.severity {
                AuditSeverity::Critical => "CRITICAL",
                AuditSeverity::Warning => "WARNING",
                AuditSeverity::Info => "INFO",
            };
            let scope_str = match i.repair_scope {
                Some(crate::features::story::RepairScope::Local) => " [local]",
                Some(crate::features::story::RepairScope::Structural) => " [structural]",
                Some(crate::features::story::RepairScope::Unknown) => " [unknown]",
                None => "",
            };
            format!("[{}]{} {}: {}", severity_str, scope_str, i.category, i.description)
        }).collect()
    };

    let heading = if language == "en" {
        format!("Chapter {}", chapter_number)
    } else {
        format!("第{}章", chapter_number)
    };

    let audit_label = if language == "en" { "Audit result" } else { "审计结果" };
    let passed_label = if language == "en" { "Passed" } else { "通过" };
    let score_label = if language == "en" { "Score" } else { "分数" };
    let issues_label = if language == "en" { "Issues to fix" } else { "需要修复的问题" };

    format!(
        "## {} {}\n\n{}\n\n## {}\n\n{}: {}\n{}: {}/100\n\n{}:\n{}",
        heading,
        if language == "en" { "Body" } else { "正文" },
        chapter_content,
        audit_label,
        passed_label, audit.passed,
        score_label, audit.score,
        issues_label,
        issues_summary.join("\n")
    )
}

/// 构建 auto 模式的分层 issue 列表（Critical/High/Medium）。
fn build_tiered_issue_list(audit: &AuditResult, language: &str) -> Vec<String> {
    let en = language == "en";
    let mut critical: Vec<String> = Vec::new();
    let mut high: Vec<String> = Vec::new();
    let mut medium: Vec<String> = Vec::new();

    for issue in &audit.issues {
        let scope_str = match issue.repair_scope {
            Some(crate::features::story::RepairScope::Local) => " [local]",
            Some(crate::features::story::RepairScope::Structural) => " [structural]",
            Some(crate::features::story::RepairScope::Unknown) => " [unknown]",
            None => "",
        };
        let line = format!("- {}{}: {}", issue.category, scope_str, issue.description);
        match issue.severity {
            AuditSeverity::Critical => critical.push(line),
            AuditSeverity::Warning => high.push(line),
            AuditSeverity::Info => medium.push(line),
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if !critical.is_empty() {
        let header = if en { "## Critical — Must Fix" } else { "## Critical（必须解决）" };
        parts.push(format!("{}\n{}", header, critical.join("\n")));
    }
    if !high.is_empty() {
        let header = if en { "## High — Should Improve" } else { "## High（应当改善）" };
        parts.push(format!("{}\n{}", header, high.join("\n")));
    }
    if !medium.is_empty() {
        let header = if en { "## Medium — Reference" } else { "## Medium（参考建议）" };
        parts.push(format!("{}\n{}", header, medium.join("\n")));
    }

    if parts.is_empty() {
        vec![if en { "(no issues)" } else { "（无问题）" }.to_string()]
    } else {
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::story::{AuditIssue, RepairScope};

    fn make_audit(passed: bool, score: f64, issues: Vec<AuditIssue>) -> AuditResult {
        AuditResult {
            passed, score, issues,
            summary: String::new(),
            parse_failed: false,
        }
    }

    fn make_issue(severity: AuditSeverity, category: &str, desc: &str, scope: Option<RepairScope>) -> AuditIssue {
        AuditIssue {
            severity,
            category: category.to_string(),
            description: desc.to_string(),
            suggestion: String::new(),
            repair_scope: scope,
        }
    }

    #[test]
    fn test_build_system_prompt_de_aigc_mode() {
        let prompt = build_system_prompt(&ReviseMode::DeAigc, "zh", None, AutoOutputMode::AllowFull, None);
        assert!(prompt.contains("去 AIGC 改写专家"));
    }

    #[test]
    fn test_build_system_prompt_de_aigc_with_style_guide() {
        let guide = "# 文风指南\n\n> DeAIGC voice calibration\n\n## 统计风格指纹\n- 平均句长：12.5";
        let prompt = build_system_prompt(&ReviseMode::DeAigc, "zh", None, AutoOutputMode::AllowFull, Some(guide));
        assert!(prompt.contains("去 AIGC 改写专家"));
        assert!(prompt.contains("## 声音校准数据（来自书级风格指纹）"));
        assert!(prompt.contains("平均句长：12.5"));
    }

    #[test]
    fn test_build_system_prompt_de_aigc_with_style_guide_en() {
        let guide = "# Style Guide\n\n> DeAIGC voice calibration\n\n## Statistical Fingerprint\n- Average sentence length: 12.5";
        let prompt = build_system_prompt(&ReviseMode::DeAigc, "en", None, AutoOutputMode::AllowFull, Some(guide));
        assert!(prompt.contains("de-AIGC rewrite specialist"));
        assert!(prompt.contains("## Voice Calibration Data (from book style profile)"));
        assert!(prompt.contains("Average sentence length: 12.5"));
    }

    #[test]
    fn test_build_system_prompt_auto_rewrite_only() {
        let prompt = build_system_prompt(&ReviseMode::Auto, "zh", None, AutoOutputMode::RewriteOnly, None);
        assert!(prompt.contains("分流指令"));
        assert!(prompt.contains("REVISED_CONTENT"));
        assert!(prompt.contains("禁止输出 PATCHES"));
    }

    #[test]
    fn test_build_system_prompt_auto_patch_only() {
        let prompt = build_system_prompt(&ReviseMode::Auto, "zh", None, AutoOutputMode::PatchOnly, None);
        assert!(prompt.contains("只输出 PATCHES"));
    }

    #[test]
    fn test_build_system_prompt_auto_allow_full() {
        let prompt = build_system_prompt(&ReviseMode::Auto, "zh", None, AutoOutputMode::AllowFull, None);
        assert!(prompt.contains("混合或未标注类型"));
    }

    #[test]
    fn test_build_system_prompt_legacy_polish() {
        let prompt = build_system_prompt(&ReviseMode::Polish, "zh", None, AutoOutputMode::AllowFull, None);
        assert!(prompt.contains("润色模式"));
        assert!(prompt.contains("=== REVISED_CONTENT ==="));
    }

    #[test]
    fn test_build_system_prompt_legacy_spot_fix() {
        let prompt = build_system_prompt(&ReviseMode::SpotFix, "zh", None, AutoOutputMode::AllowFull, None);
        assert!(prompt.contains("定点修复"));
        assert!(prompt.contains("=== PATCHES ==="));
        assert!(prompt.contains("TARGET_TEXT"));
    }

    #[test]
    fn test_build_user_message_auto_mode_tiered() {
        let audit = make_audit(false, 60.0, vec![
            make_issue(AuditSeverity::Critical, "OOC", "人设崩", Some(RepairScope::Structural)),
            make_issue(AuditSeverity::Warning, "措辞", "用词不当", Some(RepairScope::Local)),
            make_issue(AuditSeverity::Info, "建议", "可优化", None),
        ]);
        let msg = build_user_message(1, "正文内容", &audit, "zh", true);
        assert!(msg.contains("## Critical（必须解决）"));
        assert!(msg.contains("## High（应当改善）"));
        assert!(msg.contains("## Medium（参考建议）"));
        assert!(msg.contains("[structural]"));
        assert!(msg.contains("[local]"));
    }

    #[test]
    fn test_build_user_message_legacy_mode_flat() {
        let audit = make_audit(false, 60.0, vec![
            make_issue(AuditSeverity::Critical, "OOC", "人设崩", Some(RepairScope::Structural)),
        ]);
        let msg = build_user_message(1, "正文内容", &audit, "zh", false);
        // legacy 模式不分层，只列平铺 issue
        assert!(!msg.contains("## Critical（必须解决）"));
        assert!(msg.contains("[CRITICAL] [structural] OOC: 人设崩"));
    }

    #[test]
    fn test_build_user_message_en() {
        let audit = make_audit(true, 95.0, vec![]);
        let msg = build_user_message(1, "body", &audit, "en", false);
        assert!(msg.contains("Chapter 1"));
        assert!(msg.contains("Passed: true"));
    }
}
