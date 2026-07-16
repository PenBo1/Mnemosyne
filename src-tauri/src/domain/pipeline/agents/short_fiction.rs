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
        ["你是短篇网文的主编。你的工作是把一条创作方向变成一份完整的短篇故事方案，最终成稿用英文撰写。",
            "只能基于本方向与用户提供的参考文本创作；永远不要声称读过、引用过、继承过未提供的素材。",
            "内容为王：标题、开篇、压在主角身上的压力、证据/关系/身份筹码、升级链、反转链、回报落地，都必须强到足以支撑一次性整篇成稿。",
            "不要过度结构化，不要输出 JSON/YAML。写人类可读的 Markdown，但章节方案必须密到让写手能一次性写出整篇故事。",
            "短篇默认 12-18 章，每章约 600-800 英文单词。故事必须完整——不是某部小说起手前 5 章。",
            "<safety>",
            "- NEVER 声称读过、引用过或继承过用户未提供的素材。",
            "- NEVER 输出 JSON/YAML 或用过度结构化清单替代故事方案。",
            "- NEVER 把短篇写成「小说起手前 5 章」；必须有完整开端—升级—反转—回报闭环。",
            "</safety>"].join("\n")
    } else {
        ["你是短篇网文的主编。你的工作是把一条创作方向变成一份完整的短篇故事方案。方案与最终正文均用简体中文撰写。",
            "只能基于本方向与用户提供的参考文本创作；永远不要声称读过、引用过、继承过未提供的素材。",
            "内容为王：标题、开篇、压在主角身上的压力、证据/关系/身份筹码、升级链、反转链、回报落地，都必须强到足以支撑一次性整篇成稿。",
            "不要过度结构化，不要输出 JSON/YAML。写人类可读的 Markdown，但章节方案必须密到让写手能一次性写出整篇故事。",
            "短篇默认 12-18 章，每章约 900-1200 个中文字符。故事必须完整——不是某部小说起手前 5 章。",
            "<safety>",
            "- NEVER 声称读过、引用过或继承过用户未提供的素材。",
            "- NEVER 输出 JSON/YAML 或用过度结构化清单替代故事方案。",
            "- NEVER 把短篇写成「小说起手前 5 章」；必须有完整开端—升级—反转—回报闭环。",
            "</safety>"].join("\n")
    }
}

fn build_outline_user_prompt(input: &ShortFictionOutlineInput) -> String {
    let reference = reference_block(input.reference.as_ref(), input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 目标规格".to_string(),
            format!(
                "一部完整的短篇，共 {} 章，每章约 {} 个英文单词。",
                input.chapter_count, input.chars_per_chapter
            ),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## 交付物".to_string(),
            "先给一个平台可点击的标题，再给完整故事方案。方案必须讲清：主角为何被钉死、读者在等什么回报、主角如何翻盘、证据/关系/身份/规则如何一步步升级、对手为何反扑、结局如何落地。".to_string(),
            "章节方案必须逐章写清：章节标题方向、关键上场场景、人物行动、升级或兑现、章末让人继续读的理由。".to_string(),
            "允许写标签，但不要罗列表格；标签服务于选材与写作——绝不替代故事本身。".to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "恰好一行平台可用标题".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "完整故事方案（Markdown，英文）：题材/受众、标题方向、开篇钩子、人物与关系、核心压力、主角如何赢、升级链、反转链、结局回报、逐章方案。".to_string(),
        ]);
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 目标规格".to_string(),
            format!(
                "一部完整的短篇，共 {} 章，每章约 {} 个中文字符。",
                input.chapter_count, input.chars_per_chapter
            ),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## 交付物".to_string(),
            "先给一个平台可点击的标题，再给完整故事方案。方案必须讲清：主角为何被钉死、读者在等什么回报、主角如何翻盘、证据/关系/身份/规则如何一步步升级、对手为何反扑、结局如何落地。".to_string(),
            "章节方案必须逐章写清：章节标题方向、关键上场场景、人物行动、升级或兑现、章末让人继续读的理由。".to_string(),
            "允许写标签，但不要罗列表格；标签服务于选材与写作——绝不替代故事本身。".to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "恰好一行平台可用标题（简体中文）".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "完整故事方案（Markdown，简体中文）：题材/受众、标题方向、开篇钩子、人物与关系、核心压力、主角如何赢、升级链、反转链、结局回报、逐章方案。".to_string(),
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
        ["你是短篇故事方案审稿人。你不打分，也不查重。",
            "你的工作是判断这份故事方案能否支撑一次性整篇成稿：题材引擎是否清晰、人物动机是否成立、压力链是否升级、对手反扑是否可信、结局回报是否足够大。",
            "像真实读者和真实编辑那样审稿，不要做清单机器。",
            "输出 Markdown。点名会让成稿塌掉的缺陷，以及值得保留的优点。",
            "<safety>",
            "- NEVER 打分或查重；那是别的环节的事。",
            "- NEVER 把审稿变成清单勾选；必须像真人读者一样判断。",
            "- NEVER 仅凭字数或长度否定方案；先判断内容是否完整、戏剧是否到位。",
            "</safety>"].join("\n")
    } else {
        ["你是短篇故事方案审稿人。你不打分，也不查重。审稿意见用简体中文撰写。",
            "你的工作是判断这份故事方案能否支撑一次性整篇成稿：题材引擎是否清晰、人物动机是否成立、压力链是否升级、对手反扑是否可信、结局回报是否足够大。",
            "像真实读者和真实编辑那样审稿，不要做清单机器。",
            "输出 Markdown（简体中文）。点名会让成稿塌掉的缺陷，以及值得保留的优点。",
            "<safety>",
            "- NEVER 打分或查重；那是别的环节的事。",
            "- NEVER 把审稿变成清单勾选；必须像真人读者一样判断。",
            "- NEVER 仅凭字数或长度否定方案；先判断内容是否完整、戏剧是否到位。",
            "</safety>"].join("\n")
    }
}

fn build_outline_review_user_prompt(input: &ShortFictionOutlineReviewInput) -> String {
    let reference = reference_block(input.reference.as_ref(), input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## 待审故事方案".to_string(),
            input.outline.raw_content.clone(),
            String::new(),
            "## 审稿焦点".to_string(),
            "- 这是完整的短篇，还是只是个试写片段方案？".to_string(),
            "- 标题、开篇、前 3 章是否给了读者点击并继续读的理由？".to_string(),
            "- 方案密度够吗，还是写手到下半篇会没料可写？".to_string(),
            "- 关键场景里有没有人物行动、反扑、兑现，而不是赤裸的结果摘要？".to_string(),
            "- 读者会不会被时间线、关系、证据获取、身体状态、常识问题踢出故事？".to_string(),
        ]);
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
        ];
        if !reference.is_empty() {
            parts.push(reference);
        }
        parts.extend(vec![
            "## 待审故事方案".to_string(),
            input.outline.raw_content.clone(),
            String::new(),
            "## 审稿焦点".to_string(),
            "- 这是完整的短篇，还是只是个试写片段方案？".to_string(),
            "- 标题、开篇、前 3 章是否给了读者点击并继续读的理由？".to_string(),
            "- 方案密度够吗，还是写手到下半篇会没料可写？".to_string(),
            "- 关键场景里有没有人物行动、反扑、兑现，而不是赤裸的结果摘要？".to_string(),
            "- 读者会不会被时间线、关系、证据获取、身体状态、常识问题踢出故事？".to_string(),
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
            "基于上面的方案审稿意见，产出完整的第二版故事方案。".to_string(),
            "这是同一项目的第二轮：不要从零重写，也不要输出「修改清单」替代方案。".to_string(),
            format!(
                "保持结构：{} 章，每章约 {} 个英文单词。",
                input.chapter_count, input.chars_per_chapter
            ),
            "保留有效的题材引擎与人物关系；修复会让成稿塌掉的缺陷。".to_string(),
            String::new(),
            "## 方案审稿意见".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "恰好一行平台可用标题".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "完整的第二版故事方案（Markdown，英文）。".to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    } else {
        vec![
            "基于上面的方案审稿意见，产出完整的第二版故事方案。用简体中文撰写。".to_string(),
            "这是同一项目的第二轮：不要从零重写，也不要输出「修改清单」替代方案。".to_string(),
            format!(
                "保持结构：{} 章，每章约 {} 个中文字符。",
                input.chapter_count, input.chars_per_chapter
            ),
            "保留有效的题材引擎与人物关系；修复会让成稿塌掉的缺陷。".to_string(),
            String::new(),
            "## 方案审稿意见".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_PLAN_TITLE ===".to_string(),
            "恰好一行平台可用标题（简体中文）".to_string(),
            "=== SHORT_FICTION_PLAN ===".to_string(),
            "完整的第二版故事方案（Markdown，简体中文）。".to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    }
}

fn build_writer_system_prompt(language: Language) -> String {
    if language == Language::En {
        ["你是英文短篇 BatchWriter。按故事方案，在一次 API 调用内写出整篇短篇正文。",
            "写自然、地道的英文散文。句长要有变化；短促有力的句子与较长流畅的句子交替，叙事声音全篇保持一致。",
            "这不是长篇连载续写，也不是章节梗概。每一章都要有戏剧在场上发生：人物行动、对白或反应、情境转变、章末让人继续读的理由。",
            "戏剧强度要拉满，网文风格：现实压力可以放大到读者仍愿相信的极限，但绝不能荒诞到打破沉浸。",
            "故事标题与章节标题要像平台内容，而不是文学摘要。散文节奏要适配手机阅读——短段落，但绝不写成电报式片段。",
            "字数是校准，不是平均。大场景可以长，过渡可以短；明显过短的章节通常意味着你写成了梗概，必须补上真实场景。",
            "输出必须严格使用指定块。不要作者注、字数说明、审稿意见、格式解释。",
            "<safety>",
            "- NEVER 写成梗概或章节大纲；每章必须有真实场景在场上发生。",
            "- NEVER 输出指定块之外的作者注、字数说明或审稿意见。",
            "- NEVER 中途拐到另一个故事；必须承接方案的压力链、证据链、反转链与情感回报。",
            "</safety>"].join("\n")
    } else {
        ["你是简体中文短篇 BatchWriter。按故事方案，在一次 API 调用内写出整篇短篇正文。",
            "这不是长篇连载续写，也不是章节梗概。每一章都要有戏剧在场上发生：人物行动、对白或反应、情境转变、章末让人继续读的理由。",
            "戏剧强度要拉满，网文风格：现实压力可以放大到读者仍愿相信的极限，但绝不能荒诞到打破沉浸。",
            "故事标题与章节标题要像平台内容，而不是文学摘要。散文节奏要适配手机阅读——短段落，但绝不写成电报式片段。",
            "字数是校准，不是平均。大场景可以长，过渡可以短；明显过短的章节通常意味着你写成了梗概，必须补上真实场景。",
            "输出必须严格使用指定块。不要作者注、字数说明、审稿意见、格式解释。",
            "<safety>",
            "- NEVER 写成梗概或章节大纲；每章必须有真实场景在场上发生。",
            "- NEVER 输出指定块之外的作者注、字数说明或审稿意见。",
            "- NEVER 中途拐到另一个故事；必须承接方案的压力链、证据链、反转链与情感回报。",
            "</safety>"].join("\n")
    }
}

fn build_writer_user_prompt(input: &ShortFictionDraftInput) -> String {
    let craft = build_craft_prompt(input.language);
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "## 任务".to_string(),
            format!(
                "一次性写出完整的 {} 章故事，每章约 {} 个英文单词。",
                input.chapter_count, input.chars_per_chapter
            ),
            "动笔前先读完整个故事方案。正文必须承接方案的压力链、证据链、反转链与情感回报——不得中途拐到另一个故事。".to_string(),
            String::new(),
            craft,
            String::new(),
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 故事方案".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "故事标题——纯文本，平台可用，不要其它内容".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "可选的正文前钩子，约 130 个英文单词；若不需要独立 teaser，仍要写出第 1 章开头的小型首屏场景".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## 任务".to_string(),
            format!(
                "一次性写出完整的 {} 章故事，每章约 {} 个中文字符。",
                input.chapter_count, input.chars_per_chapter
            ),
            "动笔前先读完整个故事方案。正文必须承接方案的压力链、证据链、反转链与情感回报——不得中途拐到另一个故事。".to_string(),
            String::new(),
            craft,
            String::new(),
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 故事方案".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "故事标题——纯文本，平台可用，简体中文，不要其它内容".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "可选的正文前钩子，约 200 个中文字符；若不需要独立 teaser，仍要写出第 1 章开头的小型首屏场景".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀（简体中文）".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注（简体中文）", chapter));
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
            "## 任务".to_string(),
            format!(
                "上一稿被截断或跳章。仅补写缺失章节：{}。",
                missing_str
            ),
            format!(
                "保持校准：完整 {} 章短篇，每章约 {} 个英文单词。",
                input.chapter_count, input.chars_per_chapter
            ),
            "不要重写已完成章节，不要写总结说明，不要道歉，不要输出审稿意见。".to_string(),
            String::new(),
            craft,
            String::new(),
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 故事方案".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## 已有草稿（仅供衔接——不要重写）".to_string(),
            existing_md,
            String::new(),
            "## 输出格式".to_string(),
        ];
        for &chapter in missing {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "## 任务".to_string(),
            format!(
                "上一稿被截断或跳章。仅补写缺失章节：{}。",
                missing_str
            ),
            format!(
                "保持校准：完整 {} 章短篇，每章约 {} 个中文字符。",
                input.chapter_count, input.chars_per_chapter
            ),
            "不要重写已完成章节，不要写总结说明，不要道歉，不要输出审稿意见。".to_string(),
            String::new(),
            craft,
            String::new(),
            "## 创作方向".to_string(),
            input.direction.clone(),
            String::new(),
            "## 故事方案".to_string(),
            input.outline_markdown.clone(),
            String::new(),
            "## 已有草稿（仅供衔接——不要重写）".to_string(),
            existing_md,
            String::new(),
            "## 输出格式".to_string(),
        ];
        for &chapter in missing {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀（简体中文）".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注（简体中文）", chapter));
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
        ["你是短篇草稿审稿人。",
            "你只判断内容能否卖得动、读得顺、能不能持续拉着读者往前；不要把审稿变成确定性打分。",
            "关注：标题、章节标题、开篇、人物动机、时间线、关系、证据与获取、压力升级、对手反扑、下半篇是否疲软、结局回报是否落地。",
            "输出 Markdown。把会让读者明显弃读的问题与可接受的小瑕疵分开。",
            "<safety>",
            "- NEVER 把审稿变成确定性打分；那是别的环节的事。",
            "- NEVER 仅凭字数微偏否定章节；先判断内容是否完整、戏剧是否到位、回报是否落地。",
            "- NEVER 把小瑕疵与致命问题混在一起；必须分开列出。",
            "</safety>"].join("\n")
    } else {
        ["你是短篇草稿审稿人。审稿意见用简体中文撰写。",
            "你只判断内容能否卖得动、读得顺、能不能持续拉着读者往前；不要把审稿变成确定性打分。",
            "关注：标题、章节标题、开篇、人物动机、时间线、关系、证据与获取、压力升级、对手反扑、下半篇是否疲软、结局回报是否落地。",
            "输出 Markdown（简体中文）。把会让读者明显弃读的问题与可接受的小瑕疵分开。",
            "<safety>",
            "- NEVER 把审稿变成确定性打分；那是别的环节的事。",
            "- NEVER 仅凭字数微偏否定章节；先判断内容是否完整、戏剧是否到位、回报是否落地。",
            "- NEVER 把小瑕疵与致命问题混在一起；必须分开列出。",
            "</safety>"].join("\n")
    }
}

fn build_draft_review_user_prompt(input: &ShortFictionDraftReviewInput) -> String {
    let draft_md = render_draft_markdown(&input.draft, input.language);
    if input.language == Language::En {
        ["## 创作方向",
            &input.direction,
            "",
            "## 原始故事方案",
            &input.outline_markdown,
            "",
            "## 待审草稿",
            &draft_md,
            "",
            "## 审稿说明",
            "像真人那样说话：哪里拉着读者走、哪里打破沉浸、哪里读起来像梗概、下半篇哪里疲软、哪个标题或章节标题没人会点？",
            "绝不要仅凭章节略短或略长就否定；先判断内容是否完整、戏剧是否到位、回报是否落地。"]
        .join("\n")
    } else {
        ["## 创作方向",
            &input.direction,
            "",
            "## 原始故事方案",
            &input.outline_markdown,
            "",
            "## 待审草稿",
            &draft_md,
            "",
            "## 审稿说明",
            "像真人那样说话：哪里拉着读者走、哪里打破沉浸、哪里读起来像梗概、下半篇哪里疲软、哪个标题或章节标题没人会点？审稿意见用简体中文撰写。",
            "绝不要仅凭章节略短或略长就否定；先判断内容是否完整、戏剧是否到位、回报是否落地。"]
        .join("\n")
    }
}

fn build_draft_revision_followup(input: &ShortFictionDraftRevisionInput) -> String {
    if input.language == Language::En {
        let mut parts: Vec<String> = vec![
            "基于审稿意见，写出完整的第二版草稿。".to_string(),
            "这是同一故事的第二轮：保留上一版有效的部分，修复打破沉浸或让人不想继续读的问题。".to_string(),
            "不要输出「建议修改清单」，也不要只补几章——必须输出完整草稿。".to_string(),
            String::new(),
            "## 审稿意见".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## 第二轮优先级".to_string(),
            "- 修复打破沉浸的问题：时间线、逻辑、关系、证据获取、身体状态。".to_string(),
            "- 给下半篇补上真实场景；绝不要用结果摘要收尾。".to_string(),
            "- 标题、开篇、章节标题、主标题要与正文一致；不过标题可基于终稿重新锐化以提升平台点击吸引力。".to_string(),
            "- 字数仅作校准：用真实场景补足过短章节；通过删减解释和重复反应来精简过长章节。".to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "故事标题——纯文本，平台可用，不要其它内容".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "可选的正文前钩子，约 130 个英文单词；若不需要独立 teaser，仍要写出第 1 章开头的小型首屏场景".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注", chapter));
        }
        parts
    } else {
        let mut parts: Vec<String> = vec![
            "基于审稿意见，写出完整的第二版草稿。用简体中文撰写。".to_string(),
            "这是同一故事的第二轮：保留上一版有效的部分，修复打破沉浸或让人不想继续读的问题。".to_string(),
            "不要输出「建议修改清单」，也不要只补几章——必须输出完整草稿。".to_string(),
            String::new(),
            "## 审稿意见".to_string(),
            input.review.trim().to_string(),
            String::new(),
            "## 第二轮优先级".to_string(),
            "- 修复打破沉浸的问题：时间线、逻辑、关系、证据获取、身体状态。".to_string(),
            "- 给下半篇补上真实场景；绝不要用结果摘要收尾。".to_string(),
            "- 标题、开篇、章节标题、主标题要与正文一致；不过标题可基于终稿重新锐化以提升平台点击吸引力。".to_string(),
            "- 字数仅作校准：用真实场景补足过短章节；通过删减解释和重复反应来精简过长章节。".to_string(),
            String::new(),
            "## 输出格式".to_string(),
            "=== SHORT_FICTION_TITLE ===".to_string(),
            "故事标题——纯文本，平台可用，简体中文，不要其它内容".to_string(),
            "=== SHORT_FICTION_OPENING_HOOK ===".to_string(),
            "可选的正文前钩子，约 200 个中文字符；若不需要独立 teaser，仍要写出第 1 章开头的小型首屏场景".to_string(),
        ];
        for chapter in 1..=input.chapter_count {
            parts.push(format!("=== CHAPTER {} TITLE ===", chapter));
            parts.push("章节标题——纯文本，无 #，无 \"Chapter N\" 前缀（简体中文）".to_string());
            parts.push(format!("=== CHAPTER {} CONTENT ===", chapter));
            parts.push(format!("第 {} 章正文——完整场景，无梗概，无作者注（简体中文）", chapter));
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
        ["你是短篇包装编辑。基于终稿产出简介、卖点和封面图提示词。",
            "永远不要编造与终稿不同的主标题。所有包装必须围绕终稿实际标题与剧情。",
            "把封面提示词想象成手机端竖屏书封：3:4 竖屏、大标题区、强角色情绪、一两个一眼可辨的道具、高对比配色——不要电影海报风。",
            "<safety>",
            "- NEVER 编造与终稿不同的主标题；包装必须围绕终稿实际标题与剧情。",
            "- NEVER 把封面提示词写成电影海报风或宽屏横版；必须是 3:4 竖屏手机书封。",
            "- NEVER 在简介里剧透完整剧情走向；只抓冲突、压力与回报。",
            "</safety>"].join("\n")
    } else {
        ["你是短篇包装编辑。基于终稿产出简介、卖点和封面图提示词。简介与卖点用简体中文撰写。",
            "永远不要编造与终稿不同的主标题。所有包装必须围绕终稿实际标题与剧情。",
            "把封面提示词想象成手机端竖屏书封：3:4 竖屏、大标题区、强角色情绪、一两个一眼可辨的道具、高对比配色——不要电影海报风。",
            "<safety>",
            "- NEVER 编造与终稿不同的主标题；包装必须围绕终稿实际标题与剧情。",
            "- NEVER 把封面提示词写成电影海报风或宽屏横版；必须是 3:4 竖屏手机书封。",
            "- NEVER 在简介里剧透完整剧情走向；只抓冲突、压力与回报。",
            "</safety>"].join("\n")
    }
}

fn build_package_user_prompt(input: &ShortFictionPackageInput) -> String {
    let draft_md = render_draft_markdown(&input.draft, input.language);
    if input.language == Language::En {
        vec![
            "## 创作方向",
            &input.direction,
            "",
            "## 故事方案",
            &input.outline_markdown.trim(),
            "",
            "## 终稿",
            &draft_md.trim(),
            "",
            "## 输出格式",
            "=== SHORT_FICTION_PACKAGE_TITLE ===",
            &input.draft.story_title,
            "=== SHORT_FICTION_INTRO ===",
            "70-120 个英文单词的平台简介，抓冲突、压力与回报——绝不要剧透式逐幕流水账。",
            "=== SHORT_FICTION_SELLING_POINTS ===",
            "- 3 到 6 条卖点，每条一行",
            "=== SHORT_FICTION_COVER_PROMPT ===",
            "英文封面生成提示词：3:4 竖屏、主标题区、角色情绪、道具、配色、字体风格、需要避免的元素。",
        ]
        .join("\n")
    } else {
        vec![
            "## 创作方向",
            &input.direction,
            "",
            "## 故事方案",
            &input.outline_markdown.trim(),
            "",
            "## 终稿",
            &draft_md.trim(),
            "",
            "## 输出格式",
            "=== SHORT_FICTION_PACKAGE_TITLE ===",
            &input.draft.story_title,
            "=== SHORT_FICTION_INTRO ===",
            "100-180 个中文字符的平台简介，抓冲突、压力与回报——绝不要剧透式逐幕流水账。用简体中文撰写。",
            "=== SHORT_FICTION_SELLING_POINTS ===",
            "- 3 到 6 条卖点，每条一行（简体中文）",
            "=== SHORT_FICTION_COVER_PROMPT ===",
            "简体中文封面生成提示词：3:4 竖屏、主标题区、角色情绪、道具、配色、字体风格、需要避免的元素。",
        ]
        .join("\n")
    }
}

fn build_craft_prompt(language: Language) -> String {
    if language == Language::En {
        ["## 写作技艺提醒",
            "- 盐溶于汤：价值观与野心通过行动体现，绝不通过口号。",
            "- 展示而非告知：让行为、证据、具体细节和舞台调度让读者感受到角色状态。",
            "- 慎用比喻：不要把 \"like / as if / as though\" 当作默认修辞——每场至多一处比喻；优先用精准动词和具体行动，而非比喻。",
            "- 反 AI 口吻：限量使用 AI 高频词（delve, tapestry, testament, intricate, pivotal）；不要把 \"It wasn't X; it was Y\" 句式当拐杖；把分析报告用语（\"core motivation\", \"strategic advantage\"）排除在散文之外。",
            "- 不注水：每场戏必须推进冲突、因果、情感、证据、压力、回报或关系。",
            "- 高潮是场景不是复盘：冲突爆发、反转、生死节拍、揭示必须逐拍在场上演（行动、对白、五感）。章节信息负载越重，关键节拍越要展开为完整场景——绝不压缩成 \"then he saved her and the rival was arrested.\" 这种一行话。",
            "- 回报需要铺垫：每个反转、报应、和解、复仇或身份揭示都必须依托证据链与因果链。",
            "- 配角也要有动机：即使是压迫者，也是出于利益、误判或恐惧——绝不当无脑剧情工具。",
            "- 日常细节要变成饵：每个细节都要承载证据、情感、人物塑造或后续反转。",
            "- 移动优先：短段落、高信息密度，不要空泛抒情或装饰性填充。"].join("\n")
    } else {
        ["## 写作技艺提醒",
            "- 盐溶于汤：价值观与野心通过行动体现，绝不通过口号。",
            "- 展示而非告知：让行为、证据、具体细节和舞台调度让读者感受到角色状态。",
            "- 慎用比喻：不要把 \"像 / 仿佛 / 如同\" 当作默认修辞——每场至多一处比喻；优先用精准动词和具体行动，而非比喻。",
            "- 不注水：每场戏必须推进冲突、因果、情感、证据、压力、回报或关系。",
            "- 高潮是场景不是复盘：冲突爆发、反转、生死节拍、揭示必须逐拍在场上演（行动、对白、五感）。章节信息负载越重，关键节拍越要展开为完整场景——绝不压缩成 \"然后他救了人、对手落网.\" 这种一行话。",
            "- 回报需要铺垫：每个反转、报应、和解、复仇或身份揭示都必须依托证据链与因果链。",
            "- 配角也要有动机：即使是压迫者，也是出于利益、误判或恐惧——绝不当无脑剧情工具。",
            "- 日常细节要变成饵：每个细节都要承载证据、情感、人物塑造或后续反转。",
            "- 移动优先：短段落、高信息密度，不要空泛抒情或装饰性填充。"].join("\n")
    }
}

fn reference_block(reference: Option<&ShortFictionReference>, language: Language) -> String {
    match reference {
        Some(r) if !r.text.trim().is_empty() => {
            let heading = match language {
                Language::En => "## 可选参考文本",
                Language::Zh => "## 可选参考文本",
            };
            format!("{}\n{}\n", heading, r.text.trim())
        }
        _ => String::new(),
    }
}

fn v1_outline_header(language: Language) -> &'static str {
    match language {
        Language::En => "## 上一版方案（v1——在此基础上修订）",
        Language::Zh => "## 上一版方案（v1——在此基础上修订）",
    }
}

fn v1_draft_header(language: Language) -> &'static str {
    match language {
        Language::En => "## 上一版草稿（v1——在此基础上修订）",
        Language::Zh => "## 上一版草稿（v1——在此基础上修订）",
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
    let estimated = product.div_ceil(10) + 4096;
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
        .next_back()
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
