// Script/Storyboard/InteractiveFilm Agents。
//
// 职责：3 个独立 agent（剧本创作 / 分镜创作 / 互动影游创作）+ 规格渲染 + 提示词抽取。
//
// 约束：AgentEngine.prompt_once 只支持单轮对话，TS 中的多轮对话合并为单轮。
// temp/maxTokens 参数无法透传（prompt_once 不支持），仅在注释中标注原始值。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::types::Language;

// ── 常量 ─────────────────────────────────────────────────────

const DEFAULT_MAX_SHOTS: u32 = 24;

// ── ScriptTargetFormat ──────────────────────────────────────

/// 剧本目标格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptTargetFormat {
    VerticalShortDrama,
    Screenplay,
    AudioDrama,
    InteractiveScript,
    GeneralScript,
}

impl Default for ScriptTargetFormat {
    fn default() -> Self {
        Self::GeneralScript
    }
}

// ── 输入结构 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScriptCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub target_format: Option<ScriptTargetFormat>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub language: Option<Language>,
}

#[derive(Debug, Clone)]
pub struct StoryboardCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub visual_style: Option<String>,
    pub aspect_ratio: Option<String>,
    pub granularity: Option<String>,
    pub max_shots: Option<u32>,
    pub language: Option<Language>,
}

#[derive(Debug, Clone)]
pub struct InteractiveFilmCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub target_audience: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub budget: Option<String>,
    pub reference_mode: Option<String>,
    pub language: Option<Language>,
}

// ═══════════════════════════════════════════════════════════════
//  Agent 执行函数
// ═══════════════════════════════════════════════════════════════

/// 1. 剧本创作（temp=0.55, maxTokens=min(32000, max(12000, episodes*2200))）。
pub async fn write_script(
    engine: &AgentEngine,
    input: &ScriptCreationInput,
) -> Result<String, AppError> {
    let language = input.language.unwrap_or_default();
    let system_prompt = build_script_system_prompt(language);
    let user_message = build_script_user_prompt(input, language);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(normalize_episode_end_labels(response.trim(), 1))
}

/// 2. 分镜创作（temp=0.45, maxTokens=min(24000, max(10000, shots*700))）。
pub async fn write_storyboard(
    engine: &AgentEngine,
    input: &StoryboardCreationInput,
) -> Result<String, AppError> {
    let language = input.language.unwrap_or_default();
    let system_prompt = build_storyboard_system_prompt(language);
    let user_message = build_storyboard_user_prompt(input, language);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

/// 3. 互动影游创作（temp=0.5, maxTokens=min(36000, max(16000, episodes*3000))）。
pub async fn write_interactive_film(
    engine: &AgentEngine,
    input: &InteractiveFilmCreationInput,
) -> Result<String, AppError> {
    let language = input.language.unwrap_or_default();
    let system_prompt = build_interactive_film_system_prompt(language);
    let user_message = build_interactive_film_user_prompt(input, language);
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    Ok(response.trim().to_string())
}

// ═══════════════════════════════════════════════════════════════
//  规格渲染（public，runner 使用）
// ═══════════════════════════════════════════════════════════════

/// 渲染剧本创作规格。
pub fn render_script_spec(input: &ScriptCreationInput) -> String {
    let language = input.language.unwrap_or_default();
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

    if language == Language::En {
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
        vec![
            format!("# {} 剧本创作规格", input.title),
            String::new(),
            "## 目标".to_string(),
            format!("- 交付类型：{}", format_script_target(input.target_format, language)),
            match input.episode_count {
                Some(n) => format!("- 集数/段落数：{}", n),
                None => "- 集数/段落数：未指定，按素材和用户要求判断".to_string(),
            },
            match input.episode_duration.as_deref() {
                Some(d) => format!("- 单集/单段时长：{}", d),
                None => "- 单集/单段时长：未指定".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- 原素材：{}", k),
                None => "- 原素材：用户输入/对话需求".to_string(),
            },
            String::new(),
            "## 用户要求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独指定；以用户确认时的 instruction 为准。".to_string()),
            String::new(),
            "## 改编边界".to_string(),
            "- 优先保留用户明确指定的人物、关系、冲突、关键事件和禁忌。".to_string(),
            "- 不替用户擅自决定“忠实改编 / 商业强化 / 低成本拍摄”等强度；只执行用户已确认的规格。".to_string(),
            "- 如果原素材是小说，内心戏要转成可演的动作、对白、证据、物件或场面后果。".to_string(),
            "- 如果目标是短剧，每集必须有可见冲突和集尾继续看的理由。".to_string(),
            String::new(),
            "## 源素材摘要".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    }
}

/// 渲染分镜创作规格。
pub fn render_storyboard_spec(input: &StoryboardCreationInput) -> String {
    let language = input.language.unwrap_or_default();
    let granularity_line = match input.granularity.as_deref() {
        Some(g) if !g.trim().is_empty() => g.trim().to_string(),
        _ => match language {
            Language::En => "split by scene and key shots".to_string(),
            Language::Zh => "按场景和关键镜头拆分".to_string(),
        },
    };
    let aspect_ratio_line = match input.aspect_ratio.as_deref() {
        Some(a) if !a.trim().is_empty() => a.trim().to_string(),
        _ => match language {
            Language::En => "unspecified; default to what the user's material and target imply".to_string(),
            Language::Zh => "未指定，默认按用户素材目标判断".to_string(),
        },
    };
    let visual_style_line = match input.visual_style.as_deref() {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => match language {
            Language::En => "unspecified; judge from the user's material and target platform".to_string(),
            Language::Zh => "未指定，按用户素材和目标平台判断".to_string(),
        },
    };

    if language == Language::En {
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
        vec![
            format!("# {} 分镜创作规格", input.title),
            String::new(),
            "## 目标".to_string(),
            format!("- 分镜粒度：{}", granularity_line),
            format!("- 画幅：{}", aspect_ratio_line),
            format!("- 视觉风格：{}", visual_style_line),
            match input.max_shots {
                Some(n) => format!("- 镜头上限：{}", n),
                None => "- 镜头上限：未指定".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- 原素材：{}", k),
                None => "- 原素材：用户输入/对话需求".to_string(),
            },
            String::new(),
            "## 用户要求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独指定；以用户确认时的 instruction 为准。".to_string()),
            String::new(),
            "## 分镜边界".to_string(),
            "- 分镜是创作工具，不替用户锁死最终拍法；输出要便于继续讨论、增删、改镜头。".to_string(),
            "- 每个镜头只写画面能看见、角色能演、镜头能表达的信息。".to_string(),
            "- 分镜图提示词服务图像生成：角色、动作、景别、场景、光线、情绪和关键道具要清楚。".to_string(),
            "- 只遵循用户已确认的画风、格式、构图和视觉限制；用户没说的，不写成默认硬限制。".to_string(),
            String::new(),
            "## 源素材摘要".to_string(),
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
            "- 交付类型：互动影游 / 互动叙事类游戏 / 影游剧本".to_string(),
            match input.episode_count {
                Some(n) => format!("- 剧情段落/集数：{}", n),
                None => "- 剧情段落/集数：未指定，按素材和用户要求判断".to_string(),
            },
            match input.episode_duration.as_deref() {
                Some(d) => format!("- 单段/单集时长：{}", d),
                None => "- 单段/单集时长：未指定".to_string(),
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
                None => "- 参考模式：用户未指定，不擅自套固定游戏模板".to_string(),
            },
            match input.source_kind.as_deref() {
                Some(k) => format!("- 原素材：{}", k),
                None => "- 原素材：用户输入/对话需求".to_string(),
            },
            String::new(),
            "## 用户要求".to_string(),
            input.requirements.as_deref().map(|r| r.trim()).filter(|r| !r.is_empty())
                .map(|r| r.to_string())
                .unwrap_or_else(|| "未单独指定；以用户确认时的 instruction 为准。".to_string()),
            String::new(),
            "## 互动影游边界".to_string(),
            "- 这是创作交付稿，不是硬数值 RPG 引擎设计；变量、旗标、关系和结局条件必须服务剧情分支。".to_string(),
            "- 必须包含多分支剧情、玩家关键选择、变量/旗标如何改变后续剧情，以及多结局达成条件。".to_string(),
            "- 变量系统用自然语言说明即可：状态、关系、隐瞒/公开、证据、物品、身份、好感/信任等；不要强行套固定数值或装备等级。".to_string(),
            "- 交付要适配影游/互动剧制作：剧情树清晰、节点可拍、对白可演、分镜可画、图片提示词可用于资产生成。".to_string(),
            "- 不替用户擅自决定题材、预算、画风和商业强化强度；未指定处写为可调整。".to_string(),
            String::new(),
            "## 源素材摘要".to_string(),
            summarize_source_for_spec(&input.source_text, language),
        ].join("\n")
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数（public）
// ═══════════════════════════════════════════════════════════════

/// 从分镜 Markdown 中抽取图像提示词（合并抽取与解析两步）。
///
/// 先尝试定位"图像提示词"小节，找不到则用全文。支持：
/// - Markdown 表格中的 Prompt 列
/// - `Prompt: ...` / `提示词: ...` 行
/// - 编号列表行 `1. ...`
pub fn extract_image_prompts(markdown: &str) -> Vec<String> {
    let section = extract_image_prompt_section(markdown);
    let section_trimmed = section.trim();
    let source = if section_trimmed.is_empty() {
        markdown.trim()
    } else {
        section_trimmed
    };
    parse_prompt_lines(source)
}

/// 抽取 Markdown 小节（单标题版本）。
///
/// 匹配规则：标题文本归一化（小写、去 markdown 标记）后，
/// 若 text == heading 或 text 以 heading 开头且剩余部分为空或以分隔符开头，则匹配。
pub fn extract_markdown_section(content: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let normalized_heading = normalize_heading_text(heading);

    let mut start: i64 = -1;
    let mut level: usize = 0;

    let heading_re = regex::Regex::new(r"^(#{1,6})\s*(.+?)\s*$").ok()?;

    for (index, line) in lines.iter().enumerate() {
        if let Some(caps) = heading_re.captures(line) {
            let text = normalize_heading_text(&caps[2]);
            if heading_matches(&text, &normalized_heading) {
                start = index as i64 + 1;
                level = caps[1].len();
                break;
            }
        }
    }

    if start < 0 {
        return None;
    }

    let start_idx = start as usize;
    let mut end = lines.len();
    for (index, line) in lines.iter().enumerate().skip(start_idx) {
        if let Some(caps) = heading_re.captures(line) {
            if caps[1].len() <= level {
                end = index;
                break;
            }
        }
    }

    Some(lines[start_idx..end].join("\n"))
}

/// 规范化剧本集尾标签。
///
/// 跟踪 `## 第N集` 标题，将 "字幕：第X集完" 替换为 "字幕：第{current}集完"。
/// current_episode 作为初始值（在未遇到任何集标题前使用）。
pub fn normalize_episode_end_labels(markdown: &str, current_episode: u32) -> String {
    let heading_re = regex::Regex::new(
        r"^#{1,6}\s*第\s*([一二三四五六七八九十百千万\d]+)\s*集(?:\s|$)",
    )
    .expect("invalid episode heading regex");

    let replace_re = regex::Regex::new(
        r"(字幕\s*[：:]\s*)第\s*[一二三四五六七八九十百千万\d]+\s*集完",
    )
    .expect("invalid episode end label regex");

    let mut current = current_episode.to_string();
    markdown
        .lines()
        .map(|line| {
            if let Some(caps) = heading_re.captures(line.trim()) {
                current = caps[1].to_string();
            }
            replace_re
                .replace_all(line, format!("${{1}}第{}集完", current))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 估算 max_tokens。
pub fn estimate_max_tokens(episodes: u32, per_episode: u32, min: u64, max: u64) -> u64 {
    std::cmp::min(max, std::cmp::max(min, episodes as u64 * per_episode as u64))
}

// ═══════════════════════════════════════════════════════════════
//  Prompt 构造函数（private）
// ═══════════════════════════════════════════════════════════════

fn build_script_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are a script-creation tool, not a novel-continuation engine.",
            "Your job is to adapt a novel, concept, outline, or existing text into a script that production can keep working from, following the spec the user has confirmed.",
            "Never decide adaptation intensity on the user's behalf; execute only the goals, format, boundaries, and constraints already confirmed in the spec.",
            "Action lines carry only what the audience can see, an actor can play, and a camera can shoot; convert interiority into behavior, dialogue, objects, evidence, or on-screen consequences.",
            "Dialogue must serve conflict, relationships, information flow, or emotional shifts; no hollow exposition.",
            r#"Output Markdown. No process notes, no model self-narration, no "Here is" preamble."#,
        ]
        .join("\n")
    } else {
        vec![
            "你是剧本创作工具，不是小说续写器。",
            "你的任务是根据用户确认过的规格，把小说、创意、大纲或已有文本改成可继续制作的剧本。",
            "不要替用户擅自决定改编强度；只执行规格里已经确认的目标、格式、边界和限制。",
            "动作行只写观众能看见、演员能演、镜头能拍的信息；内心戏要转成行为、对白、物件、证据或场面后果。",
            "对白要服务冲突、关系、信息推进或情绪变化，不写空泛解释。",
            r#"输出 Markdown。不要写流程说明、模型自述或"以下是"。"#,
        ]
        .join("\n")
    }
}

fn build_script_user_prompt(input: &ScriptCreationInput, language: Language) -> String {
    let spec = render_script_spec(input);
    let source = input
        .source_text
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match language {
            Language::En => "The user did not provide full source material; write an extensible script draft strictly from the creation spec and user requirements.".to_string(),
            Language::Zh => "用户没有提供完整源素材；请严格根据创作规格和用户要求写一个可继续扩展的剧本稿。".to_string(),
        });

    if language == Language::En {
        vec![
            "## Creation Spec",
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
            r#"Follow the target format. Vertical short drama: "Episode N / scene slug / characters / action / dialogue / end-of-episode hook". Standard screenplay: "scene heading / action / character / dialogue"."#,
        ]
        .join("\n")
    } else {
        vec![
            "## 创作规格",
            &spec,
            "",
            "## 完整源素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {}", input.title),
            "",
            "## 剧本正文",
            "",
            r#"按目标格式输出。竖屏短剧使用"第N集 / 场次 / 人物 / 动作 / 对白 / 集尾钩子"；标准剧本使用"场景标题 / 动作 / 角色 / 对白"。"#,
        ]
        .join("\n")
    }
}

fn build_storyboard_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are a storyboard-creation tool: you break a script, novel excerpt, or concept into shots that can be filmed, drawn, and fed to image generation.",
            "A storyboard is not a plot summary; every shot needs a visual, character placement, action, shot size, or a visual focus.",
            "Keep the visual spec the user has confirmed; never promote visual constraints the user did not confirm into default requirements.",
            "Image prompts must be generation-ready: subject, action, setting, lighting, composition, mood, and key props all explicit.",
            "Output Markdown. No model self-narration or process explanation.",
        ]
        .join("\n")
    } else {
        vec![
            "你是分镜创作工具，负责把剧本、小说片段或创意拆成可拍、可画、可生图的分镜。",
            "分镜不是剧情摘要；每个镜头都要有画面、角色位置、动作、景别或视觉重点。",
            "保留用户确认的视觉规格；不要把用户没有确认的视觉限制写成默认要求。",
            "图像提示词要便于生图：主体、动作、场景、光线、构图、情绪、关键道具明确。",
            "输出 Markdown。不要写模型自述或流程解释。",
        ]
        .join("\n")
    }
}

fn build_storyboard_user_prompt(input: &StoryboardCreationInput, language: Language) -> String {
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
            Language::Zh => "用户没有提供完整源素材；请严格根据分镜规格和用户要求写一个可继续扩展的分镜稿。".to_string(),
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
            "## 完整源素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {} 分镜", input.title),
            "",
            "## 分镜表",
            "",
            &format!("输出不超过 {} 个镜头。每个镜头包含：镜号、画面、人物/物件、动作、景别/机位、对白/字幕、时长建议、备注。", max_shots),
            "",
            "## 图像提示词",
            "",
            "为每个镜头写一条可用于生图的提示词。每条必须单独写成 `Prompt: ...`，不要混入分镜正文、表头或解释；只写用户确认过的视觉限制。",
        ]
        .join("\n")
    }
}

fn build_interactive_film_system_prompt(language: Language) -> String {
    if language == Language::En {
        vec![
            "You are an interactive-film creation tool: you turn a concept, novel, script, or user brief into an interactive-film deliverable that production can build from.",
            "An interactive film is not an ordinary script: it must have a story tree, key player choices, variables/flags, relationship/evidence/item states, and the conditions for reaching each of the multiple endings.",
            "The variable system exists only to drive plot progression and branch unlocking; no default RPG stats, combat formulas, or equipment tiers. Write such rules only when the user explicitly asks for them.",
            r#"Output must be Markdown with the specified sections. No model self-narration, process notes, or "Here is" preamble."#,
            r#"Every storyboard image prompt must be its own standalone `Prompt: ...` line so downstream asset management can pick it up; include only the visual constraints the user has confirmed."#,
        ]
        .join("\n")
    } else {
        vec![
            "你是互动影游创作工具，负责把创意、小说、剧本或用户需求整理成可制作的互动影游交付稿。",
            "互动影游不是普通剧本：必须有剧情树、关键选择、变量/旗标、关系/证据/物品状态、多结局达成条件。",
            "变量系统只服务剧情推进和分支解锁，不要默认 RPG 数值、战斗公式或装备等级；只有用户明确要求时才写对应规则。",
            r#"输出必须是 Markdown，包含指定小节。不要写模型自述、流程说明或"以下是"。"#,
            "分镜图提示词必须写成单独的 `Prompt: ...` 行，便于后续资产管理；只写用户确认过的视觉限制。",
        ]
        .join("\n")
    }
}

fn build_interactive_film_user_prompt(input: &InteractiveFilmCreationInput, language: Language) -> String {
    let spec = render_interactive_film_spec(input);
    let source = input
        .source_text
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match language {
            Language::En => "The user did not provide full source material; write an extensible interactive-film deliverable strictly from the creation spec and user requirements.".to_string(),
            Language::Zh => "用户没有提供完整源素材；请严格根据创作规格和用户要求写一个可继续扩展的互动影游交付稿。".to_string(),
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
            "## 完整源素材",
            &source,
            "",
            "## 输出格式",
            &format!("# {} 互动影游方案", input.title),
            "",
            "## 剧情树",
            "用 Markdown 列出主线节点、分支节点、关键选择、回流/不可回流关系。必须能看出多结局结构。",
            "",
            "## 变量与旗标表",
            "列出变量/旗标名、含义、触发方式、影响范围、对应节点。变量可以是关系、状态、证据、物品、身份、公开/隐瞒、结局门槛等。",
            "",
            "## 多结局路径",
            "列出每个结局的达成条件、关键选择链、必需变量/旗标，以及失败或隐藏结局条件。",
            "",
            "## 互动剧本",
            "按节点写可演剧本：场景、人物、动作、对白、玩家选择、变量变化和分支去向。不要只写摘要。",
            "",
            "## 分镜与图像提示词",
            "列出关键镜头。每个镜头包含画面、人物/物件、动作、景别、时长建议。每个镜头后必须单独写一行 `Prompt: ...`。",
        ]
        .join("\n")
    }
}

// ═══════════════════════════════════════════════════════════════
//  内部辅助函数（private）
// ═══════════════════════════════════════════════════════════════

fn format_script_target(value: Option<ScriptTargetFormat>, language: Language) -> String {
    let format = value.unwrap_or_default();
    if language == Language::En {
        match format {
            ScriptTargetFormat::VerticalShortDrama => "vertical short drama",
            ScriptTargetFormat::Screenplay => "standard screenplay",
            ScriptTargetFormat::AudioDrama => "audio drama",
            ScriptTargetFormat::InteractiveScript => "interactive script",
            ScriptTargetFormat::GeneralScript => "general script",
        }
    } else {
        match format {
            ScriptTargetFormat::VerticalShortDrama => "竖屏短剧",
            ScriptTargetFormat::Screenplay => "标准剧本",
            ScriptTargetFormat::AudioDrama => "广播剧/有声剧",
            ScriptTargetFormat::InteractiveScript => "互动剧本",
            ScriptTargetFormat::GeneralScript => "通用剧本",
        }
    }
    .to_string()
}

fn summarize_source_for_spec(source_text: &Option<String>, language: Language) -> String {
    let text = source_text
        .as_deref()
        .map(|s| regex::Regex::new(r"\s+").map(|re| re.replace_all(s, " ").to_string()).unwrap_or_else(|_| s.to_string()))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if language == Language::En {
        if text.is_empty() {
            "No full source material provided.".to_string()
        } else {
            format!("Full source material provided, about {} characters; the full content will be read during generation.", text.chars().count())
        }
    } else {
        if text.is_empty() {
            "未提供完整源素材。".to_string()
        } else {
            format!("已提供完整源素材，约 {} 字符；生成时会读取完整内容。", text.chars().count())
        }
    }
}

/// 归一化标题文本。
fn normalize_heading_text(text: &str) -> String {
    let trimmed = text.trim();
    // 去首尾 ** 包裹
    let de_bolded = if trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4 {
        &trimmed[2..trimmed.len() - 2]
    } else {
        trimmed
    };
    // 去 markdown 标记字符 ` * _
    let de_marked: String = de_bolded
        .chars()
        .filter(|&c| c != '`' && c != '*' && c != '_')
        .collect();
    // 合并空白并小写
    let collapsed = regex::Regex::new(r"\s+")
        .map(|re| re.replace_all(&de_marked, " ").to_string())
        .unwrap_or(de_marked);
    collapsed.trim().to_lowercase()
}

/// 标题匹配。
fn heading_matches(text: &str, heading: &str) -> bool {
    if text == heading {
        return true;
    }
    if !text.starts_with(heading) {
        return false;
    }
    let rest = text[heading.len()..].trim();
    if rest.is_empty() {
        return true;
    }
    // 剩余以分隔符开头
    rest.starts_with('（')
        || rest.starts_with('(')
        || rest.starts_with('【')
        || rest.starts_with('[')
        || rest.starts_with(':')
        || rest.starts_with('：')
        || rest.starts_with('-')
        || rest.starts_with('—')
        || rest.starts_with(' ')
}

/// 尝试定位图像提示词小节（多标题候选）。
fn extract_image_prompt_section(markdown: &str) -> String {
    for heading in &["图像提示词", "分镜图提示词", "Image Prompts", "Shot Image Prompts"] {
        if let Some(section) = extract_markdown_section(markdown, heading) {
            return section;
        }
    }
    String::new()
}

/// 解析提示词行。
fn parse_prompt_lines(markdown: &str) -> Vec<String> {
    let mut prompts: Vec<String> = Vec::new();
    let mut prompt_column_index: i64 = -1;

    let prompt_re = regex::Regex::new(
        r"(?i)(?:^|[|>\-\d.)、\s])(?:\*\*)?\s*(?:Prompt(?:\s+for\s+[^:*：]+)?|提示词(?:\s*[^:*：]+)?|图像提示词|分镜图提示词)\s*(?:\*\*)?\s*[：:]\s*(.+?)\s*$",
    )
    .expect("invalid prompt regex");

    let numbered_re = regex::Regex::new(
        r"^(?:[-*]\s*)?(?:\d+)[.)、：:\s-]+(.+)$",
    )
    .expect("invalid numbered prompt regex");

    for raw_line in markdown.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            prompt_column_index = -1;
            continue;
        }

        // Markdown 表格行
        if let Some(cells) = parse_markdown_table_row(line) {
            if is_markdown_table_separator(&cells) {
                continue;
            }
            if let Some(idx) = cells.iter().position(|c| is_prompt_column_header(c)) {
                prompt_column_index = idx as i64;
                continue;
            }
            if prompt_column_index >= 0 {
                let cell = cells.get(prompt_column_index as usize).map(|s| s.as_str()).unwrap_or("");
                let prompt = clean_prompt_text(cell);
                if !prompt.is_empty() {
                    prompts.push(prompt);
                }
            }
            continue;
        }

        prompt_column_index = -1;

        // Prompt: ... / 提示词: ... 行
        if let Some(caps) = prompt_re.captures(line) {
            let prompt = clean_prompt_text(&caps[1]);
            if !prompt.is_empty() {
                prompts.push(prompt);
            }
            continue;
        }

        // 编号列表行 1. ...
        if let Some(caps) = numbered_re.captures(line) {
            let prompt = caps[1]
                .chars()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !prompt.is_empty() {
                prompts.push(prompt);
            }
        }
    }

    prompts
}

/// 解析 Markdown 表格行。
fn parse_markdown_table_row(line: &str) -> Option<Vec<String>> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let inner = &line[1..line.len() - 1];
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    if cells.len() >= 2 {
        Some(cells)
    } else {
        None
    }
}

/// 判断是否为表格分隔行。
fn is_markdown_table_separator(cells: &[String]) -> bool {
    let re = regex::Regex::new(r"^:?-{3,}:?$").expect("invalid separator regex");
    cells.iter().all(|c| re.is_match(c))
}

/// 判断是否为 Prompt 列表头。
fn is_prompt_column_header(cell: &str) -> bool {
    let cleaned: String = cell.chars().filter(|&c| c != '`' && c != '*' && c != '_').collect();
    let trimmed = cleaned.trim();
    let re = regex::Regex::new(r"(?i)^(?:prompt|image\s*prompt|shot\s*prompt|提示词|图像提示词|分镜图提示词)$")
        .expect("invalid header regex");
    re.is_match(trimmed)
}

/// 清理提示词文本。
fn clean_prompt_text(text: &str) -> String {
    let step1 = regex::Regex::new(r"\s*\|\s*$")
        .map(|re| re.replace_all(text, "").to_string())
        .unwrap_or_else(|_| text.to_string());

    let step2 = regex::Regex::new(r"\*\*$")
        .map(|re| re.replace_all(&step1, "").to_string())
        .unwrap_or(step1);

    let step3 = regex::Regex::new(r"(?i)^(?:Prompt(?:\s+for\s+[^:*：]+)?|提示词(?:\s*[^:*：]+)?|图像提示词|分镜图提示词)\s*[：:]\s*")
        .map(|re| re.replace_all(&step2, "").to_string())
        .unwrap_or(step2);

    let step4 = regex::Regex::new(r"\s+")
        .map(|re| re.replace_all(&step3, " ").to_string())
        .unwrap_or(step3);

    step4.trim().to_string()
}

// ── 单元测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── estimate_max_tokens ──

    #[test]
    fn estimate_max_tokens_clamps_to_min() {
        assert_eq!(estimate_max_tokens(1, 2200, 12000, 32000), 12000);
    }

    #[test]
    fn estimate_max_tokens_clamps_to_max() {
        assert_eq!(estimate_max_tokens(100, 2200, 12000, 32000), 32000);
    }

    #[test]
    fn estimate_max_tokens_computes_midrange() {
        assert_eq!(estimate_max_tokens(10, 2200, 12000, 32000), 22000);
    }

    // ── normalize_episode_end_labels ──

    #[test]
    fn normalize_episode_end_labels_replaces_with_heading() {
        let markdown = "## 第一集\n\nsome content\n\n字幕：第三集完\n";
        let result = normalize_episode_end_labels(markdown, 1);
        assert!(result.contains("字幕：第一集完"));
        assert!(!result.contains("第三集完"));
    }

    #[test]
    fn normalize_episode_end_labels_uses_fallback_before_heading() {
        let markdown = "字幕：第五集完\n";
        let result = normalize_episode_end_labels(markdown, 3);
        // fallback 使用 current_episode 的阿拉伯数字形式
        assert!(result.contains("字幕：第3集完"));
    }

    #[test]
    fn normalize_episode_end_labels_preserves_arabic_numerals() {
        let markdown = "## 第2集\n\n字幕：第9集完\n";
        let result = normalize_episode_end_labels(markdown, 1);
        assert!(result.contains("字幕：第2集完"));
    }

    // ── extract_markdown_section ──

    #[test]
    fn extract_markdown_section_finds_heading() {
        let content = "# Title\n\n## Storyboard\n\nshot 1\nshot 2\n\n## Image Prompts\n\nPrompt: foo\n";
        let section = extract_markdown_section(content, "Image Prompts");
        assert!(section.is_some());
        assert!(section.unwrap().contains("Prompt: foo"));
    }

    #[test]
    fn extract_markdown_section_returns_none_when_missing() {
        let content = "# Title\n\nNo headings here\n";
        let section = extract_markdown_section(content, "Image Prompts");
        assert!(section.is_none());
    }

    #[test]
    fn extract_markdown_section_stops_at_same_level() {
        let content = "## Section A\n\ncontent a\n\n## Section B\n\ncontent b\n";
        let section = extract_markdown_section(content, "Section A");
        assert!(section.is_some());
        let section = section.unwrap();
        assert!(section.contains("content a"));
        assert!(!section.contains("content b"));
    }

    // ── extract_image_prompts ──

    #[test]
    fn extract_image_prompts_parses_prompt_lines() {
        let markdown = "## Image Prompts\n\nPrompt: a beautiful sunset\nPrompt: a city at night\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "a beautiful sunset");
        assert_eq!(prompts[1], "a city at night");
    }

    #[test]
    fn extract_image_prompts_parses_chinese_prompts() {
        let markdown = "## 图像提示词\n\n提示词：日落\n提示词：夜景\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "日落");
        assert_eq!(prompts[1], "夜景");
    }

    #[test]
    fn extract_image_prompts_parses_numbered_list() {
        let markdown = "## Image Prompts\n\n1. first prompt\n2. second prompt\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "first prompt");
        assert_eq!(prompts[1], "second prompt");
    }

    #[test]
    fn extract_image_prompts_parses_table_column() {
        let markdown = "## Image Prompts\n\n| Shot | Prompt |\n| --- | --- |\n| 1 | sunset |\n| 2 | night |\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "sunset");
        assert_eq!(prompts[1], "night");
    }

    #[test]
    fn extract_image_prompts_falls_back_to_full_content() {
        let markdown = "Prompt: standalone prompt\n";
        let prompts = extract_image_prompts(markdown);
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0], "standalone prompt");
    }

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
        assert!(spec.contains("集数/段落数：6"));
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

    // ── normalize_heading_text ──

    #[test]
    fn normalize_heading_text_lowercases_and_strips_markdown() {
        assert_eq!(normalize_heading_text("**Image Prompts**"), "image prompts");
        assert_eq!(normalize_heading_text("图像提示词"), "图像提示词");
    }
}
