use async_trait::async_trait;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::utils::chapter_memo_parser::parse_chapter_memo;
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::governance::*;
use super::prompts::planner_prompts;
use super::agent_identity::AgentIdentity;

/// S5.2: memo 解析失败时的最大重试次数（与 inkos 对齐）。
const MEMO_RETRY_LIMIT: u32 = 3;

pub struct PlannerAgent;

impl Default for PlannerAgent {
    fn default() -> Self { Self }
}
impl PlannerAgent {
    pub fn new() -> Self { Self }

    /// Plan the next chapter's intent and memo.
    ///
    /// S5.2: 3 次重试 + fallback memo。失败时把 PlannerParseError 反馈给 LLM 让其修正。
    /// 全部失败后用 `build_fallback_memo_markdown` 产出降级但合格的 memo，
    /// 避免整条流水线崩溃（与 inkos 行为对齐）。
    pub async fn plan_chapter(
        &self,
        ctx: &AgentContext,
        book_dir: &std::path::Path,
        chapter_number: u32,
        external_context: Option<&str>,
        data_dir: &DataDir,
    ) -> Result<PlanOutput, AppError> {
        let language = read_book_language(book_dir).unwrap_or_else(|| "zh".to_string());
        let identity = AgentIdentity::load(data_dir, "planner");
        let task_query = format!("plan chapter {} of a novel", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = planner_prompts::build_system_prompt(&language, Some(&identity_prefix));
        let mut base_user = planner_prompts::build_user_message(
            book_dir,
            chapter_number,
            external_context,
            &language,
        )?;
        // S5.3: 黄金三章指引（chapter 1-3 条件追加到 user message）
        let golden_guidance = planner_prompts::build_golden_opening_guidance(chapter_number, &language);
        if !golden_guidance.is_empty() {
            base_user = format!("{}\n\n{}", base_user, golden_guidance);
        }
        let is_golden_opening = chapter_number <= 3;

        // S5.2: 重试循环 —— 把 PlannerParseError 反馈给 LLM 让其修正
        let mut current_user = base_user.clone();
        let mut last_error: Option<AppError> = None;
        let mut last_response: Option<String> = None;

        for attempt in 1..=MEMO_RETRY_LIMIT {
            let response = self.chat(ctx, &system, &current_user).await?;
            last_response = Some(response.content.clone());

            match parse_chapter_memo(&response.content, chapter_number, is_golden_opening) {
                Ok(memo) => {
                    // 解析成功：从同一份响应提取 intent 字段，goal 用 memo.goal 覆盖
                    let intent = extract_chapter_intent(&response.content, chapter_number, &memo);
                    save_intent_markdown(book_dir, chapter_number, &intent, &memo)?;
                    return Ok(PlanOutput { intent, memo });
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    tracing::warn!(
                        attempt,
                        error = %err_msg,
                        "[planner] memo parse failed (attempt {}/{})",
                        attempt, MEMO_RETRY_LIMIT
                    );
                    last_error = Some(e);
                    let (header, trailer) = retry_feedback_sections(&language);
                    current_user = format!(
                        "{}\n\n{}\n{}\n{}",
                        base_user, header, err_msg, trailer
                    );
                }
            }
        }

        // 全部失败 —— 用 fallback memo 降级产出
        let fallback_error = last_error.unwrap_or_else(|| AppError::internal(
            "PlannerParseError: memo planner exhausted retries without a specific error".to_string()
        ));
        tracing::warn!(
            error = %fallback_error,
            "[planner] memo planner fell back after {} attempts",
            MEMO_RETRY_LIMIT
        );

        let fallback_goal = derive_fallback_goal(chapter_number, external_context);
        let fallback_md = build_fallback_memo_markdown(
            chapter_number,
            &fallback_goal,
            &fallback_error.to_string(),
            &language,
        );
        // fallback markdown 必须能通过严格解析（否则是本模块的 bug）
        let memo = parse_chapter_memo(&fallback_md, chapter_number, is_golden_opening)
            .map_err(|e| AppError::internal(format!(
                "PlannerParseError: fallback memo also failed to parse: {}", e
            )))?;

        // fallback 时 intent 从最后一份 LLM 响应提取（可能部分有效），goal 用 memo.goal
        let mut intent = if let Some(resp) = &last_response {
            extract_chapter_intent(resp, chapter_number, &memo)
        } else {
            ChapterIntent {
                chapter: chapter_number,
                goal: memo.goal.clone(),
                outline_node: None,
                arc_context: None,
                must_keep: vec![],
                must_avoid: vec![],
                style_emphasis: vec![],
            }
        };
        intent.goal = memo.goal.clone();

        save_intent_markdown(book_dir, chapter_number, &intent, &memo)?;
        Ok(PlanOutput { intent, memo })
    }
}

#[async_trait]
impl BaseAgent for PlannerAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Planner
    }

    fn name(&self) -> &str {
        "planner"
    }
}

pub struct PlanOutput {
    pub intent: ChapterIntent,
    pub memo: ChapterMemo,
}

/// 从 LLM 响应中提取 ChapterIntent 字段（must_keep / must_avoid / style_emphasis）。
/// goal 用 memo.goal 覆盖（与 inkos 行为对齐：LLM 产出的具体 goal 优先于 outline 派生值）。
fn extract_chapter_intent(
    content: &str,
    chapter_number: u32,
    memo: &ChapterMemo,
) -> ChapterIntent {
    ChapterIntent {
        chapter: chapter_number,
        goal: memo.goal.clone(),
        outline_node: None,
        arc_context: None,
        must_keep: extract_list_items(content, &["must keep", "必须保持"]),
        must_avoid: extract_list_items(content, &["must avoid", "不要做", "Do not"]),
        style_emphasis: extract_list_items(content, &["style emphasis", "风格强调"]),
    }
}

fn save_intent_markdown(
    book_dir: &std::path::Path,
    chapter_number: u32,
    intent: &ChapterIntent,
    memo: &ChapterMemo,
) -> Result<(), AppError> {
    let runtime_dir = book_dir.join("story").join("runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    let intent_path = runtime_dir.join(format!("chapter_{:04}_intent.md", chapter_number));
    let intent_content = render_intent_markdown(intent, memo);
    std::fs::write(&intent_path, &intent_content)?;
    Ok(())
}

fn render_intent_markdown(intent: &ChapterIntent, memo: &ChapterMemo) -> String {
    let must_keep = if intent.must_keep.is_empty() {
        "- none".to_string()
    } else {
        intent.must_keep.iter().map(|i| format!("- {}", i)).collect::<Vec<_>>().join("\n")
    };
    let must_avoid = if intent.must_avoid.is_empty() {
        "- none".to_string()
    } else {
        intent.must_avoid.iter().map(|i| format!("- {}", i)).collect::<Vec<_>>().join("\n")
    };

    format!(
        "# Chapter Intent\n\n## Goal\n{}\n\n## Must Keep\n{}\n\n## Must Avoid\n{}\n\n## Memo\n\n{}",
        intent.goal, must_keep, must_avoid, memo.body
    )
}

fn extract_section(content: &str, headings: &[&str]) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim().to_lowercase();
        for heading in headings {
            if trimmed.contains(&heading.to_lowercase()) {
                let mut result = Vec::new();
                for next in lines.iter().skip(i + 1) {
                    let next = next.trim();
                    if next.starts_with('#') {
                        break;
                    }
                    if !next.is_empty() {
                        result.push(next.to_string());
                    }
                }
                if !result.is_empty() {
                    return Some(result.join("\n"));
                }
            }
        }
    }
    None
}

fn extract_list_items(content: &str, headings: &[&str]) -> Vec<String> {
    let section = extract_section(content, headings).unwrap_or_default();
    section.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(|s| s.to_string())
        })
        .collect()
}

fn read_book_language(book_dir: &std::path::Path) -> Option<String> {
    crate::infrastructure::state_store::gc::utils::read_book_language_from_dir(book_dir)
}

/// 返回 (错误反馈标题, 修正要求尾注)，按语言切换中英文。
fn retry_feedback_sections(language: &str) -> (&'static str, &'static str) {
    if language == "en" {
        ("## Error from previous output", "Fix and re-emit.")
    } else {
        ("## 上次输出的错误", "请修正后重新输出。")
    }
}

/// 当 LLM 全部失败时，从外部指令或章节号派生一个 fallback goal。
fn derive_fallback_goal(chapter_number: u32, external_context: Option<&str>) -> String {
    if let Some(ctx) = external_context {
        let first_line = ctx.lines().find(|l| !l.trim().is_empty()).unwrap_or(ctx);
        let trimmed = first_line.trim();
        if !trimmed.is_empty() && trimmed.chars().count() <= 50 {
            return trimmed.to_string();
        }
    }
    format!("按当前大纲推进第 {} 章", chapter_number)
}

/// S5.2: 当 3 次重试全部失败时，构造一份降级但合格的 memo。
///
/// 这份 memo 必须能通过 `parse_chapter_memo` 的严格校验（8 小节 + minContentChars），
/// 保证流水线不因 planner 失败而崩溃。内容是模板化的稳健指引，
/// 并在 `## Planner warning` 节显式标注 fallback 状态以便日志聚合。
fn build_fallback_memo_markdown(
    chapter_number: u32,
    fallback_goal: &str,
    error_message: &str,
    language: &str,
) -> String {
    if language == "en" {
        return vec![
            format!("# Chapter {} memo", chapter_number),
            String::new(),
            "## Chapter goal".to_string(),
            if fallback_goal.is_empty() {
                format!("Continue chapter {} according to the current outline", chapter_number)
            } else {
                fallback_goal.to_string()
            },
            String::new(),
            "## Thread refs".to_string(),
            "none".to_string(),
            String::new(),
            "## Current task".to_string(),
            format!("Use the current chapter goal and authoritative book context to continue chapter {} without inventing a new direction.", chapter_number),
            String::new(),
            "## What the reader is waiting for right now".to_string(),
            "Keep the reader's active expectation from the outline and previous chapter in focus; do not replace it with a generic scene.".to_string(),
            String::new(),
            "## To pay off / to keep buried".to_string(),
            "Pay off only the near-term promises already supported by context; keep larger secrets buried unless the outline explicitly asks for them.".to_string(),
            String::new(),
            "## What the slow / transitional beats carry".to_string(),
            "If a slower beat is needed, make it carry pressure, evidence, relationship movement, or a concrete setup for the next action.".to_string(),
            String::new(),
            "## Three-question check on the key choice".to_string(),
            "The protagonist's main choice must have a reason, match current interest, and stay consistent with the established persona.".to_string(),
            String::new(),
            "## Required end-of-chapter change".to_string(),
            "End with a concrete change in information, pressure, relationship, objective, or risk so the chapter is not only summary.".to_string(),
            String::new(),
            "## Hook ledger for this chapter".to_string(),
            "advance: keep the active promise moving; resolve: only settle what has evidence; defer: preserve larger threads for later chapters.".to_string(),
            String::new(),
            "## Do not".to_string(),
            "Do not contradict established facts, ignore the user's current instruction, or turn the fallback memo into a new outline.".to_string(),
            String::new(),
            "## Planner warning".to_string(),
            format!("The model failed to produce a valid chapter memo after {} attempts. Last parser error: {}", MEMO_RETRY_LIMIT, error_message),
        ].join("\n");
    }

    vec![
        format!("# 第 {} 章 memo", chapter_number),
        String::new(),
        "## 本章目标".to_string(),
        if fallback_goal.is_empty() {
            format!("按当前大纲继续推进第 {} 章", chapter_number)
        } else {
            fallback_goal.to_string()
        },
        String::new(),
        "## 关联线索".to_string(),
        "无".to_string(),
        String::new(),
        "## 当前任务".to_string(),
        format!("沿用当前章节目标和权威设定推进第 {} 章，不临时改方向，也不把章节写成泛泛过渡。", chapter_number),
        String::new(),
        "## 读者此刻在等什么".to_string(),
        "延续大纲和上一章形成的读者期待，优先回应当前已经建立的压力、证据、关系或目标变化。".to_string(),
        String::new(),
        "## 该兑现的 / 暂不掀的".to_string(),
        "只兑现已有上下文支撑的近端承诺；更大的秘密、身份、幕后主使或终局信息，除非大纲明确要求，否则继续压住。".to_string(),
        String::new(),
        "## 日常/过渡承担什么任务".to_string(),
        "如果需要日常或过渡，它必须承担压力、证据、人物关系、目标变化或下一步行动铺垫，不能只是闲聊和气氛。".to_string(),
        String::new(),
        "## 关键抉择过三连问".to_string(),
        "主角本章的关键选择必须有原因、符合当前利益，并且不背离已经建立的人设和行为逻辑。".to_string(),
        String::new(),
        "## 章尾必须发生的改变".to_string(),
        "章尾至少要在信息、压力、关系、目标或风险上发生一个明确变化，避免只有剧情摘要没有推进。".to_string(),
        String::new(),
        "## 本章 hook 账".to_string(),
        "advance: 推进当前活跃承诺；resolve: 只结清已有证据支撑的线索；defer: 大线继续保留到更合适的位置。".to_string(),
        String::new(),
        "## 不要做".to_string(),
        "不要违背既成事实，不要无视用户当前指令，不要把 fallback memo 当成新大纲重写整本书。".to_string(),
        String::new(),
        "## Planner warning".to_string(),
        format!("模型连续 {} 次没有产出合格章节 memo。最后一次解析错误：{}", MEMO_RETRY_LIMIT, error_message),
    ].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_memo_zh_passes_strict_parse() {
        let md = build_fallback_memo_markdown(5, "推进主线", "missing sections: ## 当前任务", "zh");
        let memo = parse_chapter_memo(&md, 5, false)
            .expect("fallback memo (zh) must pass strict parse");
        assert_eq!(memo.chapter, 5);
        assert_eq!(memo.goal, "推进主线");
        assert!(memo.thread_refs.is_empty());
        assert!(memo.body.contains("## Planner warning"));
        assert!(memo.body.contains("missing sections: ## 当前任务"));
    }

    #[test]
    fn test_fallback_memo_en_passes_strict_parse() {
        let md = build_fallback_memo_markdown(7, "Reveal the secret", "empty sections", "en");
        let memo = parse_chapter_memo(&md, 7, false)
            .expect("fallback memo (en) must pass strict parse");
        assert_eq!(memo.chapter, 7);
        assert_eq!(memo.goal, "Reveal the secret");
        assert!(memo.thread_refs.is_empty());
        assert!(memo.body.contains("## Planner warning"));
    }

    #[test]
    fn test_fallback_memo_empty_goal_uses_default() {
        let md = build_fallback_memo_markdown(3, "", "all retries failed", "zh");
        let memo = parse_chapter_memo(&md, 3, true).unwrap();
        assert_eq!(memo.goal, "按当前大纲继续推进第 3 章");
        assert!(memo.is_golden_opening);
    }

    #[test]
    fn test_derive_fallback_goal_from_external_context() {
        let ctx = "主角发现废弃教堂的秘密\n这是第二行";
        let goal = derive_fallback_goal(5, Some(ctx));
        assert_eq!(goal, "主角发现废弃教堂的秘密");
    }

    #[test]
    fn test_derive_fallback_goal_long_context_falls_back_to_chapter() {
        // > 50 字符的外部指令应回退到章节号派生的默认目标
        let long = "这是一段非常非常非常非常长的外部指令内容已经远远超过五十个字符的限制应该回退到章节号派生的默认目标才行";
        let goal = derive_fallback_goal(5, Some(long));
        assert_eq!(goal, "按当前大纲推进第 5 章");
    }

    #[test]
    fn test_derive_fallback_goal_no_context() {
        let goal = derive_fallback_goal(10, None);
        assert_eq!(goal, "按当前大纲推进第 10 章");
    }

    #[test]
    fn test_retry_feedback_sections_zh() {
        let (h, t) = retry_feedback_sections("zh");
        assert_eq!(h, "## 上次输出的错误");
        assert_eq!(t, "请修正后重新输出。");
    }

    #[test]
    fn test_retry_feedback_sections_en() {
        let (h, t) = retry_feedback_sections("en");
        assert_eq!(h, "## Error from previous output");
        assert_eq!(t, "Fix and re-emit.");
    }

    #[test]
    fn test_extract_chapter_intent_uses_memo_goal() {
        let memo = ChapterMemo {
            chapter: 5,
            goal: "LLM 产出的具体目标".to_string(),
            is_golden_opening: false,
            body: "## 当前任务\n...".to_string(),
            thread_refs: vec![],
        };
        let content = "## must keep\n- 人设一致\n- 时间线\n## must avoid\n- 不要水字数";
        let intent = extract_chapter_intent(content, 5, &memo);
        assert_eq!(intent.goal, "LLM 产出的具体目标");
        assert_eq!(intent.must_keep, vec!["人设一致".to_string(), "时间线".to_string()]);
        assert_eq!(intent.must_avoid, vec!["不要水字数".to_string()]);
    }
}
