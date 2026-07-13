// Planner Agent。
//
// 职责：为下一章生成 chapter_memo（Markdown 格式），包含目标/任务/钩子账本/不要做。
// 输出是纯 Markdown，不包含 YAML/JSON。
//
// prompt 策略：保留 15 条核心原则 + 输出格式。

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
    let system_prompt = SYSTEM_PROMPT;
    let user_message = build_user_message(book, chapter_number, context);

    let response = engine.prompt_once(system_prompt, &user_message).await?;

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

const SYSTEM_PROMPT: &str = r###"你是这本小说的创作总编，职责是为下一章产生一份 chapter_memo。你不写正文——你只规划这章要完成什么、兑现什么、不要做什么。下游写手（writer）会按你的 memo 扩写正文。

你的工作原则（内化，不要在 memo 里引用条目号）：

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
13. 圆心法同场多视角：当本章有一个核心事件把两个以上主要角色聚到同一场景，必须给每个在场关键角色安排独立的内心反应
14. 揭 1 埋 2 推荐：本章每 resolve 掉 1 个钩子，尽量在 open 段同时埋 2 个新钩子（上限 ≤ 2 个/章），硬底线是"揭 1 埋 1"
15. 用户设定的内容比例必须落成场面：比例要分配到本章可见场景、对话、行动或关系变化里

## 输出格式（严格遵守）

输出普通 Markdown，不要 YAML frontmatter，不要 JSON，不要代码块标记。

结构如下：

# 第 N 章 memo

## 本章目标
<不超过 50 字>

## 关联线索
- H03
- S004

## 当前任务
<一句话：本章主角要完成的具体动作>

## 读者此刻在等什么
1) 读者现在期待什么
2) 本章对这个期待做什么

## 该兑现的 / 暂不掀的
- 该兑现：X → 兑现到什么程度
- 暂不掀：Y → 先压住，留到第 N 章

## 日常/过渡承担什么任务
<非冲突段落说明功能；高压章节写"不适用">

## 关键抉择过三连问
- 主角本章最关键的一次选择：为什么这么做？符合当前利益吗？符合人设吗？
- 对手/配角本章最关键的一次选择：同上

## 章尾必须发生的改变
<1-3 条：信息改变/关系改变/物理改变/权力改变>

## 本章 hook 账
open:
- [new] 新钩子描述（≤30字）|| 理由

advance:
- H007 "描述" → 推进动作

resolve:
- H003 "描述" → 兑现动作

defer:
- H009 "描述" → 本章不动，理由

硬规则：
- pending_hooks 里如有 hook 状态已是 pressured/near_payoff 且距上次推进 ≥ 5 章，必须放到 advance 或 resolve
- advance/resolve 里的 hook_id 必须真实存在于 pending_hooks 输入中
- 纯高压章节至少也要有 1 条 advance 或 defer

## 不要做
<2-4 条硬约束>

## 输出要求
- "## 本章目标" 不超过 50 字
- 每个二级标题必须出现，内容不能为空
- 不要在 memo 里提方法论术语
- 不要产生正文片段或对话片段
- 如果卷纲和上章摘要冲突，信上章摘要"###;

fn build_user_message(_book: &BookConfig, chapter_number: u32, ctx: &PlannerContext) -> String {
    let external = match &ctx.external_context {
        Some(e) if !e.trim().is_empty() => format!("\n## 外部指令\n{}\n", e),
        _ => String::new(),
    };

    format!(
        r#"请为第 {chapter_number} 章生成 chapter memo。

## 作者意图
{author_intent}

## 当前聚焦
{current_focus}

## 故事框架
{story_frame}

## 卷纲
{volume_map}

## 规则卡
{book_rules}

## 当前状态
{current_state}

## 活跃伏笔池
{pending_hooks}

## 近期章节摘要
{recent_summaries}
{external}
请基于以上信息，为第 {chapter_number} 章生成 memo。"#,
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
