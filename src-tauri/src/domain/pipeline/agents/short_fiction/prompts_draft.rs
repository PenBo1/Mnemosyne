//! ═══════════════════════════════════════════════════════════════════════════
//! Short Fiction Prompts Draft - 草稿阶段 Prompt 构造
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：write_draft / continue_draft / review_draft / revise_draft 四个 agent 的
//! system/user prompt 与修订指令构造，以及草稿阶段共享的写作技艺提醒、v1 标头辅助函数。

use crate::domain::pipeline::types::Language;

use super::helpers::render_draft_markdown;
use super::types::{
    ShortFictionDraftContinuationInput, ShortFictionDraftInput, ShortFictionDraftReviewInput,
    ShortFictionDraftRevisionInput,
};

// ═══════════════════════════════════════════════════════════════
//  Prompt 构造函数 —— 草稿
// ═══════════════════════════════════════════════════════════════

pub(super) fn build_writer_system_prompt(language: Language) -> String {
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

pub(super) fn build_writer_user_prompt(input: &ShortFictionDraftInput) -> String {
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

pub(super) fn build_draft_continuation_user_prompt(
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

pub(super) fn build_draft_review_system_prompt(language: Language) -> String {
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

pub(super) fn build_draft_review_user_prompt(input: &ShortFictionDraftReviewInput) -> String {
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

pub(super) fn build_draft_revision_followup(input: &ShortFictionDraftRevisionInput) -> String {
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

pub(super) fn v1_draft_header(language: Language) -> &'static str {
    match language {
        Language::En => "## 上一版草稿（v1——在此基础上修订）",
        Language::Zh => "## 上一版草稿（v1——在此基础上修订）",
    }
}
