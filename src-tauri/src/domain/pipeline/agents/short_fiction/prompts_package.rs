//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Prompts Package - 销售包装阶段 Prompt 构造
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：generate_package agent 的 system/user prompt 构造。

use crate::domain::pipeline::types::Language;

use super::helpers::render_draft_markdown;
use super::types::ShortFictionPackageInput;

// ═══════════════════════════════════════════════════════════════
//  Prompt 构造函数 —— 销售包装
// ═══════════════════════════════════════════════════════════════

pub(super) fn build_package_system_prompt(language: Language) -> String {
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

pub(super) fn build_package_user_prompt(input: &ShortFictionPackageInput) -> String {
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
