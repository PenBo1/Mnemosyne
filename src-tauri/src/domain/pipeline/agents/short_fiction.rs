// Short Fiction Agents。
//
// 职责：短篇 pipeline 的 6 个 agent（create_outline / review_outline / revise_outline /
// write_draft / review_draft / revise_draft / generate_package）+ 输出解析。
//
// 约束：AgentEngine.prompt_once 只支持单轮对话。多轮对话（revise 系列）
// 合并为单轮：在 user message 中嵌入原 v1 输出 + review + 修订指令。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::types::Language;

// ── 常量 ─────────────────────────────────────────────────────

pub const SHORT_FICTION_DEFAULT_CHAPTERS: u32 = 12;
pub const SHORT_FICTION_MIN_CHAPTERS: u32 = 12;
pub const SHORT_FICTION_MAX_CHAPTERS: u32 = 18;
pub const SHORT_FICTION_DEFAULT_CHARS_PER_CHAPTER: u32 = 1000;
pub const SHORT_FICTION_EN_DEFAULT_WORDS_PER_CHAPTER: u32 = 650;

/// 漏章补写最大重试次数。
pub const SHORT_FICTION_DRAFT_COMPLETION_ATTEMPTS: u32 = 3;

// ── 数据结构 ─────────────────────────────────────────────────

/// 短篇大纲
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionOutline {
    pub story_title: String,
    pub raw_content: String,
}

/// 短篇章节
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionChapter {
    pub number: u32,
    pub title: String,
    pub content: String,
    pub char_count: u32,
}

/// 短篇批量草稿
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionBatchDraft {
    pub story_title: String,
    pub opening_hook: Option<String>,
    pub chapters: Vec<ShortFictionChapter>,
    pub raw_content: String,
}

/// 短篇销售包装
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShortFictionSalesPackage {
    pub title: String,
    pub intro: String,
    pub selling_points: Vec<String>,
    pub cover_prompt: String,
    pub raw_content: String,
}

/// 参考文本
#[derive(Debug, Clone)]
pub struct ShortFictionReference {
    pub text: String,
}

// ── 输入结构 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineInput {
    pub direction: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub reference: Option<ShortFictionReference>,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineReviewInput {
    pub direction: String,
    pub outline: ShortFictionOutline,
    pub reference: Option<ShortFictionReference>,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionOutlineRevisionInput {
    pub direction: String,
    pub outline: ShortFictionOutline,
    pub review: String,
    pub reference: Option<ShortFictionReference>,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftContinuationInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftReviewInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionDraftRevisionInput {
    pub direction: String,
    pub outline_markdown: String,
    pub chapter_count: u32,
    pub chars_per_chapter: u32,
    pub draft: ShortFictionBatchDraft,
    pub review: String,
    pub language: Language,
}

#[derive(Debug, Clone)]
pub struct ShortFictionPackageInput {
    pub direction: String,
    pub outline_markdown: String,
    pub draft: ShortFictionBatchDraft,
    pub language: Language,
}

// ═══════════════════════════════════════════════════════════════
//  Agent 执行函数
// ═══════════════════════════════════════════════════════════════

/// 1. 生成短篇大纲（temp=0.55, maxTokens=8192）。
pub async fn create_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineInput,
) -> Result<ShortFictionOutline, AppError> {
    let system_prompt = build_outline_system_prompt(input.language);
    let user_message = build_outline_user_prompt(input);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_outline(&response, input.language))
}

/// 2. 审核短篇大纲（temp=0.3, maxTokens=4096）。
pub async fn review_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineReviewInput,
) -> Result<String, AppError> {
    let system_prompt = build_outline_review_system_prompt(input.language);
    let user_message = build_outline_review_user_prompt(input);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

/// 3. 修订短篇大纲（temp=0.45, maxTokens=8192）。
/// 多轮对话合并为单轮：user message 包含原 prompt + v1 outline + 修订指令。
pub async fn revise_outline(
    engine: &AgentEngine,
    input: &ShortFictionOutlineRevisionInput,
) -> Result<ShortFictionOutline, AppError> {
    let system_prompt = build_outline_system_prompt(input.language);
    let original_user = build_outline_user_prompt(&ShortFictionOutlineInput {
        direction: input.direction.clone(),
        chapter_count: input.chapter_count,
        chars_per_chapter: input.chars_per_chapter,
        reference: input.reference.clone(),
        language: input.language,
    });
    let followup = build_outline_revision_followup(input);
    let user_message = format!(
        "{original}\n\n---\n\n{v1_header}\n\n{v1}\n\n---\n\n{followup}",
        original = original_user,
        v1_header = v1_outline_header(input.language),
        v1 = input.outline.raw_content.trim(),
        followup = followup,
    );
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_outline(&response, input.language))
}

/// 4. 一次性写出整篇短篇草稿（temp=0.58）。
pub async fn write_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let system_prompt = build_writer_system_prompt(input.language);
    let user_message = build_writer_user_prompt(input);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_batch_draft(&response, input.chapter_count, input.language))
}

/// 4b. 补写缺失章节（temp=0.68）。
pub async fn continue_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftContinuationInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let missing = find_empty_chapters(&input.draft);
    if missing.is_empty() {
        return Ok(input.draft.clone());
    }
    let system_prompt = build_writer_system_prompt(input.language);
    let user_message = build_draft_continuation_user_prompt(input, &missing);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let combined = format!(
        "{}\n\n{}",
        input.draft.raw_content.trim(),
        response.trim()
    );
    Ok(parse_batch_draft(&combined, input.chapter_count, input.language))
}

/// 5. 审核整篇草稿（temp=0.3, maxTokens=8192）。
pub async fn review_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftReviewInput,
) -> Result<String, AppError> {
    let system_prompt = build_draft_review_system_prompt(input.language);
    let user_message = build_draft_review_user_prompt(input);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

/// 6. 修订整篇草稿（temp=0.45）。
/// 多轮对话合并为单轮。
pub async fn revise_draft(
    engine: &AgentEngine,
    input: &ShortFictionDraftRevisionInput,
) -> Result<ShortFictionBatchDraft, AppError> {
    let system_prompt = build_writer_system_prompt(input.language);
    let original_user = build_writer_user_prompt(&ShortFictionDraftInput {
        direction: input.direction.clone(),
        outline_markdown: input.outline_markdown.clone(),
        chapter_count: input.chapter_count,
        chars_per_chapter: input.chars_per_chapter,
        language: input.language,
    });
    let followup = build_draft_revision_followup(input);
    let v1_raw = if !input.draft.raw_content.trim().is_empty() {
        input.draft.raw_content.trim().to_string()
    } else {
        render_draft_markdown(&input.draft, input.language)
    };
    let user_message = format!(
        "{original}\n\n---\n\n{v1_header}\n\n{v1}\n\n---\n\n{followup}",
        original = original_user,
        v1_header = v1_draft_header(input.language),
        v1 = v1_raw,
        followup = followup,
    );
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_batch_draft(&response, input.chapter_count, input.language))
}

/// 7. 生成销售包装（temp=0.45, maxTokens=4096）。
pub async fn generate_package(
    engine: &AgentEngine,
    input: &ShortFictionPackageInput,
) -> Result<ShortFictionSalesPackage, AppError> {
    let system_prompt = build_package_system_prompt(input.language);
    let user_message = build_package_user_prompt(input);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(parse_sales_package(&response, &input.draft.story_title))
}

// ═══════════════════════════════════════════════════════════════
//  Prompt 构造函数
// ═══════════════════════════════════════════════════════════════

fn build_outline_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are the managing editor for short web fiction. Your job is to turn one creative direction into a complete short-story plan.",
            "Work only from this direction and any reference text the user supplied; never claim to have read, quoted, or inherited material that was not provided.",
            "Content comes first: the title, the opening, the pressure on the protagonist, the evidence/relationship/identity leverage, the escalation chain, the reversal chain, and the payoff landing must be strong enough to carry a single-pass full draft.",
            "Do not over-structure and do not output JSON/YAML. Write human-readable Markdown, but the chapter plan must be dense enough that a writer can draft the whole story in one pass.",
            "A short defaults to 12-18 chapters at roughly 600-800 words per chapter. The story must be complete — not the first five chapters of a novel starter kit.",
        ].join("\n")
    } else {
        vec![
            "You are the managing editor for short web fiction. Your job is to turn one creative direction into a complete short-story plan. Write the plan and the resulting prose in Simplified Chinese.",
            "Work only from this direction and any reference text the user supplied; never claim to have read, quoted, or inherited material that was not provided.",
            "Content comes first: the title, the opening, the pressure on the protagonist, the evidence/relationship/identity leverage, the escalation chain, the reversal chain, and the payoff landing must be strong enough to carry a single-pass full draft.",
            "Do not over-structure and do not output JSON/YAML. Write human-readable Markdown, but the chapter plan must be dense enough that a writer can draft the whole story in one pass.",
            "A short defaults to 12-18 chapters at roughly 900-1200 Chinese characters per chapter. The story must be complete — not the first five chapters of a novel starter kit.",
        ].join("\n")
    }
}

fn build_outline_user_prompt(input: &ShortFictionOutlineInput) -> String {
    let reference = reference_block(input.reference.as_ref(), input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Target Spec".to_string(),
            format!(
                "A complete short story of {} chapters, about {} words per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## Deliverable".to_string(),
            "Start with one platform-ready clickable title, then the full story plan. The plan must make clear why the protagonist is pinned down, what payoff the reader is waiting for, how the protagonist turns the tables, how evidence/relationships/identity/rules escalate step by step, why the antagonist strikes back, and how the ending lands.".to_string(),
            "The chapter plan must spell out, chapter by chapter: the direction of the chapter title, the key on-page scene, the characters' actions, the escalation or payoff, and the reason to keep reading at the chapter break.".to_string(),
            "Tags are allowed, but do not enumerate a tag table; tags serve premise selection and writing — they never replace the story.".to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "Exactly one platform-ready title on a single line".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "The full story plan in Markdown, covering: genre/audience, title direction, the opening hook, characters and relationships, the core pressure, how the protagonist wins, the escalation chain, the reversal chain, the ending payoff, and the chapter-by-chapter plan.".to_string(),
        ]);
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Target Spec".to_string(),
            format!(
                "A complete short story of {} chapters, about {} Chinese characters per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## Deliverable".to_string(),
            "Start with one platform-ready clickable title, then the full story plan. The plan must make clear why the protagonist is pinned down, what payoff the reader is waiting for, how the protagonist turns the tables, how evidence/relationships/identity/rules escalate step by step, why the antagonist strikes back, and how the ending lands.".to_string(),
            "The chapter plan must spell out, chapter by chapter: the direction of the chapter title, the key on-page scene, the characters' actions, the escalation or payoff, and the reason to keep reading at the chapter break.".to_string(),
            "Tags are allowed, but do not enumerate a tag table; tags serve premise selection and writing — they never replace the story.".to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "Exactly one platform-ready title on a single line (in Simplified Chinese)".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "The full story plan in Markdown (in Simplified Chinese), covering: genre/audience, title direction, the opening hook, characters and relationships, the core pressure, how the protagonist wins, the escalation chain, the reversal chain, the ending payoff, and the chapter-by-chapter plan.".to_string(),
        ]);
        parts
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_outline_review_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are a short-fiction outline reviewer. You do not assign scores and you do not police plagiarism.",
            "Your job is to judge whether this story plan can carry a single-pass full draft: is the genre engine clear, do character motivations hold, does the pressure chain escalate, is the antagonist's counterattack believable, is the ending payoff big enough.",
            "Review like a real reader and a real editor, not a checklist machine.",
            "Output Markdown. Name the flaws that would make the finished draft fall flat, and the strengths worth keeping.",
        ].join("\n")
    } else {
        vec![
            "You are a short-fiction outline reviewer. You do not assign scores and you do not police plagiarism. Write your review in Simplified Chinese.",
            "Your job is to judge whether this story plan can carry a single-pass full draft: is the genre engine clear, do character motivations hold, does the pressure chain escalate, is the antagonist's counterattack believable, is the ending payoff big enough.",
            "Review like a real reader and a real editor, not a checklist machine.",
            "Output Markdown in Simplified Chinese. Name the flaws that would make the finished draft fall flat, and the strengths worth keeping.",
        ].join("\n")
    }
}

fn build_outline_review_user_prompt(input: &ShortFictionOutlineReviewInput) -> String {
    let reference = reference_block(input.reference.as_ref(), input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## Story Plan Under Review".to_string(),
            input.outline.raw_content.clone(),
            String::new(),
            "## Review Focus".to_string(),
            "- Is this a complete short story, rather than a partial tryout plan?".to_string(),
            "- Do the title, the opening, and the first three chapters give readers a reason to click and keep reading?".to_string(),
            "- Is the outline dense enough, or will the writer run out of material in the back half?".to_string(),
            "- Do the key scenes contain character action, counterattack, and payoff, instead of bare result summaries?".to_string(),
            "- Will readers be thrown out of the story by timeline, relationship, evidence-access, physical-state, or common-sense problems?".to_string(),
        ]);
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## Story Plan Under Review".to_string(),
            input.outline.raw_content.clone(),
            String::new(),
            "## Review Focus".to_string(),
            "- Is this a complete short story, rather than a partial tryout plan?".to_string(),
            "- Do the title, the opening, and the first three chapters give readers a reason to click and keep reading?".to_string(),
            "- Is the outline dense enough, or will the writer run out of material in the back half?".to_string(),
            "- Do the key scenes contain character action, counterattack, and payoff, instead of bare result summaries?".to_string(),
            "- Will readers be thrown out of the story by timeline, relationship, evidence-access, physical-state, or common-sense problems?".to_string(),
        ]);
        parts
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_outline_revision_followup(input: &ShortFictionOutlineRevisionInput) -> String {
    if input.language == Language::En {
        vec![
            "Based on the outline review above, produce the complete second version of the story plan.".to_string(),
            "This is round two of the same project: do not start over from scratch, and do not output a list of edits instead of the plan.".to_string(),
            format!(
                "Keep the structure at {} chapters of about {} words each.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Keep the genre engine and relationships that work; fix the flaws that would make the finished draft fall flat.".to_string(),
            String::new(),
            "## Outline Review".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "Exactly one platform-ready title on a single line".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "The complete second-version story plan in Markdown.".to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    } else {
        vec![
            "Based on the outline review above, produce the complete second version of the story plan. Write in Simplified Chinese.".to_string(),
            "This is round two of the same project: do not start over from scratch, and do not output a list of edits instead of the plan.".to_string(),
            format!(
                "Keep the structure at {} chapters of about {} Chinese characters each.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Keep the genre engine and relationships that work; fix the flaws that would make the finished draft fall flat.".to_string(),
            String::new(),
            "## Outline Review".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "Exactly one platform-ready title on a single line (in Simplified Chinese)".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "The complete second-version story plan in Markdown (in Simplified Chinese).".to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    }
}

fn build_writer_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are an English short-fiction BatchWriter. You write the complete short story in one API pass, following the story plan.",
            "Write natural, native English prose. Vary sentence length; mix short punchy sentences with longer flowing ones, and keep the narrative voice consistent throughout.",
            "This is not serialized-novel continuation and not chapter synopsis. Every chapter needs drama happening on the page: character action, dialogue or reaction, a shift in the situation, and a reason to keep reading at the chapter break.",
            "Keep the drama dialed up, web-fiction style: real-world pressure may be amplified as far as readers will still believe, but never so absurd that immersion breaks.",
            "The story title and chapter titles must read like platform content, not literary summaries. Keep the prose paced for mobile reading — short paragraphs, but never telegram-style fragments.",
            "The word count is a calibration, not an averaging exercise. Big scenes may run long and transitions short; a clearly short chapter usually means you wrote a synopsis and must add real scenes.",
            "Output must strictly use the specified blocks. No author notes, no word-count remarks, no review comments, no format explanations.",
        ].join("\n")
    } else {
        vec![
            "You are a Simplified-Chinese short-fiction BatchWriter. Following the story plan, write the complete short-story prose in one API pass.",
            "This is not serialized-novel continuation and not chapter synopsis. Every chapter needs drama happening on the page: character action, dialogue or reaction, a shift in the situation, and a reason to keep reading at the chapter break.",
            "Keep the drama dialed up, web-fiction style: real-world pressure may be amplified as far as readers will still believe, but never so absurd that immersion breaks.",
            "The story title and chapter titles must read like platform content, not literary summaries. Keep the prose paced for mobile reading — short paragraphs, but never telegram-style fragments.",
            "The character count is a calibration, not an averaging exercise. Big scenes may run long and transitions short; a clearly short chapter usually means you wrote a synopsis and must add real scenes.",
            "Output must strictly use the specified blocks. No author notes, no word-count remarks, no review comments, no format explanations.",
        ].join("\n")
    }
}

fn build_writer_user_prompt(input: &ShortFictionDraftInput) -> String {
    let craft = build_craft_prompt(input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## Task".to_string(),
            format!(
                "Write the complete {}-chapter story in one pass, about {} words per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Read the full story plan before writing. The prose must carry the plan's pressure chain, evidence chain, reversal chain, and emotional payoff — do not swerve into a different story midway.".to_string(),
            String::new(),
            craft,
            String::new(),
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Story Plan".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "The story title — plain text, platform-ready, nothing else".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "An optional pre-story hook of about 130 words; if no standalone teaser is needed, still write the small first-screen scene that opens chapter 1".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## Task".to_string(),
            format!(
                "Write the complete {}-chapter story in one pass, about {} Chinese characters per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Read the full story plan before writing. The prose must carry the plan's pressure chain, evidence chain, reversal chain, and emotional payoff — do not swerve into a different story midway.".to_string(),
            String::new(),
            craft,
            String::new(),
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Story Plan".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "The story title — plain text, platform-ready, in Simplified Chinese, nothing else".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "An optional pre-story hook of about 200 Chinese characters; if no standalone teaser is needed, still write the small first-screen scene that opens chapter 1".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix (in Simplified Chinese)".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes (in Simplified Chinese)", chapter));
        }
        parts
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_draft_continuation_user_prompt(
    input: &ShortFictionDraftContinuationInput,
    missing: &[u32],
) -> String {
    let missing_str = missing
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let craft = build_craft_prompt(input.language);
    let existing_md = render_draft_markdown(&input.draft, input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## Task".to_string(),
            format!(
                "The previous draft was truncated or skipped chapters. Write ONLY the missing chapters: {}.",
                missing_str
            ),
            format!(
                "Stay calibrated to the complete {}-chapter short at about {} words per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Do not rewrite finished chapters, do not write summary notes, do not apologize, do not output review comments.".to_string(),
            String::new(),
            craft,
            String::new(),
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Story Plan".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## Existing Draft (for continuity only — do not rewrite)".to_string(),
            existing_md,
            String::new(),
            "## Output Format".to_string(),
        ];
        for &chapter in missing {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## Task".to_string(),
            format!(
                "The previous draft was truncated or skipped chapters. Write ONLY the missing chapters: {}.",
                missing_str
            ),
            format!(
                "Stay calibrated to the complete {}-chapter short at about {} Chinese characters per chapter.",
                input.chapter_count, input.chars_per_chapter
            ),
            "Do not rewrite finished chapters, do not write summary notes, do not apologize, do not output review comments.".to_string(),
            String::new(),
            craft,
            String::new(),
            "## Creative Direction".to_string(),
            input.direction.clone(),
            String::new(),
            "## Story Plan".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## Existing Draft (for continuity only — do not rewrite)".to_string(),
            existing_md,
            String::new(),
            "## Output Format".to_string(),
        ];
        for &chapter in missing {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix (in Simplified Chinese)".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes (in Simplified Chinese)", chapter));
        }
        parts
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_draft_review_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are a short-fiction draft reviewer.",
            "You judge only whether the content can sell, reads smoothly, and keeps pulling the reader forward; do not turn the review into deterministic scoring.",
            "Focus on: the title, chapter titles, the opening, character motivation, the timeline, relationships, evidence and access, escalating pressure, the antagonist's counterattack, whether the back half sags, and whether the ending payoff lands.",
            "Output Markdown. Separate the problems that would visibly stop readers from reading on from the small blemishes that are acceptable.",
        ].join("\n")
    } else {
        vec![
            "You are a short-fiction draft reviewer. Write your review in Simplified Chinese.",
            "You judge only whether the content can sell, reads smoothly, and keeps pulling the reader forward; do not turn the review into deterministic scoring.",
            "Focus on: the title, chapter titles, the opening, character motivation, the timeline, relationships, evidence and access, escalating pressure, the antagonist's counterattack, whether the back half sags, and whether the ending payoff lands.",
            "Output Markdown in Simplified Chinese. Separate the problems that would visibly stop readers from reading on from the small blemishes that are acceptable.",
        ].join("\n")
    }
}

fn build_draft_review_user_prompt(input: &ShortFictionDraftReviewInput) -> String {
    let draft_md = render_draft_markdown(&input.draft, input.language);
    if input.language == Language::En {
        vec![
            "## Creative Direction",
            &input.direction,
            "",
            "## Original Story Plan",
            &input.outline_markdown,
            "",
            "## Draft Under Review",
            &draft_md,
            "",
            "## Review Instructions",
            "Talk like a person: where does this story pull, where does it break immersion, where does it read like a synopsis, where does the back half sag, which title or chapter titles would nobody tap?",
            "Never condemn a chapter just for running slightly short or long; judge first whether the content is complete, dramatic, and paying off.",
        ]
        .join("\n")
    } else {
        vec![
            "## Creative Direction",
            &input.direction,
            "",
            "## Original Story Plan",
            &input.outline_markdown,
            "",
            "## Draft Under Review",
            &draft_md,
            "",
            "## Review Instructions",
            "Talk like a person: where does this story pull, where does it break immersion, where does it read like a synopsis, where does the back half sag, which title or chapter titles would nobody tap? Write your review in Simplified Chinese.",
            "Never condemn a chapter just for running slightly short or long; judge first whether the content is complete, dramatic, and paying off.",
        ]
        .join("\n")
    }
}

fn build_draft_revision_followup(input: &ShortFictionDraftRevisionInput) -> String {
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "Based on the review notes, write the complete second-version draft.".to_string(),
            "This is round two of the same story: keep what worked in the last version, fix what breaks immersion or kills the desire to keep reading.".to_string(),
            "Do not output a list of suggested edits, and do not patch just a few chapters — output the complete draft.".to_string(),
            String::new(),
            "## Review Notes".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## Round-Two Priorities".to_string(),
            "- Fix the immersion-breaking problems: timeline, logic, relationships, evidence access, physical state.".to_string(),
            "- Add real scenes to the back half; never close on result summaries.".to_string(),
            "- Keep the title, opening, chapter titles, and main title consistent with the prose, though the title may be re-sharpened from the final draft for platform click appeal.".to_string(),
            "- Word count is calibration only: pad short chapters with real scenes; trim long ones by cutting explanation and repeated reactions.".to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "The story title — plain text, platform-ready, nothing else".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "An optional pre-story hook of about 130 words; if no standalone teaser is needed, still write the small first-screen scene that opens chapter 1".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "Based on the review notes, write the complete second-version draft. Write in Simplified Chinese.".to_string(),
            "This is round two of the same story: keep what worked in the last version, fix what breaks immersion or kills the desire to keep reading.".to_string(),
            "Do not output a list of suggested edits, and do not patch just a few chapters — output the complete draft.".to_string(),
            String::new(),
            "## Review Notes".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## Round-Two Priorities".to_string(),
            "- Fix the immersion-breaking problems: timeline, logic, relationships, evidence access, physical state.".to_string(),
            "- Add real scenes to the back half; never close on result summaries.".to_string(),
            "- Keep the title, opening, chapter titles, and main title consistent with the prose, though the title may be re-sharpened from the final draft for platform click appeal.".to_string(),
            "- Character count is calibration only: pad short chapters with real scenes; trim long ones by cutting explanation and repeated reactions.".to_string(),
            String::new(),
            "## Output Format".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "The story title — plain text, platform-ready, in Simplified Chinese, nothing else".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "An optional pre-story hook of about 200 Chinese characters; if no standalone teaser is needed, still write the small first-screen scene that opens chapter 1".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("Chapter title — plain text only, no #, no \"Chapter N\" prefix (in Simplified Chinese)".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("Chapter {} prose — full scenes, no synopsis, no author notes (in Simplified Chinese)", chapter));
        }
        parts
    }
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_package_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are a short-fiction packaging editor. From the final draft you produce the synopsis, the selling points, and the cover-image prompt.",
            "Never invent a main title different from the draft's. All packaging must revolve around the draft's actual title and plot.",
            "Think of the cover prompt as a mobile portrait book cover: 3:4 vertical, a large title zone, strong character emotion, one or two instantly recognizable props, high-contrast colors — not a movie poster.",
        ].join("\n")
    } else {
        vec![
            "You are a short-fiction packaging editor. From the final draft you produce the synopsis, the selling points, and the cover-image prompt. Write the synopsis and selling points in Simplified Chinese.",
            "Never invent a main title different from the draft's. All packaging must revolve around the draft's actual title and plot.",
            "Think of the cover prompt as a mobile portrait book cover: 3:4 vertical, a large title zone, strong character emotion, one or two instantly recognizable props, high-contrast colors — not a movie poster.",
        ].join("\n")
    }
}

fn build_package_user_prompt(input: &ShortFictionPackageInput) -> String {
    let draft_md = render_draft_markdown(&input.draft, input.language);
    if input.language == Language::En {
        vec![
            "## Creative Direction",
            &input.direction,
            "",
            "## Story Plan",
            &input.outline_markdown.trim(),
            "",
            "## Final Draft",
            &draft_md.trim(),
            "",
            "## Output Format",
            "=== SHORT_FICTION_PACKAGE_TITLE ===",
            &input.draft.story_title,
            "=== SHORT_FICTION_INTRO ===",
            "A 70-120 word platform synopsis that grabs the conflict, the pressure, and the payoff — never a spoiler-filled play-by-play.",
            "=== SHORT_FICTION_SELLING_POINTS ===",
            "- 3 to 6 selling points, one per line",
            "=== SHORT_FICTION_COVER_PROMPT ===",
            "An English cover-generation prompt: 3:4 portrait, main title zone, character emotion, props, color palette, typography style, and what to avoid.",
        ]
        .join("\n")
    } else {
        vec![
            "## Creative Direction",
            &input.direction,
            "",
            "## Story Plan",
            &input.outline_markdown.trim(),
            "",
            "## Final Draft",
            &draft_md.trim(),
            "",
            "## Output Format",
            "=== SHORT_FICTION_PACKAGE_TITLE ===",
            &input.draft.story_title,
            "=== SHORT_FICTION_INTRO ===",
            "A 100-180 Chinese-character platform synopsis that grabs the conflict, the pressure, and the payoff — never a spoiler-filled play-by-play. Write in Simplified Chinese.",
            "=== SHORT_FICTION_SELLING_POINTS ===",
            "- 3 to 6 selling points, one per line (in Simplified Chinese)",
            "=== SHORT_FICTION_COVER_PROMPT ===",
            "A Simplified-Chinese cover-generation prompt: 3:4 portrait, main title zone, character emotion, props, color palette, typography style, and what to avoid.",
        ]
        .join("\n")
    }
}

fn build_craft_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "## Craft Reminders",
            "- Salt dissolves in the soup: values and ambition show through action, never through slogans.",
            "- Show, don't tell: let behavior, evidence, concrete detail, and staging make the reader feel a character's state.",
            "- Simile restraint: do not lean on \"like / as if / as though\" as default rhetoric — at most one simile per scene; prefer a precise verb and a concrete action over a figure of speech.",
            "- Anti-AI wording: ration AI-tell words (delve, tapestry, testament, intricate, pivotal); do not use the \"It wasn't X; it was Y\" construction as a crutch; keep analytical report language (\"core motivation\", \"strategic advantage\") out of the prose.",
            "- No padding: every scene must advance conflict, causality, emotion, evidence, pressure, payoff, or a relationship.",
            "- The climax is a scene, not a recap: eruptions of conflict, reversals, life-or-death beats, and reveals must play out beat by beat on the page (action, dialogue, the five senses). The heavier a chapter's information load, the more its key beat must be staged as a full scene — never compressed into one line like \"then he saved her and the rival was arrested.\"",
            "- Payoffs need setup: every reversal, comeuppance, reconciliation, revenge, or identity reveal must ride a chain of evidence and causality.",
            "- Side characters need motives: even the oppressor acts from interest, misjudgment, or fear — never a brainless plot device.",
            "- Everyday detail must become bait: each detail carries evidence, emotion, characterization, or a later reversal.",
            "- Mobile-first: short paragraphs, dense information, no vague lyricism or decorative filler.",
        ].join("\n")
    } else {
        vec![
            "## Craft Reminders",
            "- Salt dissolves in the soup: values and ambition show through action, never through slogans.",
            "- Show, don't tell: let behavior, evidence, concrete detail, and staging make the reader feel a character's state.",
            "- Simile restraint: do not lean on \"像 / 仿佛 / 如同\" as default rhetoric — at most one simile per scene; prefer a precise verb and a concrete action over a figure of speech.",
            "- No padding: every scene must advance conflict, causality, emotion, evidence, pressure, payoff, or a relationship.",
            "- The climax is a scene, not a recap: eruptions of conflict, reversals, life-or-death beats, and reveals must play out beat by beat on the page (action, dialogue, the five senses). The heavier a chapter's information load, the more its key beat must be staged as a full scene — never compressed into one line like \"然后他救了人、对手落网.\"",
            "- Payoffs need setup: every reversal, comeuppance, reconciliation, revenge, or identity reveal must ride a chain of evidence and causality.",
            "- Side characters need motives: even the oppressor acts from interest, misjudgment, or fear — never a brainless plot device.",
            "- Everyday detail must become bait: each detail carries evidence, emotion, characterization, or a later reversal.",
            "- Mobile-first: short paragraphs, dense information, no vague lyricism or decorative filler.",
        ].join("\n")
    }
}

fn reference_block(reference: Option<&ShortFictionReference>, language: Language) -> String {
    match reference {
        Some(r) if !r.text.trim().is_empty() => {
            let heading = match language {
                Language::En => "## Optional Reference Text",
                Language::Zh => "## Optional Reference Text",
            };
            format!("{}\n{}\n", heading, r.text.trim())
        }
        _ => String::new(),
    }
}

fn v1_outline_header(language: Language) -> &'static str {
    match language {
        Language::En => "## Previous Outline (v1 — revise on this basis)",
        Language::Zh => "## Previous Outline (v1 — revise on this basis)",
    }
}

fn v1_draft_header(language: Language) -> &'static str {
    match language {
        Language::En => "## Previous Draft (v1 — revise on this basis)",
        Language::Zh => "## Previous Draft (v1 — revise on this basis)",
    }
}

// ═══════════════════════════════════════════════════════════════
//  解析函数
// ═══════════════════════════════════════════════════════════════

/// 解析短篇大纲输出。
pub fn parse_outline(raw_content: &str, language: Language) -> ShortFictionOutline {
    let fallback_title = untitled_short_title(language);
    let story_title = extract_tagged_block(raw_content, "SHORT_FICTION_PLAN_TITLE")
        .or_else(|| extract_tagged_block(raw_content, "SHORT_FICTION_TITLE"))
        .or_else(|| extract_first_heading(raw_content))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    ShortFictionOutline {
        story_title,
        raw_content: raw_content.trim().to_string(),
    }
}

/// 解析批量草稿输出。
pub fn parse_batch_draft(
    raw_content: &str,
    expected_chapters: u32,
    language: Language,
) -> ShortFictionBatchDraft {
    let fallback_title = untitled_short_title(language);
    let story_title = extract_tagged_block(raw_content, "SHORT_FICTION_TITLE")
        .or_else(|| extract_first_heading(raw_content))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    let opening_hook = extract_tagged_block(raw_content, "SHORT_FICTION_OPENING_HOOK")
        .or_else(|| extract_tagged_block(raw_content, "OPENING_HOOK"))
        .filter(|s| !s.is_empty());

    let mut chapters = Vec::with_capacity(expected_chapters as usize);
    for number in 1..=expected_chapters {
        let title = extract_tagged_block(raw_content, &format!("CHAPTER {} TITLE", number))
            .or_else(|| extract_markdown_chapter_title(raw_content, number))
            .map(|t| normalize_chapter_title(&t, number, language))
            .unwrap_or_else(|| fallback_chapter_title(number, language));

        let content = extract_last_nonempty_tagged_block(raw_content, &format!("CHAPTER {} CONTENT", number))
            .or_else(|| extract_duplicate_title_tagged_chapter_content(raw_content, number))
            .or_else(|| extract_markdown_chapter_content(raw_content, number))
            .map(|c| sanitize_chapter_content(&c))
            .unwrap_or_default();

        let char_count = count_chapter_length(&content, language);
        chapters.push(ShortFictionChapter {
            number,
            title,
            content,
            char_count,
        });
    }

    ShortFictionBatchDraft {
        story_title,
        opening_hook,
        chapters,
        raw_content: raw_content.to_string(),
    }
}

/// 解析销售包装输出。
pub fn parse_sales_package(raw_content: &str, fallback_title: &str) -> ShortFictionSalesPackage {
    let title = extract_tagged_block(raw_content, "SHORT_FICTION_PACKAGE_TITLE")
        .or_else(|| extract_tagged_block(raw_content, "SHORT_FICTION_TITLE"))
        .map(|t| normalize_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| fallback_title.to_string());

    let intro = extract_tagged_block(raw_content, "SHORT_FICTION_INTRO")
        .or_else(|| extract_tagged_block(raw_content, "INTRO"))
        .unwrap_or_default();

    let selling_raw = extract_tagged_block(raw_content, "SHORT_FICTION_SELLING_POINTS")
        .or_else(|| extract_tagged_block(raw_content, "SELLING_POINTS"))
        .unwrap_or_default();

    let selling_points: Vec<String> = selling_raw
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            // Strip leading "- " or "* " bullet
            let stripped = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .or_else(|| trimmed.strip_prefix("-"))
                .or_else(|| trimmed.strip_prefix("*"))
                .unwrap_or(trimmed);
            stripped.trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    let cover_prompt = extract_tagged_block(raw_content, "SHORT_FICTION_COVER_PROMPT")
        .or_else(|| extract_tagged_block(raw_content, "COVER_PROMPT"))
        .unwrap_or_default();

    ShortFictionSalesPackage {
        title,
        intro: intro.trim().to_string(),
        selling_points,
        cover_prompt: cover_prompt.trim().to_string(),
        raw_content: raw_content.trim().to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════
//  渲染与验证辅助
// ═══════════════════════════════════════════════════════════════

/// 渲染草稿为 Markdown（用于审稿/修订上下文）。
pub fn render_draft_markdown(draft: &ShortFictionBatchDraft, language: Language) -> String {
    let hook_heading = match language {
        Language::En => "## Opening Hook",
        Language::Zh => "## Opening Hook",
    };
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("# {}", draft.story_title));
    if let Some(hook) = &draft.opening_hook {
        if !hook.trim().is_empty() {
            parts.push(format!("{}\n\n{}", hook_heading, hook));
        }
    }
    for chapter in &draft.chapters {
        let heading = format_chapter_heading(chapter.number, &chapter.title, language);
        parts.push(format!("## {}\n\n{}", heading, chapter.content));
    }
    parts.join("\n\n")
}

/// 查找内容为空的章节编号。
pub fn find_empty_chapters(draft: &ShortFictionBatchDraft) -> Vec<u32> {
    draft
        .chapters
        .iter()
        .filter(|c| c.content.trim().is_empty())
        .map(|c| c.number)
        .collect()
}

/// 校验草稿完整性。
pub fn validate_draft_for_final(
    draft: &ShortFictionBatchDraft,
    expected_chapters: Option<u32>,
) -> Result<(), AppError> {
    if let Some(expected) = expected_chapters {
        if draft.chapters.len() != expected as usize {
            return Err(AppError::invalid_format(format!(
                "Short-hit draft is incomplete; expected {} chapters, got {}.",
                expected,
                draft.chapters.len()
            )));
        }
    }
    let empty = find_empty_chapters(draft);
    if !empty.is_empty() {
        let list = empty
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::invalid_format(format!(
            "Short-hit draft is incomplete; empty chapters: {}.",
            list
        )));
    }
    Ok(())
}

/// 格式化章节标题（用于落盘 Markdown）。
pub fn format_chapter_heading(number: u32, title: &str, language: Language) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return fallback_chapter_title(number, language);
    }
    match language {
        Language::En => {
            let pattern = format!(r"(?i)^Chapter\s*{}\b", number);
            if regex::Regex::new(&pattern)
                .map(|re| re.is_match(trimmed))
                .unwrap_or(false)
            {
                trimmed.to_string()
            } else {
                format!("Chapter {}: {}", number, trimmed)
            }
        }
        Language::Zh => {
            let pattern = format!(r"^第\s*{}\s*章", number);
            if regex::Regex::new(&pattern)
                .map(|re| re.is_match(trimmed))
                .unwrap_or(false)
            {
                trimmed.to_string()
            } else {
                format!("第{}章 {}", number, trimmed)
            }
        }
    }
}

/// 统计章节长度（zh=中文字符数, en=单词数）。
pub fn count_chapter_length(content: &str, language: Language) -> u32 {
    match language {
        Language::Zh => content.chars().filter(|c| is_cjk_char(*c)).count() as u32,
        Language::En => content.split_whitespace().count() as u32,
    }
}

/// 估算短篇写作 max_tokens。
/// 公式：max(12288, ceil(chapters * chars_per_chapter * 2.2) + 4096)
pub fn estimate_short_fiction_max_tokens(chapter_count: u32, chars_per_chapter: u32) -> u64 {
    let product = (chapter_count as u64) * (chars_per_chapter as u64) * 22;
    let estimated = (product + 9) / 10 + 4096;
    std::cmp::max(12288, estimated)
}

// ═══════════════════════════════════════════════════════════════
//  标签区块提取
// ═══════════════════════════════════════════════════════════════

/// 提取 `=== TAG ===` 到下一个 `=== ... ===` 之间的内容（第一个非空块）。
/// 返回首个块，空字符串视为未找到。
pub fn extract_tagged_block(content: &str, tag: &str) -> Option<String> {
    extract_tagged_blocks(content, tag)
        .into_iter()
        .next()
        .filter(|b| !b.is_empty())
}

/// 提取最后一个非空标签块。
fn extract_last_nonempty_tagged_block(content: &str, tag: &str) -> Option<String> {
    extract_tagged_blocks(content, tag)
        .into_iter()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .last()
}

/// 提取所有 `=== TAG ===` 标签块。
fn extract_tagged_blocks(content: &str, tag: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if is_tag_line(lines[i].trim(), tag) {
            i += 1;
            // 跳过标签后紧跟的空行
            if i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            let mut block: Vec<&str> = Vec::new();
            while i < lines.len() && !is_any_tag_line(lines[i].trim()) {
                block.push(lines[i]);
                i += 1;
            }
            blocks.push(block.join("\n").trim().to_string());
        } else {
            i += 1;
        }
    }
    blocks
}

/// 判断行是否为指定标签行（大小写不敏感）。
fn is_tag_line(line: &str, tag: &str) -> bool {
    let line = line.trim();
    match line.strip_prefix("===").and_then(|s| s.strip_suffix("===")) {
        Some(inner) => inner.trim().eq_ignore_ascii_case(tag),
        None => false,
    }
}

/// 判断行是否为任意标签行（`=== [A-Z0-9_ ]+ ===`，大小写不敏感）。
fn is_any_tag_line(line: &str) -> bool {
    let line = line.trim();
    let inner = match line
        .strip_prefix("===")
        .and_then(|s| s.strip_suffix("==="))
    {
        Some(s) => s.trim(),
        None => return false,
    };
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ' ')
}

// ═══════════════════════════════════════════════════════════════
//  Markdown 回退解析
// ═══════════════════════════════════════════════════════════════

/// 提取首个 `# ` 一级标题。
fn extract_first_heading(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// 从 Markdown `## ` 标题提取章节标题（回退方案）。
fn extract_markdown_chapter_title(content: &str, number: u32) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            let rest = rest.trim();
            let title = strip_chapter_prefix(rest, number);
            if !title.is_empty() {
                return Some(title.to_string());
            }
            return Some(rest.to_string());
        }
    }
    None
}

/// 从 Markdown `## ` 标题提取章节正文（回退方案）。
fn extract_markdown_chapter_content(content: &str, _number: u32) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    // 找到首个 `## ` 标题
    while i < lines.len() {
        if lines[i].trim_start().starts_with("## ") {
            i += 1;
            break;
        }
        i += 1;
    }
    if i >= lines.len() {
        return None;
    }
    let mut block: Vec<&str> = Vec::new();
    while i < lines.len() {
        if lines[i].trim_start().starts_with("## ") {
            break;
        }
        block.push(lines[i]);
        i += 1;
    }
    let content = block.join("\n").trim().to_string();
    if content.is_empty() {
        None
    } else {
        Some(content)
    }
}

/// 提取重复 `=== CHAPTER N TITLE ===` 标签后的内容。
/// 当模型意外把内容放在第二个 TITLE 标签下而非 CONTENT 标签时使用。
fn extract_duplicate_title_tagged_chapter_content(content: &str, number: u32) -> Option<String> {
    let tag = format!("CHAPTER {} TITLE", number);
    let blocks = extract_tagged_blocks(content, &tag);
    // 第二个块即重复标题后的内容
    blocks.into_iter().nth(1).filter(|b| !b.is_empty())
}

/// 剥离章节标题前缀（第N章 / Chapter N）。
fn strip_chapter_prefix(s: &str, number: u32) -> String {
    // zh: "第N章" 前缀
    let zh_prefix = format!("第{}章", number);
    if let Some(rest) = s.strip_prefix(&zh_prefix as &str) {
        return rest.trim().to_string();
    }
    // zh: "第 N 章" 带空格
    let zh_spaced = format!("第 {} 章", number);
    if let Some(rest) = s.strip_prefix(&zh_spaced as &str) {
        return rest.trim().to_string();
    }
    // en: "Chapter N" 前缀（大小写不敏感）
    let lower = s.to_lowercase();
    let en_prefix = format!("chapter {}", number);
    if let Some(rest) = lower.strip_prefix(&en_prefix) {
        // 剥离分隔符：: ： . - – — 及空格
        let stripped: &str = rest.trim_start_matches(|c: char| {
            matches!(c, ':' | '：' | '.' | '-' | '–' | '—' | ' ')
        });
        // 返回原始字符串对应位置
        let prefix_len = s.len() - rest.len();
        let sep_len = rest.len() - stripped.len();
        return s[prefix_len + sep_len..].trim().to_string();
    }
    s.trim().to_string()
}

/// 清理章节内容：去除代码围栏和残留标签行。
fn sanitize_chapter_content(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.lines().collect();
    // 去除开头代码围栏
    if let Some(first) = lines.first() {
        let f = first.trim().to_lowercase();
        if f == "```" || f == "```md" || f == "```markdown" {
            lines.remove(0);
        }
    }
    // 去除结尾代码围栏
    if let Some(last) = lines.last() {
        if last.trim() == "```" {
            lines.pop();
        }
    }
    // 去除残留的 === TAG === 行
    lines
        .into_iter()
        .filter(|line| !is_any_tag_line(line.trim()))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// 标题归一化：剥离 `#` 前缀和 `《》` 书名号。
fn normalize_title(raw: &str) -> String {
    for line in raw.lines() {
        let stripped = line.trim_start_matches('#').trim();
        if !stripped.is_empty() {
            let result = stripped
                .strip_prefix("《")
                .and_then(|s| s.strip_suffix("》"))
                .unwrap_or(stripped);
            return result.trim().to_string();
        }
    }
    String::new()
}

/// 章节标题归一化：剥离前缀，空则回退。
fn normalize_chapter_title(raw: &str, number: u32, language: Language) -> String {
    let normalized = normalize_title(raw);
    let stripped = strip_chapter_prefix(&normalized, number);
    let title = stripped.trim().to_string();
    if title.is_empty() {
        fallback_chapter_title(number, language)
    } else {
        title
    }
}

/// 未命名短篇回退标题。
fn untitled_short_title(language: Language) -> &'static str {
    match language {
        Language::En => "Untitled Short Story",
        Language::Zh => "未命名短篇",
    }
}

/// 章节回退标题。
fn fallback_chapter_title(number: u32, language: Language) -> String {
    match language {
        Language::En => format!("Chapter {}", number),
        Language::Zh => format!("第{}章", number),
    }
}

/// 判断字符是否为 CJK 汉字。
fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    matches!(
        code,
        0x4E00..=0x9FFF       // CJK Unified Ideographs
        | 0x3400..=0x4DBF     // CJK Extension A
        | 0x20000..=0x2A6DF   // CJK Extension B
        | 0x2A700..=0x2B73F   // CJK Extension C
        | 0x2B740..=0x2B81F   // CJK Extension D
        | 0x2B820..=0x2CEAF   // CJK Extension E
        | 0xF900..=0xFAFF     // CJK Compatibility Ideographs
        | 0x2F800..=0x2FA1F   // CJK Compatibility Ideographs Supplement
    )
}

// ═══════════════════════════════════════════════════════════════
//  测试
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outline_with_tagged_title() {
        let raw = "=== SHORT_FICTION_PLAN_TITLE ===\n我的短篇\n=== SHORT_FICTION_PLAN ===\n故事方案内容";
        let outline = parse_outline(raw, Language::Zh);
        assert_eq!(outline.story_title, "我的短篇");
        assert!(outline.raw_content.contains("故事方案内容"));
    }

    #[test]
    fn parses_outline_fallback_to_heading() {
        let raw = "# 回家的路\n\n故事方案";
        let outline = parse_outline(raw, Language::Zh);
        assert_eq!(outline.story_title, "回家的路");
    }

    #[test]
    fn parses_batch_draft_with_tagged_chapters() {
        let raw = r#"=== SHORT_FICTION_TITLE ===
测试短篇
=== SHORT_FICTION_OPENING_HOOK ===
开篇钩子内容
=== CHAPTER 1 TITLE ===
第一章标题
=== CHAPTER 1 CONTENT ===
第一章正文内容
=== CHAPTER 2 TITLE ===
第二章标题
=== CHAPTER 2 CONTENT ===
第二章正文内容"#;
        let draft = parse_batch_draft(raw, 2, Language::Zh);
        assert_eq!(draft.story_title, "测试短篇");
        assert_eq!(draft.opening_hook.as_deref(), Some("开篇钩子内容"));
        assert_eq!(draft.chapters.len(), 2);
        assert_eq!(draft.chapters[0].number, 1);
        assert_eq!(draft.chapters[0].title, "第一章标题");
        assert_eq!(draft.chapters[0].content, "第一章正文内容");
        assert_eq!(draft.chapters[1].number, 2);
        assert_eq!(draft.chapters[1].content, "第二章正文内容");
    }

    #[test]
    fn parses_sales_package() {
        let raw = r#"=== SHORT_FICTION_PACKAGE_TITLE ===
包装标题
=== SHORT_FICTION_INTRO ===
简介内容
=== SHORT_FICTION_SELLING_POINTS ===
- 卖点1
- 卖点2
- 卖点3
=== SHORT_FICTION_COVER_PROMPT ===
封面提示词内容"#;
        let pkg = parse_sales_package(raw, "回退标题");
        assert_eq!(pkg.title, "包装标题");
        assert_eq!(pkg.intro, "简介内容");
        assert_eq!(pkg.selling_points.len(), 3);
        assert_eq!(pkg.selling_points[0], "卖点1");
        assert_eq!(pkg.cover_prompt, "封面提示词内容");
    }

    #[test]
    fn find_empty_chapters_detects_missing_content() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![
                ShortFictionChapter {
                    number: 1,
                    title: "第一章".to_string(),
                    content: "内容".to_string(),
                    char_count: 2,
                },
                ShortFictionChapter {
                    number: 2,
                    title: "第二章".to_string(),
                    content: "   ".to_string(),
                    char_count: 0,
                },
            ],
            raw_content: String::new(),
        };
        let empty = find_empty_chapters(&draft);
        assert_eq!(empty, vec![2]);
    }

    #[test]
    fn validate_draft_rejects_empty_chapters() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "第一章".to_string(),
                content: String::new(),
                char_count: 0,
            }],
            raw_content: String::new(),
        };
        assert!(validate_draft_for_final(&draft, Some(1)).is_err());
    }

    #[test]
    fn validate_draft_rejects_wrong_chapter_count() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试".to_string(),
            opening_hook: None,
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "第一章".to_string(),
                content: "内容".to_string(),
                char_count: 2,
            }],
            raw_content: String::new(),
        };
        assert!(validate_draft_for_final(&draft, Some(2)).is_err());
    }

    #[test]
    fn count_chapter_length_zh_counts_cjk() {
        let content = "你好世界hello 123";
        assert_eq!(count_chapter_length(content, Language::Zh), 4);
    }

    #[test]
    fn count_chapter_length_en_counts_words() {
        let content = "hello world foo bar";
        assert_eq!(count_chapter_length(content, Language::En), 4);
    }

    #[test]
    fn estimate_max_tokens_respects_floor() {
        let tokens = estimate_short_fiction_max_tokens(1, 100);
        assert_eq!(tokens, 12288);
    }

    #[test]
    fn estimate_max_tokens_scales_with_chapters() {
        let tokens = estimate_short_fiction_max_tokens(12, 1000);
        // 12 * 1000 * 2.2 = 26400, ceil = 26400, + 4096 = 30496
        assert_eq!(tokens, 30496);
    }

    #[test]
    fn format_chapter_heading_zh_adds_prefix() {
        let heading = format_chapter_heading(3, "暗流", Language::Zh);
        assert_eq!(heading, "第3章 暗流");
    }

    #[test]
    fn format_chapter_heading_zh_preserves_existing_prefix() {
        let heading = format_chapter_heading(3, "第3章 暗流", Language::Zh);
        assert_eq!(heading, "第3章 暗流");
    }

    #[test]
    fn format_chapter_heading_en_adds_prefix() {
        let heading = format_chapter_heading(5, "The Storm", Language::En);
        assert_eq!(heading, "Chapter 5: The Storm");
    }

    #[test]
    fn extract_tagged_block_finds_content() {
        let raw = "前文\n=== MY_TAG ===\n标签内容\n=== NEXT_TAG ===\n其他";
        let block = extract_tagged_block(raw, "MY_TAG");
        assert_eq!(block.as_deref(), Some("标签内容"));
    }

    #[test]
    fn extract_tagged_block_case_insensitive() {
        let raw = "=== my_tag ===\n内容";
        let block = extract_tagged_block(raw, "MY_TAG");
        assert_eq!(block.as_deref(), Some("内容"));
    }

    #[test]
    fn extract_tagged_block_returns_none_when_missing() {
        let raw = "无标签内容";
        assert!(extract_tagged_block(raw, "MY_TAG").is_none());
    }

    #[test]
    fn extract_last_nonempty_gets_final_block() {
        let raw = "=== CHAPTER 1 CONTENT ===\n\n=== CHAPTER 1 CONTENT ===\n实际内容";
        let block = extract_last_nonempty_tagged_block(raw, "CHAPTER 1 CONTENT");
        assert_eq!(block.as_deref(), Some("实际内容"));
    }

    #[test]
    fn render_draft_markdown_includes_all_chapters() {
        let draft = ShortFictionBatchDraft {
            story_title: "测试短篇".to_string(),
            opening_hook: Some("钩子".to_string()),
            chapters: vec![ShortFictionChapter {
                number: 1,
                title: "开局".to_string(),
                content: "正文".to_string(),
                char_count: 2,
            }],
            raw_content: String::new(),
        };
        let md = render_draft_markdown(&draft, Language::Zh);
        assert!(md.contains("# 测试短篇"));
        assert!(md.contains("## Opening Hook"));
        assert!(md.contains("## 第1章 开局"));
        assert!(md.contains("正文"));
    }

    #[test]
    fn sanitize_strips_code_fences() {
        let raw = "```md\n内容行\n```";
        let result = sanitize_chapter_content(raw);
        assert_eq!(result, "内容行");
    }

    #[test]
    fn sanitize_removes_tag_lines() {
        let raw = "内容行1\n=== SOME_TAG ===\n内容行2";
        let result = sanitize_chapter_content(raw);
        assert_eq!(result, "内容行1\n内容行2");
    }

    #[test]
    fn normalize_title_strips_book_marks() {
        assert_eq!(normalize_title("《我的标题》"), "我的标题");
    }

    #[test]
    fn normalize_title_strips_heading_marks() {
        assert_eq!(normalize_title("## 标题"), "标题");
    }
}
