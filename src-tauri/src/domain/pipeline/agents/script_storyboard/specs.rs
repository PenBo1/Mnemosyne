//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Specs - 规格渲染
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：剧本 / 分镜 / 互动影游创作规格渲染（public，runner 使用）。

use crate::domain::pipeline::types::Language;

use super::types::{
    InteractiveFilmCreationInput, ScriptCreationInput, ScriptTargetFormat,
    StoryboardCreationInput,
};

/// 渲染剧本创作规格。
pub fn render_script_spec(input: &ScriptCreationInput) -> String {
    let language = input.language.unwrap_or_default();

    if language == Language::En {
        let episode_count_line = match input.episode_count {
            Some(n) => format!("- Episode/segment count: {}", n),
            None => "- Episode/segment count: unspecified; judge from the source material and user requirements".to_string(),
        };
        let episode_duration_line = match input.episode_duration.as_deref() {
            Some(d) => format!("- Per-episode/segment duration: {}", d),
            None => "- Per-episode/segment duration: unspecified".to_string(),
        };
        let source_kind_line = match input.source_kind.as_deref() {
            Some(k) => format!("- Source material: {}", k),
            None => "- Source material: user input / conversation brief".to_string(),
        };
        vec![
            format!("# {} Script Creation Spec", input.title),
            String::new(),
            "## Goal".to_string(),
            format!("- Deliverable: {}", format_script_target(input.target_format, language)),
            episode_count_line,
            episode_duration_line,
            source_kind_line,
            String::new(),
            "## User Requirements".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "Not separately specified; follow the instruction the user confirmed.".to_string()),
            String::new(),
            "## Adaptation Boundaries".to_string(),
            "- Preserve the characters, relationships, conflicts, key events, and taboos the user explicitly specified.".to_string(),
            "- Never decide adaptation intensity (\"faithful adaptation / commercial punch-up / low-budget shoot\") on the user's behalf; execute only the spec the user has confirmed.".to_string(),
            "- If the source material is a novel, convert interiority into playable action, dialogue, evidence, objects, or on-screen consequences.".to_string(),
            "- If the target is a short drama, every episode needs visible conflict and an end-of-episode reason to keep watching.".to_string(),
            String::new(),
            "## Source Material Summary".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    } else {
        let episode_count_line = match input.episode_count {
            Some(n) => format!("- 集/段数：{}", n),
            None => "- 集/段数：未指定；根据原作素材与用户需求判断".to_string(),
        };
        let episode_duration_line = match input.episode_duration.as_deref() {
            Some(d) => format!("- 单集/段时长：{}", d),
            None => "- 单集/段时长：未指定".to_string(),
        };
        let source_kind_line = match input.source_kind.as_deref() {
            Some(k) => format!("- 原作素材：{}", k),
            None => "- 原作素材：用户输入 / 对话简报".to_string(),
        };
        vec![
            format!("# {} 剧本创作规格", input.title),
            String::new(),
            "## 目标".to_string(),
            format!("- 交付物：{}", format_script_target(input.target_format, language)),
            episode_count_line,
            episode_duration_line,
            source_kind_line,
            String::new(),
            "## 用户需求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独说明；按用户已确认的指令执行。".to_string()),
            String::new(),
            "## 改编边界".to_string(),
            "- 保留用户明确指定的人物、关系、冲突、关键事件与禁忌。".to_string(),
            "- 永不替用户决定改编强度（「忠实改编 / 商业强化 / 低成本拍摄」）；只执行用户已确认的规格。".to_string(),
            "- 若原作素材是小说，将内心活动转化为可表演的动作、对白、证据、物件或画面结果。".to_string(),
            "- 若目标是短剧，每集都需要可见的冲突与集尾留人钩子。".to_string(),
            String::new(),
            "## 原作素材摘要".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    }
}

/// 渲染分镜创作规格。
pub fn render_storyboard_spec(input: &StoryboardCreationInput) -> String {
    let language = input.language.unwrap_or_default();

    if language == Language::En {
        let granularity_line = match input.granularity.as_deref() {
            Some(g) if !g.trim().is_empty() => g.trim().to_string(),
            _ => "split by scene and key shots".to_string(),
        };
        let aspect_ratio_line = match input.aspect_ratio.as_deref() {
            Some(a) if !a.trim().is_empty() => a.trim().to_string(),
            _ => "unspecified; default to what the user's material and target imply".to_string(),
        };
        let visual_style_line = match input.visual_style.as_deref() {
            Some(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => "unspecified; judge from the user's material and target platform".to_string(),
        };
        vec![
            format!("# {} Storyboard Creation Spec", input.title),
            String::new(),
            "## Goal".to_string(),
            format!("- Shot granularity: {}", granularity_line),
            format!("- Aspect ratio: {}", aspect_ratio_line),
            format!("- Visual style: {}", visual_style_line),
            match input.max_shots {
                Some(n) => format!("- Shot cap: {}", n),
                None => "- Shot cap: unspecified".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- Source material: {}", k),
                None => "- Source material: user input / conversation brief".to_string(),
            },
            String::new(),
            "## User Requirements".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "Not separately specified; follow the instruction the user confirmed.".to_string()),
            String::new(),
            "## Storyboard Boundaries".to_string(),
            "- A storyboard is a creative tool, not a locked-in shooting plan; the output must stay easy to discuss, extend, trim, and re-shoot.".to_string(),
            "- Each shot carries only what the frame can show, an actor can play, and a camera can express.".to_string(),
            "- Image prompts serve image generation: subject, action, shot size, setting, lighting, mood, and key props must be explicit.".to_string(),
            "- Follow only the art style, format, composition, and visual constraints the user has confirmed; never turn unstated preferences into default hard constraints.".to_string(),
            String::new(),
            "## Source Material Summary".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    } else {
        let granularity_line = match input.granularity.as_deref() {
            Some(g) if !g.trim().is_empty() => g.trim().to_string(),
            _ => "按场景与关键镜头切分".to_string(),
        };
        let aspect_ratio_line = match input.aspect_ratio.as_deref() {
            Some(a) if !a.trim().is_empty() => a.trim().to_string(),
            _ => "未指定；按用户素材与目标默认推断".to_string(),
        };
        let visual_style_line = match input.visual_style.as_deref() {
            Some(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => "未指定；按用户素材与目标平台判断".to_string(),
        };
        vec![
            format!("# {} 分镜创作规格", input.title),
            String::new(),
            "## 目标".to_string(),
            format!("- 镜头粒度：{}", granularity_line),
            format!("- 画幅比例：{}", aspect_ratio_line),
            format!("- 视觉风格：{}", visual_style_line),
            match input.max_shots {
                Some(n) => format!("- 镜头上限：{}", n),
                None => "- 镜头上限：未指定".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- 原作素材：{}", k),
                None => "- 原作素材：用户输入 / 对话简报".to_string(),
            },
            String::new(),
            "## 用户需求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独说明；按用户已确认的指令执行。".to_string()),
            String::new(),
            "## 分镜边界".to_string(),
            "- 分镜是创作工具，不是锁定的拍摄计划；输出必须便于讨论、扩展、删减与重拍。".to_string(),
            "- 每个镜头只承载画面能呈现、演员能表演、镜头能表达的内容。".to_string(),
            "- 图像提示词服务于图像生成：主体、动作、景别、场景、光影、氛围与关键道具必须明确。".to_string(),
            "- 只遵循用户已确认的美术风格、格式、构图与视觉约束；永不把未说明的偏好默认为硬约束。".to_string(),
            String::new(),
            "## 原作素材摘要".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    }
}

/// 渲染互动影游创作规格。
pub fn render_interactive_film_spec(input: &InteractiveFilmCreationInput) -> String {
    let language = input.language.unwrap_or_default();

    if language == Language::En {
        vec![
            format!("# {} Interactive Film Creation Spec", input.title),
            String::new(),
            "## Goal".to_string(),
            "- Deliverable: interactive film / interactive narrative game / film-game script".to_string(),
            match input.episode_count {
                Some(n) => format!("- Story segments/episodes: {}", n),
                None => "- Story segments/episodes: unspecified; judge from the source material and user requirements".to_string(),
            },
            match input.episode_duration.as_deref() {
                Some(d) => format!("- Per-segment/episode duration: {}", d),
                None => "- Per-segment/episode duration: unspecified".to_string(),
            },
            match input.budget.as_deref() {
                Some(b) => format!("- Budget constraint: {}", b),
                None => "- Budget constraint: unspecified".to_string(),
            },
            match input.target_audience.as_deref() {
                Some(a) => format!("- Target audience: {}", a),
                None => "- Target audience: unspecified".to_string(),
            },
            match input.reference_mode.as_deref() {
                Some(r) => format!("- Reference mode: {}", r),
                None => "- Reference mode: unspecified by the user; do not impose a fixed game template".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- Source material: {}", k),
                None => "- Source material: user input / conversation brief".to_string(),
            },
            String::new(),
            "## User Requirements".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "Not separately specified; follow the instruction the user confirmed.".to_string()),
            String::new(),
            "## Interactive Film Boundaries".to_string(),
            "- This is a creative deliverable, not a hard-numbers RPG engine design; variables, flags, relationships, and ending conditions must serve story branching.".to_string(),
            "- It must include branching storylines, key player choices, how variables/flags change later plot, and the conditions for reaching each of the multiple endings.".to_string(),
            "- Describe the variable system in natural language: states, relationships, secret/public status, evidence, items, identities, affinity/trust, and the like; never force fixed numeric stats or equipment tiers.".to_string(),
            "- The deliverable must fit interactive film/drama production: a clear story tree, shootable nodes, playable dialogue, drawable storyboards, and image prompts usable for asset generation.".to_string(),
            "- Never decide subject matter, budget, art style, or commercial punch-up intensity on the user's behalf; mark anything unspecified as adjustable.".to_string(),
            String::new(),
            "## Source Material Summary".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    } else {
        vec![
            format!("# {} 互动影游创作规格", input.title),
            String::new(),
            "## 目标".to_string(),
            "- 交付物：互动影游 / 互动叙事游戏 / 影游剧本".to_string(),
            match input.episode_count {
                Some(n) => format!("- 故事段落/集数：{}", n),
                None => "- 故事段落/集数：未指定；根据原作素材与用户需求判断".to_string(),
            },
            match input.episode_duration.as_deref() {
                Some(d) => format!("- 单段/集时长：{}", d),
                None => "- 单段/集时长：未指定".to_string(),
            },
            match input.budget.as_deref() {
                Some(b) => format!("- 预算约束：{}", b),
                None => "- 预算约束：未指定".to_string(),
            },
            match input.target_audience.as_deref() {
                Some(a) => format!("- 目标受众：{}", a),
                None => "- 目标受众：未指定".to_string(),
            },
            match input.reference_mode.as_deref() {
                Some(r) => format!("- 参考模式：{}", r),
                None => "- 参考模式：用户未指定；不要强加固定游戏模板".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- 原作素材：{}", k),
                None => "- 原作素材：用户输入 / 对话简报".to_string(),
            },
            String::new(),
            "## 用户需求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独说明；按用户已确认的指令执行。".to_string()),
            String::new(),
            "## 互动影游边界".to_string(),
            "- 这是创作交付物，不是硬数值 RPG 引擎设计；变量、标记、关系与结局条件必须服务于剧情分支。".to_string(),
            "- 必须包含分支剧情、关键玩家选择、变量/标记如何影响后续剧情，以及达成多结局的条件。".to_string(),
            "- 用自然语言描述变量系统：状态、关系、秘密/公开属性、证据、物品、身份、好感/信任等；永不强加固定数值属性或装备等级。".to_string(),
            "- 交付物必须适合互动影游制作：清晰的故事树、可拍摄节点、可玩对白、可绘分镜、可用于资产生成的图像提示词。".to_string(),
            "- 永不替用户决定题材、预算、美术风格或商业强化强度；未说明项标注为可调整。".to_string(),
            String::new(),
            "## 原作素材摘要".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    }
}

// ── 内部辅助函数（private） ─────────────────────────────────

fn format_script_target(value: Option<ScriptTargetFormat>, language: Language) -> String {
    let format = value.unwrap_or_default();
    match (format, language) {
        (ScriptTargetFormat::VerticalShortDrama, Language::En) => "vertical short drama",
        (ScriptTargetFormat::VerticalShortDrama, Language::Zh) => "竖屏短剧",
        (ScriptTargetFormat::Screenplay, Language::En) => "standard screenplay",
        (ScriptTargetFormat::Screenplay, Language::Zh) => "标准剧本",
        (ScriptTargetFormat::AudioDrama, Language::En) => "audio drama",
        (ScriptTargetFormat::AudioDrama, Language::Zh) => "广播剧",
        (ScriptTargetFormat::InteractiveScript, Language::En) => "interactive script",
        (ScriptTargetFormat::InteractiveScript, Language::Zh) => "互动剧本",
        (ScriptTargetFormat::GeneralScript, Language::En) => "general script",
        (ScriptTargetFormat::GeneralScript, Language::Zh) => "通用剧本",
    }
    .to_string()
}

fn summarize_source_for_spec(source_text: &Option<String>, language: Language) -> String {
    let text = source_text
        .as_deref()
        .map(|s| regex::Regex::new(r"\s+").map(|re| re.replace_all(s, " ").to_string()).unwrap_or_else(|_| s.to_string()))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if text.is_empty() {
        match language {
            Language::En => "No full source material provided.".to_string(),
            Language::Zh => "未提供完整原作素材。".to_string(),
        }
    } else {
        match language {
            Language::En => format!("Full source material provided, about {} characters; the full content will be read during generation.", text.chars().count()),
            Language::Zh => format!("已提供完整原作素材，约 {} 字；生成时将完整阅读。", text.chars().count()),
        }
    }
}

// ── 单元测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── format_script_target ──

    #[test]
    fn format_script_target_defaults_to_general() {
        assert_eq!(format_script_target(None, Language::Zh), "通用剧本");
        assert_eq!(format_script_target(None, Language::En), "general script");
    }

    #[test]
    fn format_script_target_translates_vertical_drama() {
        assert_eq!(
            format_script_target(Some(ScriptTargetFormat::VerticalShortDrama), Language::Zh),
            "竖屏短剧"
        );
        assert_eq!(
            format_script_target(Some(ScriptTargetFormat::VerticalShortDrama), Language::En),
            "vertical short drama"
        );
    }

    // ── render_script_spec ──

    #[test]
    fn render_script_spec_zh_includes_title() {
        let input = ScriptCreationInput {
            title: "测试剧本".to_string(),
            source_kind: None,
            target_format: None,
            source_text: None,
            requirements: None,
            episode_count: Some(6),
            episode_duration: None,
            language: Some(Language::Zh),
        };
        let spec = render_script_spec(&input);
        assert!(spec.contains("测试剧本"));
        assert!(spec.contains("集/段数：6"));
        assert!(spec.contains("通用剧本"));
    }

    #[test]
    fn render_script_spec_en_includes_unspecified() {
        let input = ScriptCreationInput {
            title: "Test Script".to_string(),
            source_kind: None,
            target_format: None,
            source_text: None,
            requirements: None,
            episode_count: None,
            episode_duration: None,
            language: Some(Language::En),
        };
        let spec = render_script_spec(&input);
        assert!(spec.contains("Test Script"));
        assert!(spec.contains("unspecified"));
    }
}
