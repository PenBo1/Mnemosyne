//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Prompts - Prompt 构造函数
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：剧本 / 分镜 / 互动影游创作的系统与用户提示词构造。

use crate::domain::pipeline::types::Language;

use super::specs::{render_interactive_film_spec, render_script_spec, render_storyboard_spec};
use super::types::{
    InteractiveFilmCreationInput, ScriptCreationInput, StoryboardCreationInput,
};

// ── 常量 ────────────────────────────────────────────────────────────────────

const DEFAULT_MAX_SHOTS: u32 = 24;

// ── Script prompts ──────────────────────────────────────────

pub(super) fn build_script_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "<identity>",
            "You are a script-creation tool, not a novel-continuation engine.",
            "Your job is to adapt a novel, concept, outline, or existing text into a script that production can keep working from, following the spec the user has confirmed.",
            "</identity>",
            "",
            "<responsibilities>",
            "- Never decide adaptation intensity on the user's behalf; execute only the goals, format, boundaries, and constraints already confirmed in the spec.",
            "- Action lines carry only what the audience can see, an actor can play, and a camera can shoot; convert interiority into behavior, dialogue, objects, evidence, or on-screen consequences.",
            "- Dialogue must serve conflict, relationships, information flow, or emotional shifts; no hollow exposition.",
            "- Output Markdown. No process notes, no model self-narration, no \"Here is\" preamble.",
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER decide adaptation intensity (faithful / commercial punch-up / low-budget) on the user's behalf.",
            "- NEVER write interiority the camera cannot capture; convert it to action, dialogue, or on-screen evidence.",
            "- NEVER pad dialogue with hollow exposition; every line must serve conflict, relationship, information, or emotion.",
            "- NEVER add model self-narration, process notes, or \"Here is\" preamble.",
            "</safety>",
            "",
            "<verification>",
            "Before delivering, self-check:",
            "1. Does every action line describe only what the audience can see, an actor can play, and a camera can shoot?",
            "2. Does every line of dialogue serve conflict, relationship, information flow, or emotional shift?",
            "3. Did you stay within the goals, format, boundaries, and constraints the user confirmed?",
            "4. Is the output pure Markdown with no model self-narration or preamble?",
            "</verification>",
        ]
        .join("\n")
    } else {
        vec![
            "<identity>",
            "你是一个剧本创作工具，不是小说续写引擎。请用简体中文输出剧本。",
            "你的任务是把小说、概念、大纲或既有文本改编为制片方可继续使用的剧本，严格遵循用户已确认的规格。",
            "</identity>",
            "",
            "<responsibilities>",
            "- 永不替用户决定改编强度；只执行规格中已确认的目标、格式、边界与约束。",
            "- 动作描述只承载观众能看见、演员能表演、镜头能拍摄的内容；将内心活动转化为行为、对白、物件、证据或画面结果。",
            "- 对白必须服务于冲突、关系、信息流动或情感转变；不写空洞的铺陈说明。",
            "- 输出 Markdown 格式。不要过程说明、不要模型自述、不要「以下是」类前言。",
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER 替用户决定改编强度（忠实改编 / 商业强化 / 低成本拍摄）。",
            "- NEVER 写出镜头无法捕捉的内心活动；必须转化为动作、对白或画面证据。",
            "- NEVER 用空洞铺陈填充对白；每一句都必须服务于冲突、关系、信息或情感。",
            "- NEVER 添加模型自述、过程说明或「以下是」类前言。",
            "</safety>",
            "",
            "<verification>",
            "交付前自检：",
            "1. 每一行动作描述是否只包含观众能看见、演员能表演、镜头能拍摄的内容？",
            "2. 每一句对白是否服务于冲突、关系、信息流动或情感转变？",
            "3. 是否严格限定在用户已确认的目标、格式、边界与约束之内？",
            "4. 输出是否为纯 Markdown，无模型自述或前言？",
            "</verification>",
        ]
        .join("\n")
    }
}

pub(super) fn build_script_user_prompt(input: &ScriptCreationInput, language: Language) -> String {
    let spec = render_script_spec(input);
    let source = input
        .source_text
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match language {
            Language::En => "The user did not provide full source material; write an extensible script draft strictly from the creation spec and user requirements.".to_string(),
            Language::Zh => "用户未提供完整原作素材；请严格依据创作规格与用户需求写一份可扩展的剧本草稿。".to_string(),
        });

    if language == Language::En {
        ["## Creation Spec",
            &spec,
            "",
            "## Full Source Material",
            &source,
            "",
            "## Output Format",
            &format!("# {}", input.title),
            "",
            "## Script",
            "",
            r#"Follow the target format. Vertical short drama: "Episode N / scene slug / characters / action / dialogue / end-of-episode hook". Standard screenplay: "scene heading / action / character / dialogue"."#]
        .join("\n")
    } else {
        ["## 创作规格",
            &spec,
            "",
            "## 完整原作素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {}", input.title),
            "",
            "## 剧本",
            "",
            r#"按目标格式输出（简体中文）。竖屏短剧：「第N集 / 场次 / 人物 / 动作 / 对白 / 集尾钩子」。标准剧本：「场景标题 / 动作 / 角色 / 对白」"#]
        .join("\n")
    }
}

// ── Storyboard prompts ──────────────────────────────────────

pub(super) fn build_storyboard_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "<identity>",
            "You are a storyboard-creation tool: you break a script, novel excerpt, or concept into shots that can be filmed, drawn, and fed to image generation.",
            "</identity>",
            "",
            "<responsibilities>",
            "- A storyboard is not a plot summary; every shot needs a visual, character placement, action, shot size, or a visual focus.",
            "- Keep the visual spec the user has confirmed; never promote visual constraints the user did not confirm into default requirements.",
            "- Image prompts must be generation-ready: subject, action, setting, lighting, composition, mood, and key props all explicit.",
            "- Output Markdown. No model self-narration or process explanation.",
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER write a shot without a visual, character placement, action, shot size, or visual focus.",
            "- NEVER promote unconfirmed visual preferences into default hard constraints.",
            "- NEVER output an image prompt missing subject, action, setting, lighting, composition, mood, or key props.",
            "- NEVER merge image prompts into the storyboard body or table headers; each must be its own `Prompt: ...` line.",
            "</safety>",
            "",
            "<verification>",
            "Before delivering, self-check:",
            "1. Does every shot carry a visual, character placement, action, shot size, or visual focus?",
            "2. Are all visual constraints you used confirmed by the user (no silent defaults)?",
            "3. Is every image prompt generation-ready (subject, action, setting, lighting, composition, mood, key props)?",
            "4. Is every image prompt on its own `Prompt: ...` line, separate from the storyboard body?",
            "</verification>",
        ]
        .join("\n")
    } else {
        vec![
            "<identity>",
            "你是一个分镜创作工具：把剧本、小说片段或概念拆解为可拍摄、可绘制、可输入图像生成的镜头。请用简体中文输出分镜。",
            "</identity>",
            "",
            "<responsibilities>",
            "- 分镜不是剧情摘要；每个镜头都需要画面、人物站位、动作、景别或视觉焦点。",
            "- 保留用户已确认的视觉规格；永不把用户未确认的视觉约束提升为默认要求。",
            "- 图像提示词必须可直接用于生成：主体、动作、场景、光影、构图、氛围、关键道具全部明确。",
            "- 输出 Markdown 格式。不要模型自述或过程解释。",
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER 写出没有画面、人物站位、动作、景别或视觉焦点的镜头。",
            "- NEVER 把未确认的视觉偏好提升为默认硬约束。",
            "- NEVER 输出缺失主体、动作、场景、光影、构图、氛围或关键道具的图像提示词。",
            "- NEVER 把图像提示词合并进分镜正文或表头；每条必须单独一行 `Prompt: ...`。",
            "</safety>",
            "",
            "<verification>",
            "交付前自检：",
            "1. 每个镜头是否都包含画面、人物站位、动作、景别或视觉焦点？",
            "2. 你使用的视觉约束是否全部由用户确认（无静默默认）？",
            "3. 每条图像提示词是否可直接生成（主体、动作、场景、光影、构图、氛围、关键道具齐全）？",
            "4. 每条图像提示词是否单独占一行 `Prompt: ...`，与分镜正文分离？",
            "</verification>",
        ]
        .join("\n")
    }
}

pub(super) fn build_storyboard_user_prompt(input: &StoryboardCreationInput, language: Language) -> String {
    let spec = render_storyboard_spec(input);
    let max_shots = input.max_shots.unwrap_or(DEFAULT_MAX_SHOTS);
    let source = input
        .source_text
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match language {
            Language::En => "The user did not provide full source material; write an extensible storyboard draft strictly from the storyboard spec and user requirements.".to_string(),
            Language::Zh => "用户未提供完整原作素材；请严格依据分镜规格与用户需求写一份可扩展的分镜草稿。".to_string(),
        });

    if language == Language::En {
        vec![
            "## Storyboard Spec",
            &spec,
            "",
            "## Full Source Material",
            &source,
            "",
            "## Output Format",
            &format!("# {} Storyboard", input.title),
            "",
            "## Storyboard",
            "",
            &format!("Output at most {} shots. Each shot includes: shot number, visual, characters/objects, action, shot size/camera, dialogue/captions, suggested duration, notes.", max_shots),
            "",
            "## Image Prompts",
            "",
            r#"Write one generation-ready image prompt per shot. Each prompt MUST be its own `Prompt: ...` line; never merge it into the storyboard body, table headers, or explanations. Include only the visual constraints the user has confirmed."#,
        ]
        .join("\n")
    } else {
        vec![
            "## 分镜规格",
            &spec,
            "",
            "## 完整原作素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {} 分镜", input.title),
            "",
            "## 分镜",
            "",
            &format!("最多输出 {} 个镜头（简体中文）。每个镜头包含：镜头号、画面、人物/物件、动作、景别/镜头、对白/字幕、建议时长、备注。", max_shots),
            "",
            "## 图像提示词",
            "",
            r#"每个镜头写一条可直接生成的图像提示词。每条提示词必须单独占一行 `Prompt: ...`；永不合并进分镜正文、表头或说明。只包含用户已确认的视觉约束。"#,
        ]
        .join("\n")
    }
}

// ── Interactive film prompts ────────────────────────────────

pub(super) fn build_interactive_film_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "<identity>",
            "You are an interactive-film creation tool: you turn a concept, novel, script, or user brief into an interactive-film deliverable that production can build from.",
            "</identity>",
            "",
            "<responsibilities>",
            "- An interactive film is not an ordinary script: it must have a story tree, key player choices, variables/flags, relationship/evidence/item states, and the conditions for reaching each of the multiple endings.",
            "- The variable system exists only to drive plot progression and branch unlocking; no default RPG stats, combat formulas, or equipment tiers. Write such rules only when the user explicitly asks for them.",
            r#"Output must be Markdown with the specified sections. No model self-narration, process notes, or "Here is" preamble."#,
            r#"Every storyboard image prompt must be its own standalone `Prompt: ...` line so downstream asset management can pick it up; include only the visual constraints the user has confirmed."#,
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER deliver an interactive film without a story tree, key choices, variables/flags, and multi-ending conditions.",
            "- NEVER impose default RPG stats, combat formulas, or equipment tiers unless the user explicitly asks.",
            "- NEVER merge image prompts into the storyboard body; each must be its own `Prompt: ...` line.",
            "- NEVER decide subject matter, budget, art style, or commercial punch-up intensity on the user's behalf.",
            "- NEVER add model self-narration, process notes, or \"Here is\" preamble.",
            "</safety>",
            "",
            "<verification>",
            "Before delivering, self-check:",
            "1. Does the deliverable contain a story tree, key choices, variables/flags, and the conditions for each ending?",
            "2. Is the variable system described in natural language (no forced numeric stats or equipment tiers)?",
            "3. Is every image prompt on its own `Prompt: ...` line, with only user-confirmed visual constraints?",
            "4. Is the output pure Markdown with the specified sections, no model self-narration or preamble?",
            "</verification>",
        ]
        .join("\n")
    } else {
        vec![
            "<identity>",
            "你是一个互动影游创作工具：把概念、小说、剧本或用户简报转化为制片方可继续构建的互动影游交付物。请用简体中文输出交付物。",
            "</identity>",
            "",
            "<responsibilities>",
            "- 互动影游不是普通剧本：必须包含故事树、关键玩家选择、变量/标记、关系/证据/物品状态，以及达成多结局的条件。",
            "- 变量系统只为推动剧情推进与分支解锁而存在；不默认 RPG 属性、战斗公式或装备等级。仅当用户明确要求时才写此类规则。",
            r#"输出必须为 Markdown，包含指定章节。不要模型自述、过程说明或「以下是」类前言。"#,
            r#"每条分镜图像提示词必须单独占一行 `Prompt: ...`，以便下游资产管理抓取；只包含用户已确认的视觉约束。"#,
            "</responsibilities>",
            "",
            "<safety>",
            "- NEVER 交付缺少故事树、关键选择、变量/标记或多结局条件的互动影游。",
            "- NEVER 强加默认 RPG 属性、战斗公式或装备等级，除非用户明确要求。",
            "- NEVER 把图像提示词合并进分镜正文；每条必须单独一行 `Prompt: ...`。",
            "- NEVER 替用户决定题材、预算、美术风格或商业强化强度。",
            "- NEVER 添加模型自述、过程说明或「以下是」类前言。",
            "</safety>",
            "",
            "<verification>",
            "交付前自检：",
            "1. 交付物是否包含故事树、关键选择、变量/标记以及每个结局的达成条件？",
            "2. 变量系统是否用自然语言描述（无强加数值属性或装备等级）？",
            "3. 每条图像提示词是否单独占一行 `Prompt: ...`，且只含用户已确认的视觉约束？",
            "4. 输出是否为纯 Markdown，包含指定章节，无模型自述或前言？",
            "</verification>",
        ]
        .join("\n")
    }
}

pub(super) fn build_interactive_film_user_prompt(input: &InteractiveFilmCreationInput, language: Language) -> String {
    let spec = render_interactive_film_spec(input);
    let source = input
        .source_text
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match language {
            Language::En => "The user did not provide full source material; write an extensible interactive-film deliverable strictly from the creation spec and user requirements.".to_string(),
            Language::Zh => "用户未提供完整原作素材；请严格依据创作规格与用户需求写一份可扩展的互动影游交付物。".to_string(),
        });

    if language == Language::En {
        vec![
            "## Interactive Film Spec",
            &spec,
            "",
            "## Full Source Material",
            &source,
            "",
            "## Output Format",
            &format!("# {} Interactive Film Package", input.title),
            "",
            "## Story Tree",
            "Lay out main-line nodes, branch nodes, key choices, and merge/no-return relationships as Markdown. The multi-ending structure must be visible at a glance.",
            "",
            "## Variables and Flags",
            "List each variable/flag: name, meaning, trigger, scope of impact, and related nodes. Variables may be relationships, states, evidence, items, identities, secret/public status, ending gates, and so on.",
            "",
            "## Ending Paths",
            "For every ending: its unlock conditions, the key choice chain, the required variables/flags, plus any failure or hidden-ending conditions.",
            "",
            "## Interactive Script",
            "Write a playable script per node: scene, characters, action, dialogue, player choices, variable changes, and branch destinations. Never write summaries only.",
            "",
            "## Storyboard and Image Prompts",
            "List the key shots. Each shot includes visual, characters/objects, action, shot size, and suggested duration. After each shot, add exactly one standalone `Prompt: ...` line.",
        ]
        .join("\n")
    } else {
        vec![
            "## 互动影游规格",
            &spec,
            "",
            "## 完整原作素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {} 互动影游套餐", input.title),
            "",
            "## 故事树",
            "用 Markdown 列出主线节点、分支节点、关键选择、合并/不可返回关系。多结局结构必须一目了然。",
            "",
            "## 变量与标记",
            "列出每个变量/标记：名称、含义、触发条件、影响范围、关联节点。变量可以是关系、状态、证据、物品、身份、秘密/公开属性、结局门槛等。",
            "",
            "## 结局路径",
            "为每个结局写明：解锁条件、关键选择链、所需变量/标记，以及失败或隐藏结局条件。",
            "",
            "## 互动剧本",
            "为每个节点写可玩剧本：场景、人物、动作、对白、玩家选择、变量变化、分支去向。永不只写摘要。",
            "",
            "## 分镜与图像提示词",
            "列出关键镜头。每个镜头包含画面、人物/物件、动作、景别、建议时长。每个镜头后紧跟一行单独的 `Prompt: ...`。",
        ]
        .join("\n")
    }
}
