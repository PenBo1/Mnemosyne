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

use super::super::state::types::RuntimeStateDelta;
use super::super::types::BookConfig;

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

    let word_count = count_chars(&creative.content);

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
You are a professional web-fiction novelist writing for the {platform} platform.
</identity>

<core_rules>
1. Write in Simplified Chinese. Alternate long and short sentences. Paragraphs should suit mobile reading (3-5 lines per paragraph).
2. Every chapter must have a clear advancement goal. No chronicle drift (流水账).
3. Scene description must include concrete visualizable sensory detail (five senses made concrete).
4. Character behavior is driven jointly by past experience, current interest, and core personality. Never let the antagonist suddenly dumb down, never let the protagonist suddenly turn saintly.
5. Different characters must speak differently. No "the crowd gasped in unison."
6. Externalize emotion through action (do not write "he felt angry" — write the action). Convey values through behavior.
7. Stack bad on bad. Each layer must be worse than the last.
8. End every chapter on a hook.
9. The chapter memo's "Current Task", "Do Not", and "Changes That Must Occur at Chapter End" must be carried out in the prose.
10. Every hook_id listed in the hook ledger's advance/resolve must have a concrete, locatable payoff passage in the prose (≥ 60 characters).
</core_rules>

## Word-Count Governance
- Target word count: {target_words} words
- Acceptable range: {soft_min}-{soft_max} words

## Output Format (strict)

Emit the pre-write self-check first, then the prose. Output exactly three blocks:

=== PRE_WRITE_CHECK ===
(pre-write self-check: list this chapter's tasks, the hooks to pay off, and the items to avoid)
=== CHAPTER_TITLE ===
(chapter title — no book-title marks, no "Chapter N" prefix)
=== CHAPTER_CONTENT ===
(prose content)"#,
        platform = format!("{:?}", book.platform).to_lowercase(),
        target_words = book.chapter_word_count,
        soft_min = (book.chapter_word_count as f32 * 0.85) as u32,
        soft_max = (book.chapter_word_count as f32 * 1.15) as u32,
    )
}

fn build_creative_user_message(_book: &BookConfig, chapter_number: u32, ctx: &WriterContext) -> String {
    let external = match &ctx.external_context {
        Some(e) if !e.trim().is_empty() => format!("\n## This Chapter's User Instructions (highest priority)\n{}\n", e),
        _ => String::new(),
    };

    format!(
        r#"Continue with Chapter {chapter_number}.
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

Based on the information above, first emit the PRE_WRITE_CHECK self-check, then write the prose."#,
        chapter_number = chapter_number,
        external = external,
        chapter_memo = ctx.chapter_memo,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        recent_chapters = if ctx.recent_chapters.is_empty() {
            "(This is the first chapter; there is no prior text.)".to_string()
        } else {
            ctx.recent_chapters.clone()
        },
        story_frame = ctx.story_frame,
        volume_map = ctx.volume_map,
        book_rules = ctx.book_rules,
        style_guide = if ctx.style_guide.is_empty() {
            "(No style guide.)"
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
You are a fact-extraction specialist. You read chapter prose and surface every observable change that occurred within it.
</identity>

<responsibilities>
- Extract every factual change the chapter actually commits to on the page — nothing inferred, nothing hypothetical.
- Cover nine categories in lockstep: character behavior, location, resources, relationships, emotion, information flow, plot threads, time progression, and physical state.
- Make each entry specific and locatable so the settler agent can update truth files without re-reading the chapter.
</responsibilities>

## Extraction Categories

1. **Character behavior**: Who did what, to whom, and why.
2. **Location changes**: Who went where, and from where.
3. **Resource changes**: What was gained, lost, or consumed — with exact quantities.
4. **Relationship changes**: New meetings, trust/distrust shifts, alliances formed, betrayals.
5. **Emotional changes**: A character's affect moved from X to Y, triggered by which event.
6. **Information flow**: Who learned a new fact, and through what channel; who still does not know.
7. **Plot threads**: Newly planted hooks, advances on existing threads, resolutions of prior threads.
8. **Time progression**: How much time elapsed, any in-text time markers mentioned.
9. **Physical state**: Injuries, recoveries, fatigue, combat-power fluctuations.

<rules>
- Extract only from the prose — never speculate about what might have happened off-page.
- Err on the side of inclusion: when uncertain whether something matters, record it.
- Be concrete: write "the old wound on Lu Chengjin's left shoulder reopened" rather than "Lu Chengjin got hurt."
- Capture every in-chapter time marker explicitly.
- Note which characters are present in each scene.
</rules>

## Output Format

=== OBSERVATIONS ===

[Character Behavior]
- <Character>: <Action / state change> (Scene: <Location>)

[Location Changes]
- <Character> from <A> to <B>

[Resource Changes]
- <Character> gained / lost <Item> (Quantity: <n>)

[Relationship Changes]
- <Character A> → <Character B>: <Change description>

[Emotional Changes]
- <Character>: <Before> → <After> (Trigger: <Event>)

[Information Flow]
- <Character> learned: <Fact> (Source: <Channel>)
- <Character> still unaware of: <Fact>

[Plot Threads]
- Planted: <Description>
- Advanced: <Existing thread> — <Progress>
- Resolved: <Thread> — <Resolution>

[Time]
- <Time markers, elapsed duration>

[Physical State]
- <Character>: <Injury / recovery / fatigue / combat-power change>"#
        .to_string()
}

fn build_observer_user_message(chapter_number: u32, title: &str, content: &str) -> String {
    format!(
        "Extract every observable fact from Chapter {chapter_number} \"{title}\":\n\n{content}",
        chapter_number = chapter_number,
        title = title,
        content = content,
    )
}

// ── Phase 3: Settler ────────────────────────────────────────

fn build_settler_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"<identity>
You are a state-tracking analyst. Given the new chapter's prose and the current truth files, your job is to produce the updated truth-file delta.
</identity>

## Operating Mode

You are not writing fiction. Your task is:
1. Read the prose carefully and extract every state change.
2. Apply incremental updates on top of the "current tracking files."
3. Emit strictly in the `=== TAG ===` format defined below.

## Analysis Dimensions

From the prose, extract:
- Character entrances, exits, and state changes (injury / breakthrough / death, etc.).
- Location moves and scene transitions.
- Acquisition and consumption of items / resources.
- Planting, advancing, and resolving of hooks (伏笔).
- Emotional-arc shifts.
- Subplot progress.
- Inter-character relationship changes and new information boundaries.

## Book Information

- Title: {title}
- Target chapter count: {target_chapters} chapters

## Hook Tracking Rules (enforce strictly)

- New hook: only add a new hook_id when the prose raises an unresolved question that carries into later chapters and has a concrete payoff direction.
- Mentioned hook: an existing hook is referenced but gains no new information → put it in the mention array; do not update lastAdvancedChapter.
- Advanced hook: an existing hook acquires new facts, evidence, or relationship change → you must update lastAdvancedChapter to the current chapter number.
- Resolved hook: a hook is explicitly revealed or solved → set status to "resolved".
- Deferred hook: the prose clearly shows the thread is being shelved on purpose → mark it as "deferred".
- Brand-new unresolved thread: never invent a new hookId on your own. Put candidates into newHookCandidates.

## Output Format (must be followed strictly)

=== POST_SETTLEMENT ===
(Concisely describe this chapter's state changes, hook advances, and any settlement caveats.)

=== RUNTIME_STATE_DELTA ===
(Must output JSON — no Markdown, no extra commentary.)
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

<iron_rules>
1. Emit only the delta — never rewrite the full truth files.
2. Every chapter-number field must be an integer.
3. Every hookId in hookOps.upsert must already exist in the current hook pool.
4. Brand-new unresolved threads always go into newHookCandidates.
5. Record only what actually happened in the prose — do not infer.
6. chapterSummary.chapter must equal the current chapter number.
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
            "\n## Validation Feedback (from previous attempt — must fix)\n{fb}\n",
            fb = fb
        ),
        _ => String::new(),
    };
    format!(
        r#"Settle the state for Chapter {chapter_number} "{title}".

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
{feedback_section}Based on the observation log and prose above, emit POST_SETTLEMENT and RUNTIME_STATE_DELTA."#,
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

/// 从 === TAG === 格式中提取区块内容
fn extract_section(content: &str, tag: &str) -> Option<String> {
    let marker = format!("=== {} ===", tag);
    let start = content.find(&marker)?;
    let content_start = start + marker.len();

    // 找下一个 === TAG === 或文本结尾
    let remaining = &content[content_start..];
    let end = remaining
        .find("\n=== ")
        .map(|pos| content_start + pos)
        .unwrap_or(content.len());

    Some(content[content_start..end].trim().to_string())
}

/// 从 ```json ... ``` 代码块中提取 JSON
fn extract_json_block(content: &str) -> Option<&str> {
    let start_marker = "```json";
    let start = content.find(start_marker)?;
    let json_start = start + start_marker.len();
    let end = content[json_start..].find("```")?;
    Some(content[json_start..json_start + end].trim())
}

/// 统计字数（中文按字符数，英文按空格分词）
fn count_chars(content: &str) -> u32 {
    // 简化版：统计非空白字符数
    content.chars().filter(|c| !c.is_whitespace()).count() as u32
}

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
