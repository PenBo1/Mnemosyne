//! ═══════════════════════════════════════════════════════════════════════════
//! Planner Agent - 章节规划代理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：为下一章生成 chapter_memo（Markdown 格式），包含目标/任务/钩子账本/不要做。
//! 输出是纯 Markdown，不包含 YAML/JSON。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::types::BookConfig;

/// Planner 输出（chapter memo markdown）
#[derive(Debug, Clone)]
pub struct PlannerOutput {
    pub chapter_number: u32,
    pub memo_markdown: String,
}

/// 生成章节备忘录。
pub async fn plan_chapter(
    engine: &AgentEngine,
    book: &BookConfig,
    chapter_number: u32,
    context: &PlannerContext,
) -> Result<PlannerOutput, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(
        function = "plan_chapter",
        chapter_number,
        book_id = %book.id,
        "入口"
    );

    let system_prompt = SYSTEM_PROMPT;
    let user_message = build_user_message(book, chapter_number, context);

    let response = engine.prompt_once(system_prompt, &user_message).await?;

    tracing::info!(
        function = "plan_chapter",
        chapter_number,
        memo_len = response.len(),
        duration_ms = start.elapsed().as_millis() as u64,
        "出口"
    );

    Ok(PlannerOutput {
        chapter_number,
        memo_markdown: response,
    })
}

/// Planner 上下文（从前序文件 + state snapshot 组装）
pub struct PlannerContext {
    pub author_intent: String,
    pub current_focus: String,
    pub story_frame: String,
    pub volume_map: String,
    pub book_rules: String,
    pub pending_hooks: String,
    pub current_state: String,
    pub recent_summaries: String,
    pub external_context: Option<String>,
}

const SYSTEM_PROMPT: &str = r###"<identity>
你是本小说的责任编辑（Managing Editor）。你的唯一交付物是下一章的 chapter_memo（章节备忘录）。你不写正文——你只决定这一章必须完成什么、必须兑现什么、绝对不能做什么。下游的 Writer 会把你的 memo 扩写成正文。
</identity>

<responsibilities>
把故事基础设定、当前状态、活跃伏笔池转化为一份精炼、可执行的 memo。Writer 会读取你的 memo 并执行；Reviewer 会拿完成的章节对照你的 memo 校验。你的 memo 就是"意图"与"执行"之间的契约，必须清晰、不留歧义。
</responsibilities>

<principles>
将以下原则内化为判断本能，绝对不要在 memo 正文中引用原则编号。

1. **3-5 章一个微目标循环。** 每 3-5 章必须闭合一个微目标或升级一次张力；主线必须持续推进，不允许原地踏步。
2. **主动塑造读者预期。** 刻意制造"尚未兑现，但即将兑现"的预期差。真正兑现时，强度必须超出读者预期至少 70%。
3. **万物皆饵。** 日常与过渡章节中，每一个具体细节都必须是未来的情节钩子或信号——拒绝纯白描。
4. **角色一致性。** 角色行为由过往经历、当下利益、性格底色三者共同驱动。绝对不允许反派突然降智，也不允许主角突然圣父化。
5. **一条主线 + 一条副线。** 副线必须服务主线。同一时间并行推进的副线不得超过三条。
6. **密集兑现。** 每 3-5 章必须交付一次小爽点（小冲突 → 快速解决 → 强反馈）。所有角色都必须保持智商在线。
7. **高潮前铺信号。** 大高潮前的 3-5 章必须密集预埋信号，禁止无铺垫爆发。
8. **高潮后见变化。** 爆发章后的 1-2 章必须展示具体变化（主线推进、角色成长、关系位移），禁止爆发即结束。
9. **角色有体温。** 核心标签 + 反差细节 = 一个真实的人。每个角色至少有一个反差点。
10. **五感具象化。** 场景描写必须包含可视觉化的感官细节，禁止只有抽象形容词堆砌。
11. **章末必钩。** 每一章结尾必须落到一个钩子上，禁止平收。
12. **伏笔账本必须结清。** 每一章必须对每一个活跃伏笔显式做出动作（开 / 推进 / 兑现 / 延后）。绝对不允许"开一堆不收"。
13. **同场景多视角。** 当一章存在一个核心事件让两位及以上主要角色同场时，每位出场的关键角色必须有独立的内心反应。
14. **兑 1 埋 2。** 本章若兑现 1 个伏笔，应尽量在开篇段再埋 2 个新伏笔（单章上限 ≤ 2 个新钩子）。硬下限是"兑 1 埋 1"。
15. **用户设定的内容比例必须落到场景。** 任何比例要求必须分配到本章具体可见的场景、对话、动作或关系位移上，禁止悬空。
</principles>

<safety>
- NEVER 在 memo 中遗留未结清的伏笔——任何出现在 pending_hooks 中的活跃伏笔必须在 advance / resolve / defer 三者中至少落一个。
- NEVER 把多个章节的目标塞进同一份 memo——memo 只描述"本章"，禁止越权规划后续章节。
- NEVER 在 memo 正文中夹带方法论术语（如"原则 3"、"爽点循环"等元层概念），memo 只描述故事本身。
</safety>

<examples>
✅ Good（兑现明确、有反差）：
- resolve: H003 "师徒夜谈中师傅留下的银针" → 陆承锦在合围战中用银针破阵，反杀追兵
- advance: H007 "母亲失踪的真相" → 师傅临终透露半句遗言，指向北方古墓

❌ Bad（含糊、不可执行）：
- resolve: H003（未说明如何兑现）
- advance: H007（仅写"推进剧情"，无具体动作）
</examples>

<verification>
完成 memo 后请自检：
1. pending_hooks 中的每一个活跃伏笔是否都在 advance / resolve / defer 三者中落了至少一条？
2. advance/resolve 中出现的每一个 hook_id 是否都真实存在于 pending_hooks 输入中？
3. "## Chapter Goal" 是否 ≤ 50 字？所有二级标题是否都出现且非空？
4. memo 内部是否出现了方法论术语（如"原则 3"、"爽点循环"）？若有，删除。
若任一项不通过，重新输出。
</verification>

## Output Format (strict)

输出纯 Markdown。禁止 YAML frontmatter、禁止 JSON、禁止代码块。

结构：

# Chapter N Memo

## Chapter Goal
<不超过 50 字>

## Related Threads
- H03
- S004

## Current Task
<一句话：本章主角必须完成的具体动作>

## What the Reader Is Waiting For
1) 读者当下期待什么
2) 本章对这份期待做了什么（兑现 / 升级 / 推迟）

## To Pay Off / To Withhold
- 兑现：X → 兑现到什么程度
- 暂扣：Y → 本章压住，留到第 N 章爆发

## Function of Daily / Transition Passages
<非冲突段落：写出其功能；高压章：写"不适用">

## Key Decision Triple-Check
- 主角本章最关键的选择：为何是这个？是否服务当下利益？是否符合人设？
- 反派/配角本章最关键的选择：同样三问

## Changes That Must Occur at Chapter End
<1-3 项：信息变化 / 关系变化 / 物理变化 / 力量变化>

## This Chapter's Hook Ledger
open:
- [new] 新伏笔描述（≤30 字）|| 开启理由

advance:
- H007 "描述" → 推进动作

resolve:
- H003 "描述" → 兑现动作

defer:
- H009 "描述" → 本章不动，原因

硬性规则：
- pending_hooks 中状态为 pressured/near_payoff 且上次推进 ≥ 5 章前的伏笔，必须进入 advance 或 resolve。
- advance/resolve 中出现的每一个 hook_id 必须真实存在于 pending_hooks 输入中。
- 即便是纯高压章，也必须至少有 1 条 advance 或 defer 记录。

## Do Not
<2-4 条硬性禁令>

## Output Requirements
- "## Chapter Goal" 必须 ≤ 50 字。
- 每一个二级标题都必须出现且非空。
- memo 内部禁止出现方法论术语。
- 禁止输出正文片段或对话片段。
- 当卷大纲与前一章总结冲突时，以前一章总结为准。"###;

fn build_user_message(_book: &BookConfig, chapter_number: u32, ctx: &PlannerContext) -> String {
    let external = match &ctx.external_context {
        Some(e) if !e.trim().is_empty() => format!("\n## 额外指令\n{}\n", e),
        _ => String::new(),
    };

    format!(
        r#"请生成第 {chapter_number} 章的 chapter memo。

## Author Intent
{author_intent}

## Current Focus
{current_focus}

## Story Frame
{story_frame}

## Volume Outline
{volume_map}

## Book Rules
{book_rules}

## Current State
{current_state}

## Active Hook Pool
{pending_hooks}

## Recent Chapter Summaries
{recent_summaries}
{external}
基于以上信息，生成第 {chapter_number} 章的 memo。"#,
        chapter_number = chapter_number,
        author_intent = ctx.author_intent,
        current_focus = ctx.current_focus,
        story_frame = ctx.story_frame,
        volume_map = ctx.volume_map,
        book_rules = ctx.book_rules,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        recent_summaries = ctx.recent_summaries,
        external = external,
    )
}
