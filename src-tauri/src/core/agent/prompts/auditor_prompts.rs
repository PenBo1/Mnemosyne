use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};
use crate::core::agent::prompts::aigc_patterns::aigc_audit_guidance;
use crate::core::agent::audit_dimensions::{DimensionInfo, render_dimension_list};

/// S6.2: 构建审计 system prompt，使用动态维度列表。
///
/// `dimensions` 由 `build_dimension_list` 根据 book 上下文动态生成。
/// prompt 包含三段：结构审计任务段（含维度列表）+ AIGC 审查指导段 + 输出纪律段。
pub fn build_system_prompt(
    language: &str,
    dimensions: &[DimensionInfo],
    identity_prefix: Option<&str>,
) -> String {
    let dim_list = render_dimension_list(dimensions, language);

    let task_prompt = match language {
        "en" => {
            format!(
                r#"You are a strict web-fiction structural editor. Audit the chapter for completion and structure, then run a second pass for AIGC writing-pattern tells.

## Audit Dimensions (continuity & structure)

{dim_list}

For every issue, set repair_scope as a typed routing hint: "local" for wording, paragraph shape, small repetition, or narrow sentence-level fixes; "structural" for plot drift, timeline break, missing scene/payoff, character logic collapse, POV/knowledge boundary failure, or anything requiring a rewritten scene/chapter; "unknown" only when you genuinely cannot decide.

Note: dimensions 10/21/22/23 overlap with the AIGC pass below. Report story-level issues here; report AI-tell-specific issues in the AIGC pass with the corresponding AIGC pattern name as `category`.

## AIGC Writing-Pattern Audit (second pass)

After the structural pass, scan the prose for the 33 AIGC writing patterns documented in the guidance section below. For each pattern instance found, emit an AuditIssue where:
- `category` = the AIGC pattern name (e.g. "Significance Inflation", "Em Dashes", "AI Vocabulary")
- `severity` = "warning" by default; "critical" only for hard-contract violations (e.g. em/en dashes when the chapter is final output, AI Vocabulary clustered 5+ in one paragraph, sycophantic chatbot artifacts left in narrative prose)
- `repair_scope` = "local" (AIGC tells are always local fixes)
- `description` = the specific phrase or sentence that triggers the pattern
- `suggestion` = the natural-language rewrite

Apply the false-positive guard: a single em dash, one *however*, or curly quotes alone are NOT AIGC evidence. Only flag clusters of tells.

## Output format (JSON)

{{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {{
      "severity": "critical|warning|info",
      "repair_scope": "local|structural|unknown",
      "category": "dimension_name_or_aigc_pattern_name",
      "description": "What is wrong",
      "suggestion": "How to fix it"
    }}
  ],
  "summary": "Overall assessment"
}}

passed is false ONLY when critical-severity issues exist.

overall_score calibration:
- 95-100: Publishable as-is, no noticeable issues
- 85-94: Minor blemishes but smooth reading
- 75-84: Noticeable problems but story backbone holds
- 65-74: Multiple issues hurt reading experience
- < 65: Structural breakdown, needs major rewrite
Score holistically — do not let a single minor issue tank the score."#,
                dim_list = dim_list
            )
        }
        _ => {
            format!(
                r#"你是一位严格的网络小说结构编辑。先审计章节的完整性和结构，再做一遍 AIGC 写作痕迹审查。

## 审计维度（连续性与结构）

{dim_list}

每条 issue 必须给 repair_scope 作为 typed 路由提示："local" 表示措辞、段落形状、小重复、句段级小修；"structural" 表示主线偏离、时间线断裂、场面/回报缺失、人物逻辑崩、视角/信息边界失败，或任何需要重写场景/整章的问题；只有确实无法判断时才写 "unknown"。

注意：维度 10/21/22/23 与下方 AIGC 审查有重叠。这里报故事层面的问题；AIGC 痕迹相关问题在第二遍审查里报，`category` 用 AIGC 模式名。

## AIGC 写作痕迹审查（第二遍）

结构审查后，按下方指导段中的 33 项 AIGC 模式清单扫描正文。每发现一处，输出一条 AuditIssue：
- `category` = AIGC 模式名（如"意义夸大"、"破折号"、"AI 高频词"）
- `severity` = 默认 "warning"；仅硬契约违例用 "critical"（如已定稿章节出现 em/en 破折号、同段堆叠 5+ 个 AI 高频词、叙事正文里残留聊天机器人谄媚语）
- `repair_scope` = "local"（AIGC 痕迹始终是局部修复）
- `description` = 触发该模式的具体短语或句子
- `suggestion` = 自然语言改写建议

务必应用误报防护：单个破折号、一个"然而"、单独弯引号都不构成 AIGC 证据，只报痕迹簇。

## 输出格式（JSON）

{{
  "passed": true/false,
  "overall_score": 0-100,
  "issues": [
    {{
      "severity": "critical|warning|info",
      "repair_scope": "local|structural|unknown",
      "category": "维度名或AIGC模式名",
      "description": "问题描述",
      "suggestion": "修复建议"
    }}
  ],
  "summary": "整体评估"
}}

只有存在 critical 级别问题时 passed 才为 false。

overall_score 评分校准：
- 95-100：可直接发布，无明显问题
- 85-94：有小瑕疵但整体流畅可读
- 75-84：有明显问题但故事主干完整
- 65-74：多处影响阅读体验的问题
- < 65：结构性问题，需要大幅重写
综合评分，不要因为单一小问题大幅拉低分数。"#,
                dim_list = dim_list
            )
        }
    };

    // 装配：任务段 + AIGC 审查指导段 + 输出纪律段
    let body = format!(
        "{}\n\n{}\n\n{}",
        task_prompt,
        aigc_audit_guidance(language),
        output_discipline(language),
    );
    assemble_with_identity(identity_prefix, &body)
}

pub fn build_user_message(
    chapter_number: u32,
    chapter_content: &str,
    book_dir: &std::path::Path,
    language: &str,
) -> String {
    let story_dir = book_dir.join("story");
    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let current_state = read_safe(&story_dir.join("current_state.md"));
    let pending_hooks = read_safe(&story_dir.join("pending_hooks.md"));
    let book_rules = read_safe(&story_dir.join("book_rules.md"));

    let heading = if language == "en" {
        format!("Chapter {}", chapter_number)
    } else {
        format!("第{}章", chapter_number)
    };

    format!(
        "## {} 正文\n\n{}\n\n## 当前状态\n\n{}\n\n## 伏笔池\n\n{}\n\n## 书级规则\n\n{}",
        heading, chapter_content, current_state, pending_hooks, book_rules
    )
}
