//! ═══════════════════════════════════════════════════════════════════════════
//! Reviser Agent - 章节修订代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：根据审计意见修订章节。支持 6 种模式：
//! - auto: 自动路由（根据 issue 类型决定 patch-only / rewrite-only / allow-full）
//! - polish: 只改表达、节奏、段落呼吸，不改事实与剧情
//! - rewrite: 重组问题段落，保留原文绝大部分句段
//! - rework: 重构场景推进和冲突组织，不改主设定和大事件
//! - anti-detect: 反 AI 检测改写
//! - spot-fix: 定点修复，只改审稿指出的具体句子

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::continuity::{AuditIssue, IssueSeverity, RepairScope};
use super::super::types::BookConfig;
use super::super::utils::text_parse::extract_section;

/// 修订模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ReviseMode {
    #[default]
    Auto,
    Polish,
    Rewrite,
    Rework,
    AntiDetect,
    SpotFix,
}

impl ReviseMode {
    /// 从字符串解析修订模式
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "polish" => Ok(Self::Polish),
            "rewrite" => Ok(Self::Rewrite),
            "rework" => Ok(Self::Rework),
            "antidetect" => Ok(Self::AntiDetect),
            "spotfix" => Ok(Self::SpotFix),
            other => Err(format!("未知 revise mode: {}", other)),
        }
    }
}

/// Auto 模式下的输出模式（由 issue 类型决定）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoOutputMode {
    PatchOnly,
    RewriteOnly,
    AllowFull,
}

/// Reviser 输出
#[derive(Debug, Clone)]
pub struct ReviseOutput {
    pub revised_content: String,
    pub word_count: u32,
    pub fixed_issues: Vec<String>,
    pub updated_state: String,
    pub updated_hooks: String,
    pub applied: bool,
}

/// Reviser 上下文
pub struct ReviserContext {
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub volume_map: String,
    pub story_frame: String,
    pub book_rules: String,
    pub style_guide: String,
    pub chapter_memo: String,
}

/// 修订一章。
pub async fn revise_chapter(
    engine: &AgentEngine,
    book: &BookConfig,
    chapter_number: u32,
    chapter_content: &str,
    issues: &[AuditIssue],
    mode: ReviseMode,
    ctx: &ReviserContext,
) -> Result<ReviseOutput, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        function = "revise_chapter",
        chapter_number,
        book_id = %book.id,
        mode = ?mode,
        issues_count = issues.len(),
        "入口"
    );

    let auto_output_mode = if mode == ReviseMode::Auto {
        resolve_auto_output_mode(issues)
    } else {
        AutoOutputMode::AllowFull
    };

    let system_prompt = build_system_prompt(book, mode, auto_output_mode);
    let user_message = build_user_message(book, chapter_number, chapter_content, issues, mode, ctx);

    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let result = parse_output(&response, mode, auto_output_mode, chapter_content);

    tracing::info!(
        function = "revise_chapter",
        chapter_number,
        applied = result.applied,
        word_count = result.word_count,
        fixed_count = result.fixed_issues.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "出口"
    );

    Ok(result)
}

// ── Auto 路由：根据 issue 类型决定输出模式 ───────────────────

/// 局部问题模式（patch 可修）
const LOCAL_ONLY_PATTERNS: &[&str] = &[
    "Paragraph uniformity", "段落等长",
    "Hedge density", "套话密度",
    "Formulaic transitions", "公式化转折",
    "List-like structure", "列表式结构",
    "Cross-chapter repetition", "跨章重复",
    "AI-tell word density",
    "Fatigue word", "高疲劳词",
    "Information Boundary", "信息越界",
    "Knowledge Base Pollution", "知识库污染",
];

/// 结构/语义问题模式（必须重写）
const STRUCTURAL_PATTERNS: &[&str] = &[
    "OOC", "人设", "Character Fidelity",
    "Mainline", "主线偏离", "Outline Drift", "Chapter Memo Drift", "章节备忘偏离",
    "Conflict", "冲突乏力", "Payoff Dilution", "爽点虚化",
    "Timeline", "时间线",
    "Hook Check", "伏笔检查", "Hook Debt", "未兑现",
    "Power Scaling", "战力崩坏", "金手指",
    "Pacing", "节奏",
    "POV Consistency", "视角",
    "Subplot Stagnation", "支线停滞", "Arc Flatline", "弧线平坦",
    "Relationship Dynamics", "关系动态",
    "Incentive Chain", "利益链",
    "Canon Event", "正典",
];

fn resolve_auto_output_mode(issues: &[AuditIssue]) -> AutoOutputMode {
    if issues.is_empty() {
        return AutoOutputMode::AllowFull;
    }

    // 优先使用 repair_scope 路由
    let scoped_blocking: Vec<&AuditIssue> = issues
        .iter()
        .filter(|i| i.severity != IssueSeverity::Info && i.repair_scope.is_some())
        .collect();

    if !scoped_blocking.is_empty() {
        if scoped_blocking.iter().any(|i| i.repair_scope == Some(RepairScope::Structural)) {
            return AutoOutputMode::RewriteOnly;
        }
        let all_local = scoped_blocking
            .iter()
            .all(|i| i.repair_scope == Some(RepairScope::Local));
        let blocking_count = issues.iter().filter(|i| i.severity != IssueSeverity::Info).count();
        if all_local && scoped_blocking.len() == blocking_count {
            return AutoOutputMode::PatchOnly;
        }
    }

    // 回退到模式匹配
    let is_structural = |issue: &AuditIssue| {
        let text = format!("{} {}", issue.category, issue.description);
        STRUCTURAL_PATTERNS.iter().any(|p| text.contains(p))
    };
    let is_local = |issue: &AuditIssue| {
        let text = format!("{} {}", issue.category, issue.description);
        LOCAL_ONLY_PATTERNS.iter().any(|p| text.contains(p))
    };

    let blocking: Vec<&AuditIssue> = issues.iter().filter(|i| i.severity != IssueSeverity::Info).collect();
    if blocking.is_empty() {
        return AutoOutputMode::PatchOnly;
    }

    let structural_count = blocking.iter().filter(|i| is_structural(i)).count();
    let local_only_count = blocking.iter().filter(|i| is_local(i)).count();

    if structural_count > 0 {
        return AutoOutputMode::RewriteOnly;
    }
    if local_only_count == blocking.len() {
        return AutoOutputMode::PatchOnly;
    }

    AutoOutputMode::AllowFull
}

// ── System Prompt ────────────────────────────────────────────

fn build_system_prompt(book: &BookConfig, mode: ReviseMode, auto_output_mode: AutoOutputMode) -> String {
    let mode_desc = match mode {
        ReviseMode::Polish => "Polish（润色）：只触碰表达、节奏与段落呼吸——绝不改动事实或剧情结果。允许的操作：词语替换、句子重排、标点节奏微调。",
        ReviseMode::Rewrite => "Rewrite（重写）：可以重组问题段落、调整意象与叙事力度，但应尽量保留原文绝大多数的句子。",
        ReviseMode::Rework => "Rework（重构）：可以重组场景推进与冲突组织，但绝不能改动主设定与大事件结果。",
        ReviseMode::AntiDetect => "Anti-detection（反 AI 检测改写）：保持剧情不变，降低 AI 生成可检测性。打破句式规律、替换为口语化表达、降低\"了\"字密度、降低过渡词频率、外化情绪、删除旁白结论、具体化群众反应、差异化段落长度。",
        ReviseMode::SpotFix => "Spot-fix（定点修复）：只修改审稿指出的具体句子或段落——其余内容必须原封不动。",
        ReviseMode::Auto => "",
    };

    let routing_directive = if mode == ReviseMode::Auto {
        match auto_output_mode {
            AutoOutputMode::RewriteOnly => "\n\n路由指令：审稿给出的阻塞类问题是结构/语义错误（角色逻辑崩坏、主线漂移、缺兑现、时间线错误、伏笔未解、memo 漂移等）。你必须输出 REVISED_CONTENT——禁止输出 PATCHES。",
            AutoOutputMode::PatchOnly => "\n\n路由指令：审稿给出的阻塞类问题是局部错误（措辞、段落形态、疲劳词、信息边界、知识库污染）。你必须只输出 PATCHES——禁止重写整章。",
            AutoOutputMode::AllowFull => "",
        }
    } else {
        ""
    };

    let output_format = if mode == ReviseMode::SpotFix || auto_output_mode == AutoOutputMode::PatchOnly {
        r#"=== FIXED_ISSUES ===
（逐条说明每条修复解决了什么）

=== PATCHES ===
--- PATCH 1 ---
TARGET_TEXT:
（从原文逐字复制的原句或原段落，必须能在原文中唯一匹配）
REPLACEMENT_TEXT:
（局部替换文本）
--- END PATCH ---

=== UPDATED_STATE ===
（完整更新后的状态卡）

=== UPDATED_HOOKS ===
（完整更新后的伏笔池）"#
    } else {
        r#"=== FIXED_ISSUES ===
（逐条说明每条修复解决了什么）

=== REVISED_CONTENT ===
（完整修订后的正文）

=== UPDATED_STATE ===
（完整更新后的状态卡）

=== UPDATED_HOOKS ===
（完整更新后的伏笔池）"#
    };

    format!(
        r#"<identity>
你是一名专业的网文修订编辑。你的任务是根据审计反馈修订章节。
</identity>

## Book Information
- 书名：{title}
- 目标章数：{target_chapters} 章{mode_block}{routing_directive}

<revision_principles>
1. 修复根因——禁止表面润色遮盖问题。
2. 伏笔状态必须与伏笔池同步更新。
3. 不得改变剧情走向或核心冲突。
4. 保留原文的语感、节奏与呼吸——不要压缩过渡段、不要删除减速段落。
5. 通过动作外化情绪（不要写"他很愤怒"——写他做了什么动作）。通过行为传达价值观。
6. 不同角色说话风格必须不同。"众人齐声惊呼"是禁止的。
7. 坏事层层叠加——每一层必须比上一层更糟。
8. 修订后同步更新状态卡与伏笔池。
</revision_principles>

## Small-Goal Cycle Revision Guidance

- 如果本章应处于"余波"阶段但仍在持续施压，把最密集的冲突段落改写成展示变化的段落——谁失去了什么、谁的态度变了、新的常态是什么。
- 如果本章应处于"爆发"阶段但未清晰交付，找到最接近兑现的场景并放大它——让承诺的释放超出读者预期。
- 如果日常段落不服务主线，把它改写成"饵"：插入一个指向未来的细节、一个暗示、一个角色反应。

<safety>
- NEVER 假装修好未解决问题——任何 FIXED_ISSUES 条目必须在原文中可验证地对应一处修改。
- NEVER 在修订中悄悄改变人物动机或主线方向——核心冲突与剧情走向必须与原文保持一致。
- NEVER 输出元层评论（如"已按要求修改"、"希望符合要求"）——只输出区块标记与内容。
</safety>

<examples>
✅ Good（根因修复 + 动作外化）：
- TARGET_TEXT: 他很愤怒，转身离开。
- REPLACEMENT_TEXT: 他没说话，只是把茶碗往桌上一搁，瓷底磕在木面上发出脆响。转身时衣袖扫落了茶盖，他没回头去捡。

❌ Bad（表面润色 + 内心独白代替动作）：
- TARGET_TEXT: 他很愤怒，转身离开。
- REPLACEMENT_TEXT: 他感到十分愤怒和悲伤，内心充满复杂的情绪，于是转身离开了。
</examples>

<verification>
完成修订后请自检：
1. FIXED_ISSUES 中的每一条是否都能在 PATCHES / REVISED_CONTENT 中找到对应的实际修改？
2. UPDATED_STATE 与 UPDATED_HOOKS 是否与修订后的正文同步（伏笔状态、角色状态、关系变化）？
3. 是否在修订中悄悄改变了主线方向或核心冲突？若有，回退。
4. 输出是否只包含 === 区块标记与内容，没有任何元层评论？
若任一项不通过，重新输出。
</verification>

## Output Format

{output_format}"#,
        title = book.title,
        target_chapters = book.target_chapters,
        mode_block = if mode_desc.is_empty() {
            String::new()
        } else {
            format!("\n\n## Revision Mode: {}", mode_desc)
        },
        routing_directive = routing_directive,
        output_format = output_format,
    )
}

// ── User Message ─────────────────────────────────────────────

fn build_user_message(
    _book: &BookConfig,
    chapter_number: u32,
    chapter_content: &str,
    issues: &[AuditIssue],
    mode: ReviseMode,
    ctx: &ReviserContext,
) -> String {
    let issue_list = if mode == ReviseMode::Auto {
        build_tiered_issue_list(issues)
    } else {
        issues
            .iter()
            .map(|i| {
                format!(
                    "- [{:?}] {}: {}\n  建议：{}",
                    i.severity, i.category, i.description, i.suggestion
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let memo_block = if ctx.chapter_memo.is_empty() {
        String::new()
    } else {
        format!("\n## Chapter Memo\n{}\n", ctx.chapter_memo)
    };

    format!(
        r#"请修订第 {chapter_number} 章。

## Audit Issues
{issue_list}

## Current State Card
{current_state}

## Hook Pool
{pending_hooks}

## Chapter Summaries
{chapter_summaries}

## Volume Outline
{volume_map}

## World Setting
{story_frame}

## Rule Card
{book_rules}

## Style Guide
{style_guide}
{memo_block}
## Chapter to Revise
{chapter_content}"#,
        chapter_number = chapter_number,
        issue_list = issue_list,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        volume_map = ctx.volume_map,
        story_frame = ctx.story_frame,
        book_rules = ctx.book_rules,
        style_guide = if ctx.style_guide.is_empty() {
            "（无风格指南）"
        } else {
            &ctx.style_guide
        },
        memo_block = memo_block,
        chapter_content = chapter_content,
    )
}

/// Build a tiered issue list (used in Auto mode).
fn build_tiered_issue_list(issues: &[AuditIssue]) -> String {
    let mut critical = Vec::new();
    let mut high = Vec::new();
    let mut medium = Vec::new();

    for issue in issues {
        let line = format!("- {}: {}", issue.category, issue.description);
        match issue.severity {
            IssueSeverity::Critical => critical.push(line),
            IssueSeverity::Warning => high.push(line),
            IssueSeverity::Info => medium.push(line),
        }
    }

    let mut parts = Vec::new();
    if !critical.is_empty() {
        parts.push(format!("## Critical (必须解决)\n{}", critical.join("\n")));
    }
    if !high.is_empty() {
        parts.push(format!("## High (应当改进)\n{}", high.join("\n")));
    }
    if !medium.is_empty() {
        parts.push(format!("## Medium (参考建议)\n{}", medium.join("\n")));
    }

    parts.join("\n\n")
}

// ── 输出解析 ─────────────────────────────────────────────────

fn parse_output(
    content: &str,
    mode: ReviseMode,
    auto_output_mode: AutoOutputMode,
    original_chapter: &str,
) -> ReviseOutput {
    let extract = |tag: &str| -> String {
        extract_section(content, tag).unwrap_or_default()
    };

    let fixed_raw = extract("FIXED_ISSUES");
    let fixed_issues: Vec<String> = fixed_raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let updated_state = extract("UPDATED_STATE");
    let updated_state = if updated_state.is_empty() {
        "(state card not updated)".to_string()
    } else {
        updated_state
    };

    let updated_hooks = extract("UPDATED_HOOKS");
    let updated_hooks = if updated_hooks.is_empty() {
        "(hook pool not updated)".to_string()
    } else {
        updated_hooks
    };

    // 根据模式决定如何提取修订内容
    let (revised_content, applied) = if mode == ReviseMode::SpotFix
        || auto_output_mode == AutoOutputMode::PatchOnly
    {
        // patch-only 模式：应用 spot-fix patches
        let patches_raw = extract("PATCHES");
        if patches_raw.is_empty() {
            (original_chapter.to_string(), false)
        } else {
            let patches = parse_spot_fix_patches(&patches_raw);
            if patches.is_empty() {
                (original_chapter.to_string(), false)
            } else {
                let (result, applied_count) = apply_spot_fix_patches(original_chapter, &patches);
                let applied = applied_count > 0 && (applied_count as f32 / patches.len() as f32) >= 0.5;
                (result, applied)
            }
        }
    } else {
        // rewrite 模式：提取 REVISED_CONTENT
        let revised = extract("REVISED_CONTENT");
        if revised.is_empty() {
            (original_chapter.to_string(), false)
        } else {
            (revised, true)
        }
    };

    let word_count = revised_content.chars().filter(|c| !c.is_whitespace()).count() as u32;

    ReviseOutput {
        revised_content,
        word_count,
        fixed_issues: if applied { fixed_issues } else { Vec::new() },
        updated_state,
        updated_hooks,
        applied,
    }
}

// ── Spot-fix patch 解析与应用 ────────────────────────────────

/// Spot-fix 补丁
struct SpotFixPatch {
    target_text: String,
    replacement_text: String,
}

/// 解析 PATCHES 区块
fn parse_spot_fix_patches(patches_raw: &str) -> Vec<SpotFixPatch> {
    let mut patches = Vec::new();
    let mut current_target = String::new();
    let mut current_replacement = String::new();
    let mut in_patch = false;
    let mut in_target = false;
    let mut in_replacement = false;

    for line in patches_raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--- PATCH") && !trimmed.starts_with("--- END") {
            if in_patch && !current_target.is_empty() {
                patches.push(SpotFixPatch {
                    target_text: current_target.trim().to_string(),
                    replacement_text: current_replacement.trim().to_string(),
                });
            }
            current_target.clear();
            current_replacement.clear();
            in_patch = true;
            in_target = false;
            in_replacement = false;
        } else if trimmed == "TARGET_TEXT:" {
            in_target = true;
            in_replacement = false;
        } else if trimmed == "REPLACEMENT_TEXT:" {
            in_target = false;
            in_replacement = true;
        } else if trimmed.starts_with("--- END PATCH") {
            if in_patch && !current_target.is_empty() {
                patches.push(SpotFixPatch {
                    target_text: current_target.trim().to_string(),
                    replacement_text: current_replacement.trim().to_string(),
                });
            }
            in_patch = false;
            in_target = false;
            in_replacement = false;
            current_target.clear();
            current_replacement.clear();
        } else if in_target {
            if !current_target.is_empty() {
                current_target.push('\n');
            }
            current_target.push_str(line);
        } else if in_replacement {
            if !current_replacement.is_empty() {
                current_replacement.push('\n');
            }
            current_replacement.push_str(line);
        }
    }

    // 处理未关闭的最后一个 patch
    if in_patch && !current_target.is_empty() {
        patches.push(SpotFixPatch {
            target_text: current_target.trim().to_string(),
            replacement_text: current_replacement.trim().to_string(),
        });
    }

    patches
}

/// 应用 spot-fix 补丁到原文。
///
/// 仅替换每个 target_text 的**首次**出现（replacen(_, _, 1)），避免章节中
/// 多次出现相同子串时被全部替换导致意外破坏。LLM 应在 target_text 中提供足够
/// 上下文以唯一定位目标片段。
fn apply_spot_fix_patches(original: &str, patches: &[SpotFixPatch]) -> (String, usize) {
    let mut result = original.to_string();
    let mut applied_count = 0;

    for patch in patches {
        if patch.target_text.is_empty() {
            continue;
        }
        if result.contains(&patch.target_text) {
            result = result.replacen(&patch.target_text, &patch.replacement_text, 1);
            applied_count += 1;
        }
    }

    (result, applied_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_auto_mode_empty_issues() {
        let mode = resolve_auto_output_mode(&[]);
        assert_eq!(mode, AutoOutputMode::AllowFull);
    }

    #[test]
    fn resolves_auto_mode_structural_issue() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::Critical,
            repair_scope: Some(RepairScope::Structural),
            category: "OOC检查".to_string(),
            description: "主角人设崩".to_string(),
            suggestion: "重写".to_string(),
        }];
        let mode = resolve_auto_output_mode(&issues);
        assert_eq!(mode, AutoOutputMode::RewriteOnly);
    }

    #[test]
    fn resolves_auto_mode_local_issue() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::Warning,
            repair_scope: Some(RepairScope::Local),
            category: "段落等长".to_string(),
            description: "段落长度均匀".to_string(),
            suggestion: "调整段落长度".to_string(),
        }];
        let mode = resolve_auto_output_mode(&issues);
        assert_eq!(mode, AutoOutputMode::PatchOnly);
    }

    #[test]
    fn parses_rewrite_output() {
        let content = r#"=== FIXED_ISSUES ===
- 修复了 OOC 问题

=== REVISED_CONTENT ===
这是修正后的正文。

=== UPDATED_STATE ===
状态卡更新

=== UPDATED_HOOKS ===
伏笔池更新"#;
        let output = parse_output(content, ReviseMode::Rewrite, AutoOutputMode::AllowFull, "原文");
        assert!(output.applied);
        assert_eq!(output.revised_content, "这是修正后的正文。");
        assert_eq!(output.fixed_issues.len(), 1);
        assert_eq!(output.updated_state, "状态卡更新");
    }

    #[test]
    fn parses_spot_fix_patches() {
        let patches_raw = r#"--- PATCH 1 ---
TARGET_TEXT:
他要走了。
REPLACEMENT_TEXT:
他转身离开，脚步声在空旷的走廊里回荡。
--- END PATCH ---

--- PATCH 2 ---
TARGET_TEXT:
她笑了。
REPLACEMENT_TEXT:
她嘴角微微上扬，眼里却闪过一丝不易察觉的疲惫。
--- END PATCH ---"#;
        let patches = parse_spot_fix_patches(patches_raw);
        assert_eq!(patches.len(), 2);
        assert_eq!(patches[0].target_text, "他要走了。");
        assert_eq!(patches[0].replacement_text, "他转身离开，脚步声在空旷的走廊里回荡。");
    }

    #[test]
    fn applies_spot_fix_patches_to_original() {
        let original = "他要走了。她笑了。";
        let patches = vec![
            SpotFixPatch {
                target_text: "他要走了。".to_string(),
                replacement_text: "他转身离开。".to_string(),
            },
            SpotFixPatch {
                target_text: "她笑了。".to_string(),
                replacement_text: "她微笑了。".to_string(),
            },
        ];
        let (result, applied) = apply_spot_fix_patches(original, &patches);
        assert_eq!(applied, 2);
        assert_eq!(result, "他转身离开。她微笑了。");
    }

    #[test]
    fn handles_empty_revised_content() {
        let content = "=== FIXED_ISSUES ===\n- 无\n\n=== UPDATED_STATE ===\n无变化";
        let output = parse_output(content, ReviseMode::Rewrite, AutoOutputMode::AllowFull, "原文内容");
        assert!(!output.applied);
        assert_eq!(output.revised_content, "原文内容");
    }

    #[test]
    fn builds_tiered_issue_list_correctly() {
        let issues = vec![
            AuditIssue {
                severity: IssueSeverity::Critical,
                repair_scope: None,
                category: "OOC".to_string(),
                description: "人设崩".to_string(),
                suggestion: "重写".to_string(),
            },
            AuditIssue {
                severity: IssueSeverity::Warning,
                repair_scope: None,
                category: "节奏".to_string(),
                description: "太慢".to_string(),
                suggestion: "加速".to_string(),
            },
            AuditIssue {
                severity: IssueSeverity::Info,
                repair_scope: None,
                category: "文风".to_string(),
                description: "可优化".to_string(),
                suggestion: "微调".to_string(),
            },
        ];
        let list = build_tiered_issue_list(&issues);
        assert!(list.contains("Critical"));
        assert!(list.contains("High"));
        assert!(list.contains("Medium"));
    }
}
