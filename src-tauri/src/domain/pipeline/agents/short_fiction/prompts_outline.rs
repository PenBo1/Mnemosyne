//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Prompts Outline - 大纲阶段 Prompt 构造
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：create_outline / review_outline / revise_outline 三个 agent 的 system/user
//! prompt 与修订指令构造，以及大纲阶段共享的参考文本块、v1 标头辅助函数。

use crate::domain::pipeline::types::Language;

use super::types::{
    ShortFictionOutlineInput, ShortFictionOutlineReviewInput, ShortFictionOutlineRevisionInput,
    ShortFictionReference,
};

// ═══════════════════════════════════════════════════════════════
//  Prompt 构造函数 —— 大纲
// ═══════════════════════════════════════════════════════════════

pub(super) fn build_outline_system_prompt(language: Language) -> String {
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
            "- NEVER 声称读过、引用过或继承过未提供的素材。",
            "- NEVER 输出 JSON/YAML 或用过度结构化清单替代故事方案。",
            "- NEVER 把短篇写成「小说起手前 5 章」；必须有完整开端—升级—反转—回报闭环。",
            "</safety>"].join("\n")
    }
}

pub(super) fn build_outline_user_prompt(input: &ShortFictionOutlineInput) -> String {
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

pub(super) fn build_outline_review_system_prompt(language: Language) -> String {
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

pub(super) fn build_outline_review_user_prompt(input: &ShortFictionOutlineReviewInput) -> String {
    let reference = reference_block(input.reference.as_ref(), input.language);
    // 注：原 if/else 两个分支逻辑完全相同（clippy::if_same_then_else），已合并为单分支。
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
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn build_outline_revision_followup(input: &ShortFictionOutlineRevisionInput) -> String {
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

pub(super) fn v1_outline_header(language: Language) -> &'static str {
    match language {
        Language::En => "## 上一版方案（v1——在此基础上修订）",
        Language::Zh => "## 上一版方案（v1——在此基础上修订）",
    }
}
