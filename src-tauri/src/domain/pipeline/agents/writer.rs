// Writer Agent。
//
// 3-phase 流程：
// 1. Creative（temperature 0.7）：写正文，输出 PRE_WRITE_CHECK + CHAPTER_TITLE + CHAPTER_CONTENT
// 2. Observer（temperature 0.5）：提取章节事实，输出 === OBSERVATIONS ===
// 3. Settler（temperature 0.3）：状态结算，输出 === POST_SETTLEMENT === + === RUNTIME_STATE_DELTA === JSON
//
// prompt 策略：保留核心 prompt（核心规则、输出格式、观察类别、伏笔追踪规则）。
// 精简 governed context / POV filtering / dialogue fingerprints 等高级特性，留给后续阶段。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::super::state::types::RuntimeStateDelta;
use super::super::types::BookConfig;
use super::super::utils::text_parse::{count_non_whitespace_chars, extract_section};

/// Writer 输出（3-phase 合并结果）
#[derive(Debug, Clone)]
pub struct WriterOutput {
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub word_count: u32,
    pub pre_write_check: String,
    pub post_settlement: String,
    pub runtime_state_delta: Option<RuntimeStateDelta>,
    pub observations: String,
}

/// Writer 上下文（从前序文件 + state snapshot 组装）
pub struct WriterContext {
    pub story_frame: String,
    pub volume_map: String,
    pub current_state: String,
    pub pending_hooks: String,
    pub chapter_summaries: String,
    pub recent_chapters: String,
    pub chapter_memo: String,
    pub book_rules: String,
    pub style_guide: String,
    pub external_context: Option<String>,
}

/// 写一章。
pub async fn write_chapter(
    engine: &AgentEngine,
    book: &BookConfig,
    chapter_number: u32,
    ctx: &WriterContext,
) -> Result<WriterOutput, AppError> {
    // ── Phase 1: Creative writing ──
    let creative_system = build_creative_system_prompt(book);
    let creative_user = build_creative_user_message(book, chapter_number, ctx);
    let creative_response = engine.prompt_once(&creative_system, &creative_user).await?;
    let creative = parse_creative_output(chapter_number, &creative_response)?;

    // ── Phase 2: Observer — extract facts ──
    let observer_system = build_observer_system_prompt(book);
    let observer_user = build_observer_user_message(chapter_number, &creative.title, &creative.content);
    let observations = engine.prompt_once(&observer_system, &observer_user).await?;

    // ── Phase 3: Settler — merge into truth files（调用独立入口）──
    let settle_output = settle_chapter_state(
        engine,
        SettleChapterStateInput {
            book,
            chapter_number,
            title: &creative.title,
            content: &creative.content,
            current_state: &ctx.current_state,
            pending_hooks: &ctx.pending_hooks,
            chapter_summaries: &ctx.chapter_summaries,
            observations: &observations,
            validation_feedback: None,
        },
    )
    .await?;
    let post_settlement = settle_output.post_settlement;
    let runtime_state_delta = settle_output.runtime_state_delta;

    let word_count = count_non_whitespace_chars(&creative.content);

    Ok(WriterOutput {
        chapter_number,
        title: creative.title,
        content: creative.content,
        word_count,
        pre_write_check: creative.pre_write_check,
        post_settlement,
        runtime_state_delta,
        observations,
    })
}

// ── 独立 Settler 入口 ───────────────────────────────────────

/// `settle_chapter_state` 输入。
///
/// 用于独立触发 Settler phase（不重写正文），供：
/// - `write_chapter` Phase 3 内部调用
/// - `chapter_truth_validation::retry_settlement` 重试结算
/// - `chapter_state_recovery` 状态恢复
///
/// `validation_feedback` 存在时表示这是一次重试，会注入到 settler user message 中
/// 提醒模型修正上一次校验发现的问题。
pub struct SettleChapterStateInput<'a> {
    pub book: &'a BookConfig,
    pub chapter_number: u32,
    pub title: &'a str,
    pub content: &'a str,
    pub current_state: &'a str,
    pub pending_hooks: &'a str,
    pub chapter_summaries: &'a str,
    pub observations: &'a str,
    /// 重试场景下上一次校验的反馈文本（None 表示首次结算）
    pub validation_feedback: Option<&'a str>,
}

/// `settle_chapter_state` 输出。
#[derive(Debug, Clone)]
pub struct SettleChapterStateOutput {
    pub post_settlement: String,
    pub runtime_state_delta: Option<RuntimeStateDelta>,
}

/// 独立触发 Settler phase：给定章节正文 + 当前 truth 文件 + 观察日志，
/// 产出状态增量 delta（不重写正文）。
pub async fn settle_chapter_state(
    engine: &AgentEngine,
    input: SettleChapterStateInput<'_>,
) -> Result<SettleChapterStateOutput, AppError> {
    let system_prompt = build_settler_system_prompt(input.book);
    let user_message = build_settler_user_message(
        input.chapter_number,
        input.title,
        input.content,
        input.current_state,
        input.pending_hooks,
        input.chapter_summaries,
        input.observations,
        input.validation_feedback,
    );
    let response = engine.prompt_once(&system_prompt, &user_message).await?;
    let (post_settlement, runtime_state_delta) = parse_settler_output(&response)?;
    Ok(SettleChapterStateOutput {
        post_settlement,
        runtime_state_delta,
    })
}

// ── Phase 1: Creative ───────────────────────────────────────

fn build_creative_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"<identity>
你是一名专业的网文小说家，正在为 {platform} 平台写作。
</identity>

<core_rules>
1. 使用简体中文写作。长短句交替。段落适配移动端阅读（每段 3-5 行）。
2. 每章必须有明确的推进目标，禁止流水账。
3. 场景描写必须包含可视觉化的感官细节（五感具象化）。
4. 角色行为由过往经历、当下利益、性格底色三者共同驱动。绝对不允许反派突然降智，也不允许主角突然圣父化。
5. 不同角色说话风格必须不同。禁止"众人齐声惊呼"这类群像齐发声。
6. 通过动作外化情绪（不要写"他很愤怒"——写他做了什么动作）。通过行为传达价值观。
7. 坏事层层叠加。每一层必须比上一层更糟。
8. 每章结尾必须落到一个钩子上。
9. chapter memo 中的"Current Task"、"Do Not"、"Changes That Must Occur at Chapter End"必须在正文中真正落地执行。
10. hook ledger 中 advance/resolve 列出的每一个 hook_id，都必须在正文中有一段具体可定位的兑现段落（≥ 60 字符）。
</core_rules>

<safety>
- NEVER 在正文结尾追加"本章完"、"未完待续"、"敬请期待下章"等总结性或元层尾句——结尾必须以故事钩子收束。
- NEVER 让 advance/resolve 中的伏笔只在 memo 中被提及但在正文里没有对应落点——任何承诺兑现的伏笔必须有可定位的段落。
- NEVER 输出 JSON、YAML、Markdown 代码块——只输出三个 === 区块标记 + 正文，区块外禁止任何额外说明。
</safety>

<examples>
✅ Good（感官具象 + 动作外化）：
> 雨水顺着青石板的裂缝渗下来，在陆承锦的靴边积成一小汪浊水。他没抬头，只是把刀往腰带里又塞深了一寸，刀柄上缠的麻布被汗浸得发黑。

❌ Bad（抽象堆砌 + 内心独白代替动作）：
> 陆承锦心情沉重，他感到无比愤怒和悲伤，但他必须坚强。这种复杂的情绪在他心中翻涌。
</examples>

<verification>
写完正文后请自检：
1. PRE_WRITE_CHECK 中列出的每一个 hook（advance/resolve）是否在正文中都有一段 ≥ 60 字符的可定位兑现段落？
2. chapter memo 的 "Current Task"、"Do Not"、"Changes That Must Occur at Chapter End" 是否都在正文中真正落地？
3. 章节结尾是否落在一个故事钩子上（而非"本章完"等元层尾句）？
4. 字数是否在 {soft_min}-{soft_max} 范围内？
若任一项不通过，重新输出。
</verification>

## Word-Count Governance
- 目标字数：{target_words} 字
- 可接受范围：{soft_min}-{soft_max} 字

## Output Format (strict)

先输出写前自检，再输出正文。严格输出三块：

=== PRE_WRITE_CHECK ===
（写前自检：列出本章任务、需要兑现的伏笔、需要避免的事项）
=== CHAPTER_TITLE ===
（章节标题——不带书名号、不带"第 N 章"前缀）
=== CHAPTER_CONTENT ===
（正文内容）"#,
        platform = format!("{:?}", book.platform).to_lowercase(),
        target_words = book.chapter_word_count,
        soft_min = (book.chapter_word_count as f32 * 0.85) as u32,
        soft_max = (book.chapter_word_count as f32 * 1.15) as u32,
    )
}

fn build_creative_user_message(_book: &BookConfig, chapter_number: u32, ctx: &WriterContext) -> String {
    let external = match &ctx.external_context {
        Some(e) if !e.trim().is_empty() => format!("\n## 本章用户指令（最高优先级）\n{}\n", e),
        _ => String::new(),
    };

    format!(
        r#"请续写第 {chapter_number} 章。
{external}
## Chapter Memo
{chapter_memo}

## Current State Card
{current_state}

## Hook Pool
{pending_hooks}

## Chapter Summaries (compressed history)
{chapter_summaries}

## Recent Chapters
{recent_chapters}

## World Setting
{story_frame}

## Volume Outline
{volume_map}

## Book Rules
{book_rules}

## Style Guide
{style_guide}

基于以上信息，先输出 PRE_WRITE_CHECK 自检，再写正文。"#,
        chapter_number = chapter_number,
        external = external,
        chapter_memo = ctx.chapter_memo,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        recent_chapters = if ctx.recent_chapters.is_empty() {
            "（这是第一章，没有前文。）".to_string()
        } else {
            ctx.recent_chapters.clone()
        },
        story_frame = ctx.story_frame,
        volume_map = ctx.volume_map,
        book_rules = ctx.book_rules,
        style_guide = if ctx.style_guide.is_empty() {
            "（无风格指南。）"
        } else {
            &ctx.style_guide
        },
    )
}

/// Creative phase 输出
struct CreativeOutput {
    pre_write_check: String,
    title: String,
    content: String,
}

fn parse_creative_output(chapter_number: u32, response: &str) -> Result<CreativeOutput, AppError> {
    let pre_write_check = extract_section(response, "PRE_WRITE_CHECK")
        .ok_or_else(|| AppError::invalid_format("PRE_WRITE_CHECK block missing"))?;

    let title = extract_section(response, "CHAPTER_TITLE")
        .ok_or_else(|| AppError::invalid_format("CHAPTER_TITLE block missing"))?
        .trim()
        .to_string();

    let content = extract_section(response, "CHAPTER_CONTENT")
        .ok_or_else(|| AppError::invalid_format("CHAPTER_CONTENT block missing"))?;

    if title.is_empty() {
        return Err(AppError::invalid_format("chapter title is empty"));
    }
    if content.is_empty() {
        return Err(AppError::invalid_format(format!(
            "chapter {} content is empty",
            chapter_number
        )));
    }

    Ok(CreativeOutput {
        pre_write_check,
        title,
        content,
    })
}

// ── Phase 2: Observer ───────────────────────────────────────

fn build_observer_system_prompt(_book: &BookConfig) -> String {
    r#"<identity>
你是一名事实抽取专员。你的任务是阅读章节正文，浮现其中发生的一切可观察变化。
</identity>

<responsibilities>
- 抽取本章正文实际在页面上确认的事实变化——不推测、不假设、不脑补。
- 覆盖九大类别，逐项检查：角色行为、位置、资源、关系、情绪、信息流、伏笔、时间推进、物理状态。
- 每条记录必须具体且可定位，让 settler agent 无需重读正文即可更新 truth 文件。
</responsibilities>

## Extraction Categories

1. **角色行为**：谁对谁做了什么，为什么。
2. **位置变化**：谁从哪里到了哪里。
3. **资源变化**：得到了什么、失去了什么、消耗了什么——附带精确数量。
4. **关系变化**：新结识、信任/不信任的转移、结盟、背叛。
5. **情绪变化**：某角色的情感从 X 变为 Y，由哪个事件触发。
6. **信息流**：谁获知了新事实，通过什么渠道；谁仍然不知道。
7. **伏笔**：新埋的钩子、已有线索的推进、先前线索的兑现。
8. **时间推进**：经过了多长时间，正文中提到的任何时间标记。
9. **物理状态**：受伤、康复、疲劳、战力波动。

<rules>
- 只从正文中抽取——绝不推测正文之外可能发生的事。
- 宁滥勿缺：拿不准某件事是否重要时，记录下来。
- 必须具体：写"陆承锦左肩旧伤复发"，不写"陆承锦受伤"。
- 显式记录正文中提到的每一个时间标记。
- 记录每个场景中在场的角色。
</rules>

<safety>
- NEVER 推测正文之外可能发生的事——所有记录必须能在原文中找到证据。
- NEVER 把"暗示"或"可能"当事实记录——如果原文没有明确写出，就不要列入 OBSERVATIONS。
- NEVER 合并多个角色的行为到同一条记录——每个角色、每个动作单独成行。
</safety>

<verification>
完成抽取后请自检：
1. 九大类别是否都至少扫过一遍（即使为空也应显式列出标题）？
2. 每条记录是否都能在原文中找到对应的句子或段落？
3. 是否有"可能"、"也许"、"大概"等推测性措辞？若有，删除或改为正文已确认的事实。
若任一项不通过，重新输出。
</verification>

## Output Format

=== OBSERVATIONS ===

[Character Behavior]
- <角色>: <动作 / 状态变化> (Scene: <地点>)

[Location Changes]
- <角色> 从 <A> 到 <B>

[Resource Changes]
- <角色> 获得 / 失去 <物品> (Quantity: <n>)

[Relationship Changes]
- <角色 A> → <角色 B>: <变化描述>

[Emotional Changes]
- <角色>: <之前> → <之后> (Trigger: <事件>)

[Information Flow]
- <角色> 获知: <事实> (Source: <渠道>)
- <角色> 仍不知道: <事实>

[Plot Threads]
- Planted: <描述>
- Advanced: <已有伏笔> — <推进内容>
- Resolved: <伏笔> — <兑现方式>

[Time]
- <时间标记、流逝时长>

[Physical State]
- <角色>: <受伤 / 康复 / 疲劳 / 战力变化>"#
        .to_string()
}

fn build_observer_user_message(chapter_number: u32, title: &str, content: &str) -> String {
    format!(
        "请从第 {chapter_number} 章 \"{title}\" 中抽取一切可观察事实：\n\n{content}",
        chapter_number = chapter_number,
        title = title,
        content = content,
    )
}

// ── Phase 3: Settler ────────────────────────────────────────

fn build_settler_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"<identity>
你是一名状态追踪分析师。给定新章正文与当前 truth 文件，你的任务是产出更新后的 truth-file 增量 delta。
</identity>

## Operating Mode

你不是在写小说。你的任务是：
1. 仔细阅读正文，抽取每一项状态变化。
2. 在"当前追踪文件"之上叠加增量更新。
3. 严格按下方定义的 `=== TAG ===` 格式输出。

## Analysis Dimensions

从正文中抽取：
- 角色登场、退场、状态变化（受伤 / 突破 / 死亡等）。
- 位置移动与场景切换。
- 物品 / 资源的获得与消耗。
- 伏笔的开启、推进、兑现。
- 情感弧线位移。
- 副线进度。
- 角色间关系变化与新的信息边界。

## Book Information

- 书名：{title}
- 目标章数：{target_chapters} 章

## Hook Tracking Rules (enforce strictly)

- 新伏笔：仅当正文抛出一个会延续到后续章节、且具有明确兑现方向的未解问题时，才添加新 hook_id。
- 提及：已存在的伏笔被引用但无新信息 → 放入 mention 数组；不更新 lastAdvancedChapter。
- 推进：已存在的伏笔获得新事实、新证据或新关系变化 → 必须把 lastAdvancedChapter 更新为当前章节号。
- 兑现：伏笔被明确揭示或解答 → 状态设为 "resolved"。
- 延后：正文明确显示该线索被有意搁置 → 标记为 "deferred"。
- 全新的未解线索：禁止自行编造新的 hookId，候选项放入 newHookCandidates。

## Output Format (must be followed strictly)

=== POST_SETTLEMENT ===
（简明描述本章状态变化、伏笔推进、结算注意事项。）

=== RUNTIME_STATE_DELTA ===
（必须输出 JSON——无 Markdown、无额外说明。）
```json
{{
  "chapter": {chapter_example},
  "currentStatePatch": {{
    "currentLocation": "optional",
    "protagonistState": "optional",
    "currentGoal": "optional",
    "currentConstraint": "optional",
    "currentAlliances": "optional",
    "currentConflict": "optional"
  }},
  "hookOps": {{
    "upsert": [
      {{
        "hookId": "mentor-oath",
        "startChapter": 8,
        "type": "relationship",
        "status": "progressing",
        "lastAdvancedChapter": 12,
        "expectedPayoff": "reveal the truth behind the master's debt",
        "payoffTiming": "slow-burn",
        "notes": "why this chapter advances / defers / resolves the hook"
      }}
    ],
    "mention": ["hookId that was only referenced this chapter, no real advance"],
    "resolve": ["hookId that has been resolved"],
    "defer": ["hookId that must be marked as deferred"]
  }},
  "newHookCandidates": [
    {{
      "type": "mystery",
      "expectedPayoff": "where the new hook should pay off in the future",
      "payoffTiming": "near-term",
      "notes": "why this chapter produces a new unresolved question"
    }}
  ],
  "chapterSummary": {{
    "chapter": {chapter_example},
    "title": "chapter title",
    "characters": "Character1,Character2",
    "events": "one-sentence summary of key events",
    "stateChanges": "one-sentence summary of state changes",
    "hookActivity": "mentor-oath advanced",
    "mood": "tense",
    "chapterType": "main-line advancement"
  }},
  "subplotOps": [],
  "emotionalArcOps": [],
  "characterMatrixOps": [],
  "notes": []
}}
```

<safety>
- NEVER 重写完整的 truth 文件——只输出 delta 增量。
- NEVER 在 hookOps.upsert 中编造 pending_hooks 中不存在的 hookId——全新的线索必须放入 newHookCandidates。
- NEVER 在 RUNTIME_STATE_DELTA 区块之外追加任何自然语言总结——所有结算说明只能写在 POST_SETTLEMENT 区块内。
</safety>

<examples>
✅ Good（推进明确、chapter 字段正确）：
- hookOps.upsert 中 mentor-oath 的 lastAdvancedChapter 从 11 更新为 12，notes 说明"师傅临终遗言指向北方古墓"。
- 全新线索"古墓中的青铜钥匙"放入 newHookCandidates，type=mystery。

❌ Bad（编造 hookId / 推测未发生）：
- hookOps.upsert 中出现 pending_hooks 不存在的 hookId "ancient-curse"。
- notes 写"主角内心可能动摇"——这是推测，正文未确认。
</examples>

<verification>
完成结算后请自检：
1. RUNTIME_STATE_DELTA 是否是合法 JSON（无 Markdown 包裹、无自然语言注释）？
2. hookOps.upsert 中的每一个 hookId 是否都真实存在于 pending_hooks？全新的线索是否都放进了 newHookCandidates？
3. chapterSummary.chapter 是否等于当前章节号？
4. POST_SETTLEMENT 是否简明描述了本章状态变化、伏笔推进、结算注意事项？
若任一项不通过，重新输出。
</verification>

<iron_rules>
1. 只输出 delta——绝不重写整个 truth 文件。
2. 每一个 chapter 字段必须是整数。
3. hookOps.upsert 中的每一个 hookId 必须已存在于当前 hook 池中。
4. 全新的未解线索一律放入 newHookCandidates。
5. 只记录正文中真实发生的事——不推测。
6. chapterSummary.chapter 必须等于当前章节号。
</iron_rules>"#,
        title = book.title,
        target_chapters = book.target_chapters,
        chapter_example = 12,
    )
}

fn build_settler_user_message(
    chapter_number: u32,
    title: &str,
    content: &str,
    current_state: &str,
    pending_hooks: &str,
    chapter_summaries: &str,
    observations: &str,
    validation_feedback: Option<&str>,
) -> String {
    let feedback_section = match validation_feedback {
        Some(fb) if !fb.trim().is_empty() => format!(
            "\n## 校验反馈（来自上一次尝试——必须修复）\n{fb}\n",
            fb = fb
        ),
        _ => String::new(),
    };
    format!(
        r#"请结算第 {chapter_number} 章 "{title}" 的状态。

## Current State Card
{current_state}

## Current Hook Pool
{pending_hooks}

## Existing Chapter Summaries
{chapter_summaries}

## Observation Log (extracted by the Observer)
{observations}

## Chapter Prose
{content}
{feedback_section}基于以上观察日志与正文，输出 POST_SETTLEMENT 与 RUNTIME_STATE_DELTA。"#,
        chapter_number = chapter_number,
        title = title,
        current_state = current_state,
        pending_hooks = pending_hooks,
        chapter_summaries = chapter_summaries,
        observations = observations,
        content = content,
        feedback_section = feedback_section,
    )
}

/// Settler phase 输出：(post_settlement, runtime_state_delta)
fn parse_settler_output(response: &str) -> Result<(String, Option<RuntimeStateDelta>), AppError> {
    let post_settlement = extract_section(response, "POST_SETTLEMENT")
        .unwrap_or_default();

    let delta_json = extract_section(response, "RUNTIME_STATE_DELTA")
        .or_else(|| extract_json_block(response).map(|s| s.to_string()));

    let delta = match delta_json {
        Some(json_str) if !json_str.trim().is_empty() => {
            parse_delta_json(&json_str).map(Some).unwrap_or_else(|e| {
                // JSON 解析失败不阻塞流程，记录警告但继续
                tracing::warn!("settler delta parse failed: {}", e);
                None
            })
        }
        _ => None,
    };

    Ok((post_settlement, delta))
}

fn parse_delta_json(json_str: &str) -> Result<RuntimeStateDelta, AppError> {
    // 去除可能的 ```json ... ``` 包裹
    let cleaned = json_str
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str::<RuntimeStateDelta>(cleaned).map_err(|e| {
        AppError::invalid_format(format!("runtime state delta JSON invalid: {}", e))
    })
}

// ── 通用工具函数 ─────────────────────────────────────────────
// extract_section / extract_json_block / count_non_whitespace_chars 已收口到
// utils::text_parse 与 shared::utils::json，见上方 use 声明。

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_creative_output() {
        let response = r#"=== PRE_WRITE_CHECK ===
- 任务：推进 H007
- 避免：流水账

=== CHAPTER_TITLE ===
暗流

=== CHAPTER_CONTENT ===
这是正文内容。"#;
        let output = parse_creative_output(1, response).unwrap();
        assert_eq!(output.title, "暗流");
        assert_eq!(output.content, "这是正文内容。");
        assert!(output.pre_write_check.contains("推进 H007"));
    }

    #[test]
    fn rejects_missing_chapter_content() {
        let response = "=== PRE_WRITE_CHECK ===\n自检\n=== CHAPTER_TITLE ===\n标题\n";
        assert!(parse_creative_output(1, response).is_err());
    }

    #[test]
    fn parses_settler_output() {
        let response = r#"=== POST_SETTLEMENT ===
本章推进了 H007

=== RUNTIME_STATE_DELTA ===
```json
{
  "chapter": 12,
  "hookOps": {
    "upsert": [],
    "mention": ["H007"],
    "resolve": [],
    "defer": []
  }
}
```"#;
        let (post, delta) = parse_settler_output(response).unwrap();
        assert!(post.contains("H007"));
        assert!(delta.is_some());
        assert_eq!(delta.unwrap().chapter, 12);
    }

    #[test]
    fn handles_settler_without_delta() {
        let response = "=== POST_SETTLEMENT ===\n本章无状态变化";
        let (post, delta) = parse_settler_output(response).unwrap();
        assert_eq!(post, "本章无状态变化");
        assert!(delta.is_none());
    }

    #[test]
    fn extracts_json_block_without_tags() {
        let content = r#"前文
```json
{"chapter": 5}
```
后文"#;
        let json = extract_json_block(content).unwrap();
        assert_eq!(json, r#"{"chapter": 5}"#);
    }
}
