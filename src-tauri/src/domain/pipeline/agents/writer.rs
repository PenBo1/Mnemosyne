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

    // ── Phase 3: Settler — merge into truth files ──
    let settler_system = build_settler_system_prompt(book);
    let settler_user = build_settler_user_message(
        chapter_number,
        &creative.title,
        &creative.content,
        ctx,
        &observations,
    );
    let settler_response = engine.prompt_once(&settler_system, &settler_user).await?;
    let (post_settlement, runtime_state_delta) = parse_settler_output(&settler_response)?;

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

// ── Phase 1: Creative ───────────────────────────────────────

fn build_creative_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"你是一位专业的网络小说作家。你为{platform}平台写作。

## 核心规则

1. 以简体中文工作，句子长短交替，段落适合手机阅读（3-5行/段）
2. 每章必须有明确的推进目标，不允许流水账
3. 场景描写必须有具体可视化感官细节（五感具体化）
4. 角色行为由"过往经历 + 当前利益 + 性格底色"共同驱动，禁止反派突然降智、主角突然圣母
5. 不同角色说话方式必须不同，禁止"众人齐声惊呼"
6. 情绪用动作外化（不写"他感到愤怒"，写动作）。价值观通过行为传达
7. 坏事叠坏事，每层比上一层过分
8. 每章章尾留钩
9. 章节备忘（chapter_memo）中的"当前任务"、"不要做"、"章尾必须发生的改变"必须落实到正文
10. 伏笔账本中 advance/resolve 的每个 hook_id 都必须在正文里有具体可定位的兑现段（≥60字）

## 字数治理
- 目标字数：{target_words}字
- 允许区间：{soft_min}-{soft_max}字

## 输出格式（严格遵守）

先输出写作自检表，再写正文。只需输出三个区块：

=== PRE_WRITE_CHECK ===
（写作前自检：列出本章要完成的任务、要兑现的伏笔、要避免的事项）
=== CHAPTER_TITLE ===
（章节标题，不要书名号，不要章号前缀）
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
## 章节备忘
{chapter_memo}

## 当前状态卡
{current_state}

## 伏笔池
{pending_hooks}

## 章节摘要（历史章节压缩）
{chapter_summaries}

## 最近章节
{recent_chapters}

## 世界观设定
{story_frame}

## 卷纲
{volume_map}

## 规则卡
{book_rules}

## 文风指南
{style_guide}

请基于以上信息，先输出 PRE_WRITE_CHECK 自检表，再写正文。"#,
        chapter_number = chapter_number,
        external = external,
        chapter_memo = ctx.chapter_memo,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        recent_chapters = if ctx.recent_chapters.is_empty() {
            "(这是第一章，无前文)".to_string()
        } else {
            ctx.recent_chapters.clone()
        },
        story_frame = ctx.story_frame,
        volume_map = ctx.volume_map,
        book_rules = ctx.book_rules,
        style_guide = if ctx.style_guide.is_empty() {
            "(无文风指南)"
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
    r#"你是一个事实提取专家。阅读章节正文，提取每一个可观察到的事实变化。

## 提取类别

1. **角色行为**：谁做了什么，对谁，为什么
2. **位置变化**：谁去了哪里，从哪里来
3. **资源变化**：获得、失去、消耗了什么，具体数量
4. **关系变化**：新相遇、信任/不信任转变、结盟、背叛
5. **情绪变化**：角色情绪从X到Y，触发事件是什么
6. **信息流动**：谁知道了什么新信息，谁仍然不知情
7. **剧情线索**：新埋下的悬念、已有线索的推进、线索的解答
8. **时间推进**：过了多少时间，提到的时间标记
9. **身体状态**：受伤、恢复、疲劳、战力变化

## 规则

- 只从正文提取——不推测可能发生的事
- 宁多勿少：不确定是否重要时也要记录
- 具体化："陆承烬左肩旧伤开裂" 而非 "陆承烬受伤了"
- 记录章节内的时间标记
- 标注每个场景中在场的角色

## 输出格式

=== OBSERVATIONS ===

[角色行为]
- <角色名>: <行为/状态变化> (场景: <地点>)

[位置变化]
- <角色> 从 <A> 到 <B>

[资源变化]
- <角色> 获得/失去 <物品> (数量: <n>)

[关系变化]
- <角色A> → <角色B>: <变化描述>

[情绪变化]
- <角色>: <之前> → <之后> (触发: <事件>)

[信息流动]
- <角色> 得知: <事实> (来源: <途径>)
- <角色> 仍不知: <事实>

[剧情线索]
- 新埋: <描述>
- 推进: <已有线索> — <进展>
- 回收: <线索> — <解答>

[时间]
- <时间标记、时长>

[身体状态]
- <角色>: <受伤/恢复/疲劳/战力变化>"#
        .to_string()
}

fn build_observer_user_message(chapter_number: u32, title: &str, content: &str) -> String {
    format!(
        "请提取第{}章「{}」中的所有事实：\n\n{}",
        chapter_number, title, content
    )
}

// ── Phase 3: Settler ────────────────────────────────────────

fn build_settler_system_prompt(book: &BookConfig) -> String {
    format!(
        r#"你是状态追踪分析师。给定新章节正文和当前 truth 文件，你的任务是产出更新后的 truth 文件。

## 工作模式

你不是在写作。你的任务是：
1. 仔细阅读正文，提取所有状态变化
2. 基于"当前追踪文件"做增量更新
3. 严格按照 === TAG === 格式输出

## 分析维度

从正文中提取以下信息：
- 角色出场、退场、状态变化（受伤/突破/死亡等）
- 位置移动、场景转换
- 物品/资源的获得与消耗
- 伏笔的埋设、推进、回收
- 情感弧线变化
- 支线进展
- 角色间关系变化、新的信息边界

## 书籍信息

- 标题：{title}
- 目标章数：{target_chapters}章

## 伏笔追踪规则（严格执行）

- 新伏笔：只有当正文中出现一个会延续到后续章节、且有具体回收方向的未解问题时，才新增 hook_id
- 提及伏笔：已有伏笔被提到，但没有新增信息 → 放入 mention 数组，不要更新最近推进
- 推进伏笔：已有伏笔出现了新事实、证据、关系变化 → 必须更新 lastAdvancedChapter 为当前章节号
- 回收伏笔：伏笔被明确揭示、解决 → 状态改为"已回收"
- 延后伏笔：正文明确显示该线被主动搁置 → 标注"延后"
- brand-new unresolved thread：不要直接发明新的 hookId。把候选放进 newHookCandidates

## 输出格式（必须严格遵循）

=== POST_SETTLEMENT ===
（简要说明本章有哪些状态变动、伏笔推进、结算注意事项）

=== RUNTIME_STATE_DELTA ===
（必须输出 JSON，不要输出 Markdown，不要加解释）
```json
{{
  "chapter": {chapter_example},
  "currentStatePatch": {{
    "currentLocation": "可选",
    "protagonistState": "可选",
    "currentGoal": "可选",
    "currentConstraint": "可选",
    "currentAlliances": "可选",
    "currentConflict": "可选"
  }},
  "hookOps": {{
    "upsert": [
      {{
        "hookId": "mentor-oath",
        "startChapter": 8,
        "type": "relationship",
        "status": "progressing",
        "lastAdvancedChapter": 12,
        "expectedPayoff": "揭开师债真相",
        "payoffTiming": "slow-burn",
        "notes": "本章为何推进/延后/回收"
      }}
    ],
    "mention": ["本章只是被提到、没有真实推进的 hookId"],
    "resolve": ["已回收的 hookId"],
    "defer": ["需要标记延后的 hookId"]
  }},
  "newHookCandidates": [
    {{
      "type": "mystery",
      "expectedPayoff": "新伏笔未来要回收到哪里",
      "payoffTiming": "near-term",
      "notes": "本章为什么会形成新的未解问题"
    }}
  ],
  "chapterSummary": {{
    "chapter": {chapter_example},
    "title": "本章标题",
    "characters": "角色1,角色2",
    "events": "一句话概括关键事件",
    "stateChanges": "一句话概括状态变化",
    "hookActivity": "mentor-oath advanced",
    "mood": "紧绷",
    "chapterType": "主线推进"
  }},
  "subplotOps": [],
  "emotionalArcOps": [],
  "characterMatrixOps": [],
  "notes": []
}}
```

## 铁律

1. 只输出增量，不要重写完整 truth files
2. 所有章节号字段都必须是整数
3. hookOps.upsert 里只能写"当前伏笔池里已经存在"的 hookId
4. brand-new unresolved thread 一律写进 newHookCandidates
5. 只记录正文中实际发生的事，不要推断
6. chapterSummary.chapter 必须等于当前章节号"#,
        title = book.title,
        target_chapters = book.target_chapters,
        chapter_example = 12,
    )
}

fn build_settler_user_message(
    chapter_number: u32,
    title: &str,
    content: &str,
    ctx: &WriterContext,
    observations: &str,
) -> String {
    format!(
        r#"请为第 {chapter_number} 章「{title}」结算状态。

## 当前状态卡
{current_state}

## 当前伏笔池
{pending_hooks}

## 已有章节摘要
{chapter_summaries}

## 观察日志（由 Observer 提取）
{observations}

## 章节正文
{content}

基于以上观察日志和正文，输出 POST_SETTLEMENT 和 RUNTIME_STATE_DELTA。"#,
        chapter_number = chapter_number,
        title = title,
        current_state = ctx.current_state,
        pending_hooks = ctx.pending_hooks,
        chapter_summaries = ctx.chapter_summaries,
        observations = observations,
        content = content,
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
