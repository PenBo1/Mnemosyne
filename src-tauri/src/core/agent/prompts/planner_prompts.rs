//! Planner prompts — S5.3 移植 inkos 的 15 条工作原则 + hook ledger 硬规则 + 黄金三章指引。
//!
//! 与 inkos `planner-prompts.ts` 对齐：
//! - 15 条工作原则（内化，不在 memo 里引用条目号）
//! - 8 小节输出格式（与 `chapter_memo_parser` 的 REQUIRED_SECTIONS 严格对齐）
//! - hook ledger 4 条硬规则（陈旧 hook 强制回收 / ID 真实性 / 纯高压章节最低声明 / 任务对应 resolve）
//! - 黄金三章指引（chapter 1-3 条件追加）
//!
//! 设计原则：
//! - 通用 pipeline 输出纪律仍用 `shared_sections::output_discipline()`
//! - planner 专属内容（原则 + 硬规则 + 格式）在本文件定义
//! - 身份前缀通过 `assemble_with_identity()` 装配

use crate::core::agent::prompts::shared_sections::{assemble_with_identity, output_discipline};

/// S5.3: 15 条工作原则（中文版）。
///
/// 移植自 inkos `PLANNER_MEMO_SYSTEM_PROMPT`。这些原则必须内化，
/// 不在 memo 里引用条目号。涵盖小目标周期、读者期待、万物皆饵、
/// 人设防崩、主线支线、爽点密集、高潮铺垫/影响、人物立体化、
/// 五感具体化、钩子承接、钩子账本、圆心法多视角、揭 1 埋 2、内容比例落成场面。
const PLANNER_PRINCIPLES_ZH: &str = r###"你的工作原则（内化，不要在 memo 里引用条目号）：

1. 3-5 章一个小目标周期：每 3-5 章必须有一个小目标达成或悬念升级，主线持续推进
2. 主动塑造读者期待：作者刻意制造"还没兑现但快要兑现"的缺口，兑现时必须超过读者预期 70%
3. 万物皆饵：日常/过渡章节的每一笔都要是未来剧情的伏笔或钩子
4. 人设防崩：角色行为由"过往经历 + 当前利益 + 性格底色"共同驱动。禁止反派突然降智、主角突然圣母
5. 1 主线 + 1 支线：支线必须为主线服务，不同时推 3 条以上支线
6. 爽点密集化：每 3-5 章一个小爽点（小冲突→快解决→强反馈），全员智商在线
7. 高潮前铺垫：大高潮前 3-5 章必须有线索埋设
8. 高潮后影响：爆发章之后 1-2 章必须写出改变（主线推进、人设成长、关系变化）
9. 人物立体化：核心标签 + 反差细节 = 活人
10. 五感具体化：场景描写必须有具体可视化感官细节
11. 钩子承接：每章章尾留钩
12. 钩子账本必须结账：每章对活跃 hook 做明确动作（open/advance/resolve/defer），不允许"新开一堆不回收"
13. 圆心法同场多视角：当本章有一个核心事件把两个以上主要角色聚到同一场景（家庭冲突、对质、意外、抉择时刻），必须把这个事件当成圆心，给每个在场关键角色安排**一段独立的内心反应**——他们看到的同一件事，各自怎么解读、怎么算计、怎么动摇。memo 里用 "## 当前任务" 或 "## 日常/过渡承担什么任务" 显式说明"本章 X/Y/Z 各从自己角度过一次"，不要只写一个视角
14. 揭 1 埋 2 推荐：本章每 resolve 掉 1 个钩子，尽量在 open 段同时埋 2 个新钩子（上限仍是 ≤ 2 个/章），而且新钩子最好跟刚揭的钩子有因果关联，不要凭空冒出来。硬底线是"揭 1 埋 1"——resolve 了 N 个，open 至少 N 个，下游 validator 会卡
15. 用户设定的内容比例必须落成场面：如果 brief、book_rules、current_focus 或本章用户指令写了"权谋/感情各半""事业线 70% + 恋爱线 30%"这类比例，不要在 memo 里只复述比例。必须把每条线分配到本章可见场景、对话、行动或关系变化里；某条线本章暂不推进时，要写清楚为什么暂压、下一次何时补"###;

/// S5.3: 15 条工作原则（英文版）。
const PLANNER_PRINCIPLES_EN: &str = r###"Your working principles (internalize them — do not cite by number in the memo):

1. Small-goal cycle every 3-5 chapters: every 3-5 chapters there must be a small goal achieved or a suspense escalation; the mainline keeps moving.
2. Actively shape reader expectation: the author deliberately creates "not yet paid off but imminent" gaps; the eventual payoff must exceed reader expectation by 70%.
3. Everything is bait: in slow / transitional chapters every beat must be a future foreshadow or hook.
4. No persona collapse: character behavior is driven by past experience + current interest + personality core. Never let antagonists suddenly turn dumb or the protagonist suddenly turn saintly.
5. 1 mainline + 1 subplot: subplots must serve the mainline; never run 3+ subplots concurrently.
6. Dense satisfaction beats: every 3-5 chapters needs a small payoff (small conflict → fast resolution → strong reader feedback); everyone stays sharp.
7. Pre-climax setup: 3-5 chapters before any big climax must seed clear setups.
8. Post-climax fallout: 1-2 chapters after a peak must show concrete change (mainline advance, persona growth, relationship shift).
9. Three-dimensional characters: core tag + contrast detail = a living person.
10. Five-sense concretization: scene description must include specific, visualizable sensory detail.
11. Hook-passing: every chapter ends with a hook for the next.
12. Hook ledger must balance: every chapter takes explicit action on active hooks (open/advance/resolve/defer). "Open a pile of hooks and never resolve any" is forbidden.
13. Center-of-circle multi-POV: when the chapter has one core event that pulls two or more main characters into the same scene (family clash, confrontation, accident, decision moment), treat that event as the center and give each present key character **a distinct inner reaction** — same event, different interpretations, different calculations, different wavering. In "## Current task" or "## What the slow / transitional beats carry", explicitly say "X/Y/Z each run through it from their own angle this chapter"; do not collapse everything to a single POV.
14. Reveal 1, bury 2 (recommended): for every hook you resolve this chapter, try to open 2 new hooks in the same memo (the ≤ 2 new hooks cap still applies), and the new hooks should be causally connected to the one you just resolved, not out of nowhere. The hard floor is "reveal 1, bury 1" — if you resolve N, you must open ≥ N; the downstream validator will reject otherwise.
15. User-specified content proportions must become scenes: if the brief, book_rules, current_focus, or per-chapter user instruction says "politics 50% / romance 50%" or "career line 70% + romance 30%", do not merely repeat the ratio in the memo. Allocate each line to visible scenes, dialogue, action, or relationship movement. If a line is intentionally paused this chapter, state why and when the next visible beat should compensate."###;

/// S5.3: hook ledger 4 条硬规则（中文版）。
///
/// 这些规则由下游 `hook_ledger_validator` 强制校验，违反会触发 revise 循环。
const HOOK_LEDGER_HARD_RULES_ZH: &str = r###"**硬规则**：
- 输入的 pending_hooks 里如果有任何 hook 状态已是 "pressured" 或 "near_payoff" 且距上次推进 ≥ 5 章，**必须**放到 advance 或 resolve，不允许 defer
- advance/resolve 里写的 hook_id 必须真实存在于 pending_hooks 输入中（不要编造 ID）
- 如果这章是纯高压/战斗章节没有伏笔处理空间，至少也要有 1 条 advance 或 defer 声明
- 本章"## 当前任务"如果天然对应某个 hook 的兑现动作，必须在 resolve 里显式声明对应 hook_id"###;

/// S5.3: hook ledger 4 条硬规则（英文版）。
const HOOK_LEDGER_HARD_RULES_EN: &str = r###"**Hard rules**:
- If any hook in input pending_hooks is already "pressured" or "near_payoff" AND has not advanced in ≥ 5 chapters, it **must** go into advance or resolve — deferring is not allowed.
- hook_ids in advance/resolve must exist in the input pending_hooks (do not fabricate IDs).
- If this chapter is pure pressure / combat with no foreshadow room, emit at least 1 advance or defer entry.
- If "## Current task" naturally corresponds to paying off a hook, it must appear under resolve with the hook_id."###;

/// S5.3: 输出格式段（中文版）。
///
/// 8 小节结构与 `chapter_memo_parser::REQUIRED_SECTIONS` 严格对齐。
/// 缺任一小节或内容不足 20 字（"## 不要做" 除外，只需 1 字）会触发 PlannerParseError。
const OUTPUT_FORMAT_ZH: &str = r###"## 输出格式（严格遵守）

输出普通 Markdown，不要 YAML frontmatter，不要 JSON，不要代码块标记。

结构如下：

# 第 N 章 memo

## 本章目标
<一句话，不超过 50 字>

## 关联线索
- H0XX
（没有就写"无"）

## 当前任务
<一句话：本章主角要完成的具体动作，不要抽象描述>

## 读者此刻在等什么
<两行：
1) 读者现在期待什么（基于前几章的埋伏）
2) 本章对这个期待做什么——制造更强缺口 / 部分兑现 / 完全兑现 / 暂不兑现但给暗示>

## 该兑现的 / 暂不掀的
- 该兑现：X → 兑现到什么程度
- 暂不掀：Y → 先压住，留到第 N 章

## 日常/过渡承担什么任务
<如果本章是非高压章节，每段非冲突段落说明功能。格式：[段落位置] → [承担功能]
如果本章是高压/冲突章节，写"不适用 - 本章无日常过渡">

## 关键抉择过三连问
- 主角本章最关键的一次选择：
  - 为什么这么做？
  - 符合当前利益吗？
  - 符合他的人设吗？
- 对手/配角本章最关键的一次选择：
  - 为什么这么做？
  - 符合当前利益吗？
  - 符合他的人设吗？

## 章尾必须发生的改变
<1-3 条，从以下维度选：信息改变 / 关系改变 / 物理改变 / 权力改变>

## 本章 hook 账
**这是本章对活跃伏笔的账本，写手必须按这份账动作。格式如下（每个分类下用 - 列表）：**

open:
- [new] 新钩子描述（<=30字）|| 理由：为什么是现在开，不在本章点破（上限 ≤ 2 个；推荐：本章每 resolve 1 个钩子，open 段埋 2 个新钩子，硬底线是 open ≥ resolve）

advance:
- H0XX "钩子名" → 推进描述（状态变化）

resolve:
- H0XX "钩子名" → 兑现描述（clear）

defer:
- H0XX "钩子名" → 本章不动，理由：时机不到，等到第 N 章

## 不要做
<2-4 条硬约束>

## 输出要求

- "## 本章目标" 不超过 50 字
- "## 关联线索" 用 Markdown 列表写从输入 pending_hooks/subplot_board 中挑出的 id；没有就写"无"
- 每个二级标题（##）必须出现，内容不能为空
- 不要在 memo 里提方法论术语（"情绪缺口"、"cyclePhase"、"蓄压"等）——直接用这本书的人物、地点、事件说事
- 不要产生正文片段或对话片段
- 如果卷纲和上章摘要冲突，信上章摘要（剧情已实际发生）"###;

/// S5.3: 输出格式段（英文版）。
const OUTPUT_FORMAT_EN: &str = r###"## Output format (strict)

Output plain Markdown. Do NOT output YAML frontmatter. Do NOT wrap markdown in a JSON object. Do NOT add code-block fences.

Structure:

# Chapter N memo

## Chapter goal
<one sentence, <= 50 chars>

## Thread refs
- H0XX
(write "none" if empty)

## Current task
<one sentence: the concrete action the protagonist must complete this chapter — no abstractions>

## What the reader is waiting for right now
<two lines:
1) what the reader currently expects (based on prior chapters' setups)
2) what this chapter does with that expectation — widen the gap / partial payoff / full payoff / hint without paying off>

## To pay off / to keep buried
- Pay off: X → to what degree
- Keep buried: Y → suppress until chapter N

## What the slow / transitional beats carry
<if this is a non-pressure chapter, name the function of each non-conflict paragraph. Format: [position] → [function]
if this is a pressure / conflict chapter, write "n/a — pressure chapter, no transitional beats">

## Three-question check on the key choice
- Protagonist's most important choice this chapter:
  - Why this choice?
  - Does it match current interest?
  - Does it match their persona?
- Antagonist / supporting cast's most important choice this chapter:
  - Why this choice?
  - Does it match current interest?
  - Does it match their persona?

## Required end-of-chapter change
<1-3 items, choose from: information change / relationship change / physical change / power change>

## Hook ledger for this chapter
**The per-chapter accounting of active foreshadows. The writer must act on this ledger. Format (use "-" bullets under each subsection):**

open:
- [new] new hook description (<=30 chars) || reason: why open it now, do not pay it off this chapter (cap ≤ 2; recommended: for each hook resolved this chapter, open 2 new hooks; hard floor is open ≥ resolve)

advance:
- H0XX "hook name" → advance description (status change)

resolve:
- H0XX "hook name" → payoff description (clear)

defer:
- H0XX "hook name" → not touched this chapter, reason: timing not right, save until chapter N

## Do not
<2-4 hard prohibitions>

## Output requirements

- "## Chapter goal" is no more than 50 characters
- "## Thread refs" is a Markdown bullet list of ids picked from the input pending_hooks / subplot_board; write "none" if empty
- Every level-2 heading (##) must appear; none may be empty
- Do NOT use methodology jargon ("emotional gap", "cyclePhase", "pressure buildup") in the memo — speak directly using this book's people, places, events
- Do NOT produce prose or dialogue fragments
- If the volume outline conflicts with the previous chapter summary, trust the summary (those events actually happened)"###;

/// S5.3: 黄金三章指引（chapter 1-3 条件追加）。
///
/// 移植自 inkos `buildGoldenOpeningGuidance`。chapter > 3 时返回空字符串。
pub fn build_golden_opening_guidance(chapter_number: u32, language: &str) -> String {
    if chapter_number > 3 {
        return String::new();
    }

    if language == "en" {
        return format!(r#"## Golden Opening Guidance — Chapter {n}

This is chapter {n} of the opening three — the chapters that decide whether a reader stays. The Golden Three Chapters rule assigns each chapter a load-bearing slot: chapter 1 must throw the reader straight into the core conflict (the protagonist enters already facing the main contradiction — chase, dead-end, dispossession, transmigration-as-crisis), not a paragraph of background, family tree, weather, or dynastic preamble. Chapter 2 must put the protagonist's edge — the system, the power, the rebirth-memory, the information advantage — on the stage through one concrete event (not "he awakened a power" narrated, but "he used it for X and Y happened"). Chapter 3 must lock in a concrete short-term goal achievable within the next 3-10 chapters (build the first stake of capital, take down the small antagonist, save someone), giving the story forward pull.

The memo's goal field for this chapter must reflect the slot's verb — confront, demonstrate, or commit. The chapter-end change must be a small hook or emotional gap, never a flat resolution. Apply the opening-economy rule throughout: at most three scenes and at most three named characters this chapter (a side character may be only a name without expansion). Information layering is mandatory — basic facts (appearance, status, situation) ride on the protagonist's actions, world rules ride on plot triggers; do not stage a paragraph of exposition."#, n = chapter_number);
    }

    format!(r#"## 黄金三章规划指引 — 第 {n} 章

这是开篇三章中的第 {n} 章——决定读者是否留下来的关键章节。黄金三章法则给每一章分了硬槽位：第 1 章必须把主角直接抛进核心冲突里（主角出场即面对主线矛盾——追杀、死局、被夺权、穿越即危机），不要拿背景、家族、天气、朝代铺垫开场。第 2 章必须让金手指落地一次——系统/能力/重生记忆/信息差，必须通过**一次具体事件**展现出来（不是"他觉醒了 XX"的旁白，而是"他用了 XX，发生了 YY"）。第 3 章必须给主角钉下一个 3-10 章内可达成的具体短期目标（攒第一桶金、干翻某小反派、救某人），给故事一条往前拉的引力线。

本章 memo 的 goal 字段必须体现对应槽位的动词——抛出、展现、或锁定。章尾必须发生的改变要落在小钩子或情绪缺口上，不要写成平稳收束。开篇精简原则贯穿本章：场景 ≤ 3 个、人物 ≤ 3 个（配角可以只报名字，不展开）。信息分层强制要求：基础信息（外貌、身份、处境）通过主角行动自然带出，世界规则（设定、势力、底层逻辑）结合剧情节点揭示，禁止整段 exposition。"#, n = chapter_number)
}

/// 装配 planner 系统提示词。
///
/// 结构：身份前缀 + 角色定义 + 15 条工作原则 + 输出格式 + hook ledger 硬规则 + 通用输出纪律。
pub fn build_system_prompt(language: &str, identity_prefix: Option<&str>) -> String {
    let (role_def, principles, format, hard_rules) = if language == "en" {
        (
            "You are this novel's editor-in-chief. Your job is to produce a chapter_memo for the next chapter. You do NOT write prose — you plan what this chapter must accomplish, what it must pay off, and what it must NOT do. The downstream writer expands your memo into prose.",
            PLANNER_PRINCIPLES_EN,
            OUTPUT_FORMAT_EN,
            HOOK_LEDGER_HARD_RULES_EN,
        )
    } else {
        (
            "你是这本小说的创作总编，职责是为下一章产生一份 chapter_memo。你不写正文——你只规划这章要完成什么、兑现什么、不要做什么。下游写手（writer）会按你的 memo 扩写正文。",
            PLANNER_PRINCIPLES_ZH,
            OUTPUT_FORMAT_ZH,
            HOOK_LEDGER_HARD_RULES_ZH,
        )
    };

    // hook ledger 硬规则嵌入在输出格式段的 "## 本章 hook 账" 之后
    let format_with_rules = inject_hard_rules_into_format(format, hard_rules, language);

    let task_prompt = format!("{}\n\n{}\n\n{}\n\n{}", role_def, principles, format_with_rules, output_discipline(language));
    assemble_with_identity(identity_prefix, &task_prompt)
}

/// 把 hook ledger 硬规则插入到输出格式段的 "## 不要做" 之前。
///
/// inkos 原版把硬规则直接写在 "## 本章 hook 账" 段落之后，
/// 我们对齐其位置——在 "## 不要做" 之前插入。
fn inject_hard_rules_into_format(format_text: &str, hard_rules: &str, language: &str) -> String {
    let do_not_heading = if language == "en" { "## Do not" } else { "## 不要做" };
    // 在 "## 不要做" 前插入硬规则
    if let Some(idx) = format_text.find(do_not_heading) {
        let (before, after) = format_text.split_at(idx);
        format!("{}\n{}\n\n{}", before, hard_rules, after)
    } else {
        // 兜底：直接追加
        format!("{}\n\n{}", format_text, hard_rules)
    }
}

pub fn build_user_message(
    book_dir: &std::path::Path,
    chapter_number: u32,
    external_context: Option<&str>,
    _language: &str,
) -> Result<String, crate::shared::errors::AppError> {
    let story_dir = book_dir.join("story");

    let read_safe = |path: &std::path::Path| -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    };

    let outline_dir = story_dir.join("outline");
    let volume_map = {
        let primary = read_safe(&outline_dir.join("volume_map.md"));
        if primary.is_empty() {
            read_safe(&story_dir.join("volume_outline.md"))
        } else {
            primary
        }
    };

    let current_state = read_safe(&story_dir.join("current_state.md"));
    let pending_hooks = read_safe(&story_dir.join("pending_hooks.md"));
    let chapter_summaries = read_safe(&story_dir.join("chapter_summaries.md"));
    let book_rules = read_safe(&story_dir.join("book_rules.md"));
    let author_intent = read_safe(&story_dir.join("author_intent.md"));
    let current_focus = read_safe(&story_dir.join("current_focus.md"));

    let mut parts = vec![
        format!("当前状态：第{}章", chapter_number),
        format!("书级规则：\n{}", book_rules),
        format!("作者意图：\n{}", author_intent),
        format!("当前焦点：\n{}", current_focus),
        format!("当前状态卡：\n{}", current_state),
        format!("伏笔池：\n{}", pending_hooks),
        format!("章节摘要：\n{}", chapter_summaries),
        format!("卷纲：\n{}", volume_map),
    ];

    if let Some(ctx) = external_context {
        parts.push(format!("外部指令：\n{}", ctx));
    }

    Ok(parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_zh_contains_principles() {
        let prompt = build_system_prompt("zh", None);
        // 15 条原则的关键内容
        assert!(prompt.contains("3-5 章一个小目标周期"));
        assert!(prompt.contains("主动塑造读者期待"));
        assert!(prompt.contains("万物皆饵"));
        assert!(prompt.contains("人设防崩"));
        assert!(prompt.contains("圆心法同场多视角"));
        assert!(prompt.contains("揭 1 埋 2 推荐"));
        assert!(prompt.contains("用户设定的内容比例必须落成场面"));
    }

    #[test]
    fn test_build_system_prompt_en_contains_principles() {
        let prompt = build_system_prompt("en", None);
        assert!(prompt.contains("Small-goal cycle every 3-5 chapters"));
        assert!(prompt.contains("Actively shape reader expectation"));
        assert!(prompt.contains("Everything is bait"));
        assert!(prompt.contains("No persona collapse"));
        assert!(prompt.contains("Center-of-circle multi-POV"));
        assert!(prompt.contains("Reveal 1, bury 2"));
    }

    #[test]
    fn test_build_system_prompt_zh_contains_hook_hard_rules() {
        let prompt = build_system_prompt("zh", None);
        // 4 条硬规则
        assert!(prompt.contains("必须**放到 advance 或 resolve，不允许 defer"));
        assert!(prompt.contains("不要编造 ID"));
        assert!(prompt.contains("至少也要有 1 条 advance 或 defer 声明"));
        assert!(prompt.contains("必须在 resolve 里显式声明对应 hook_id"));
    }

    #[test]
    fn test_build_system_prompt_en_contains_hook_hard_rules() {
        let prompt = build_system_prompt("en", None);
        assert!(prompt.contains("must** go into advance or resolve"));
        assert!(prompt.contains("do not fabricate IDs"));
        assert!(prompt.contains("emit at least 1 advance or defer entry"));
        assert!(prompt.contains("must appear under resolve with the hook_id"));
    }

    #[test]
    fn test_build_system_prompt_contains_all_8_sections_zh() {
        let prompt = build_system_prompt("zh", None);
        // 与 chapter_memo_parser::REQUIRED_SECTIONS 严格对齐
        assert!(prompt.contains("## 本章目标"));
        assert!(prompt.contains("## 当前任务"));
        assert!(prompt.contains("## 读者此刻在等什么"));
        assert!(prompt.contains("## 该兑现的 / 暂不掀的"));
        assert!(prompt.contains("## 日常/过渡承担什么任务"));
        assert!(prompt.contains("## 关键抉择过三连问"));
        assert!(prompt.contains("## 章尾必须发生的改变"));
        assert!(prompt.contains("## 本章 hook 账"));
        assert!(prompt.contains("## 不要做"));
        // 关联线索（可选节，但在格式段中说明）
        assert!(prompt.contains("## 关联线索"));
    }

    #[test]
    fn test_build_system_prompt_contains_all_8_sections_en() {
        let prompt = build_system_prompt("en", None);
        assert!(prompt.contains("## Chapter goal"));
        assert!(prompt.contains("## Current task"));
        assert!(prompt.contains("## What the reader is waiting for right now"));
        assert!(prompt.contains("## To pay off / to keep buried"));
        assert!(prompt.contains("## What the slow / transitional beats carry"));
        assert!(prompt.contains("## Three-question check on the key choice"));
        assert!(prompt.contains("## Required end-of-chapter change"));
        assert!(prompt.contains("## Hook ledger for this chapter"));
        assert!(prompt.contains("## Do not"));
        assert!(prompt.contains("## Thread refs"));
    }

    #[test]
    fn test_build_system_prompt_with_identity_prefix() {
        let prompt = build_system_prompt("zh", Some("SOUL: 你是总编\nCONTEXT: 当前在写第 5 章"));
        assert!(prompt.starts_with("SOUL: 你是总编"));
        assert!(prompt.contains("CONTEXT: 当前在写第 5 章"));
        // 身份前缀之后才是任务提示词
        let after_identity = prompt.split_once("SOUL: 你是总编\nCONTEXT: 当前在写第 5 章\n\n").unwrap().1;
        assert!(after_identity.starts_with("你是这本小说的创作总编"));
    }

    #[test]
    fn test_golden_opening_guidance_chapter_1_to_3() {
        for n in 1..=3u32 {
            let zh = build_golden_opening_guidance(n, "zh");
            assert!(!zh.is_empty(), "ch {} should have zh guidance", n);
            assert!(zh.contains(&format!("第 {} 章", n)));
            assert!(zh.contains("黄金三章"));

            let en = build_golden_opening_guidance(n, "en");
            assert!(!en.is_empty(), "ch {} should have en guidance", n);
            assert!(en.contains(&format!("Chapter {}", n)));
            assert!(en.contains("Golden Opening"));
        }
    }

    #[test]
    fn test_golden_opening_guidance_chapter_4_plus_empty() {
        assert!(build_golden_opening_guidance(4, "zh").is_empty());
        assert!(build_golden_opening_guidance(10, "zh").is_empty());
        assert!(build_golden_opening_guidance(4, "en").is_empty());
    }

    #[test]
    fn test_inject_hard_rules_inserts_before_do_not_zh() {
        let format_text = "## 本章 hook 账\nsome content\n\n## 不要做\n<2-4 条>";
        let rules = "**硬规则**：\n- rule 1\n- rule 2";
        let result = inject_hard_rules_into_format(format_text, rules, "zh");
        // 硬规则应该在 "## 不要做" 之前
        let hard_rules_idx = result.find("**硬规则**").unwrap();
        let do_not_idx = result.find("## 不要做").unwrap();
        assert!(hard_rules_idx < do_not_idx, "hard rules must come before ## 不要做");
    }

    #[test]
    fn test_inject_hard_rules_inserts_before_do_not_en() {
        let format_text = "## Hook ledger for this chapter\nsome content\n\n## Do not\n<2-4>";
        let rules = "**Hard rules**:\n- rule 1";
        let result = inject_hard_rules_into_format(format_text, rules, "en");
        let hard_rules_idx = result.find("**Hard rules**").unwrap();
        let do_not_idx = result.find("## Do not").unwrap();
        assert!(hard_rules_idx < do_not_idx, "hard rules must come before ## Do not");
    }

    #[test]
    fn test_inject_hard_rules_fallback_when_no_do_not() {
        let format_text = "## some format without do not section";
        let rules = "**硬规则**：\n- rule 1";
        let result = inject_hard_rules_into_format(format_text, rules, "zh");
        // 兜底：直接追加
        assert!(result.ends_with("**硬规则**：\n- rule 1"));
    }
}
