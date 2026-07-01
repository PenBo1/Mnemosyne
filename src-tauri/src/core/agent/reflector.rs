// Reflector Agent —— 状态结算专家。
//
// 在 pipeline 的 Reflect 阶段运行（Observer 之后），把 Observer 提取的事实
// 合并为状态增量（SettlementDelta），供调用方合并到 StoryState 并持久化。
//
// 设计参考：inkos 的 Observer → Reflector 两阶段模式。
// - Observer (temperature 0.5)：宁多勿少地提取事实
// - Reflector (temperature 0.3)：保守地把观察结果合并为状态增量
//
// 与 inkos 的差异：
// - inkos 把 Settler 内嵌在 WriterAgent 中（1300+ 行上帝类），Mnemosyne 保持
//   独立模块以符合"职责边界"原则。
// - inkos 用 markdown truth files，Mnemosyne 用 JSON StoryState。
// - inkos 有双轨解析（delta JSON + 全量 markdown fallback），Mnemosyne 只用
//   JSON 路径（符合"no silent fallback"规则，解析失败显式报错）。
//
// S3 升级：用强类型 `SettlementDeltaPayload`（serde derive）替代手动
// `serde_json::Value` 解析，与 inkos 的 `RuntimeStateDeltaSchema` 对齐。
// 新增 `hook_ops`（upsert/mention/resolve/defer）+ `new_hook_candidates` 字段。

use async_trait::async_trait;
use serde::Deserialize;
use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::features::story::{StoryState, HookRecord, ChapterSummary, StoryFact, HookStatus};
use super::base::{AgentContext, BaseAgent};
use super::types::AgentRole;
use super::prompts::settler_prompts;
use super::agent_identity::AgentIdentity;
use super::observer::ObservationOutput;

pub struct ReflectorAgent;

impl Default for ReflectorAgent {
    fn default() -> Self { Self }
}

impl ReflectorAgent {
    pub fn new() -> Self { Self }

    /// 对一章进行状态结算。
    ///
    /// 输入：章节内容、Observer 的提取结果、当前 StoryState。
    /// 输出：SettlementDelta，包含新增/更新的 hooks、本章摘要、新增事实。
    pub async fn reflect_chapter(
        &self,
        ctx: &AgentContext,
        chapter_number: u32,
        title: &str,
        content: &str,
        observations: &ObservationOutput,
        current_state: &StoryState,
        language: &str,
        data_dir: &DataDir,
    ) -> Result<SettlementDelta, AppError> {
        let identity = AgentIdentity::load(data_dir, "reflector");
        let task_query = format!("settle state for chapter {}", chapter_number);
        let identity_prefix = identity.build_system_prompt_with_memory(
            &ctx.memory, &task_query, ctx.skill_manager.as_deref(), ctx.user_profile.as_deref(),
        ).await;
        let system = settler_prompts::build_system_prompt(language, Some(&identity_prefix));
        let observations_text = settler_prompts::format_observations(
            &observations.facts,
            &observations.hooks_new,
            &observations.hooks_advanced,
        );
        let user = settler_prompts::build_user_prompt(
            chapter_number, title, content, &observations_text, current_state, language,
        );

        let response = self.chat(ctx, &system, &user).await?;
        let delta = parse_settlement_delta(&response.content, chapter_number, title)?;
        Ok(delta)
    }
}

#[async_trait]
impl BaseAgent for ReflectorAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Reflector
    }

    fn name(&self) -> &str {
        "reflector"
    }
}

/// 状态结算增量 —— Reflector 的输出。
///
/// 调用方负责把此 delta 合并到 StoryState 并持久化。
///
/// 字段语义：
/// - `updated_hooks`：要 upsert 的完整 hook 记录（已填充 created_at/updated_at）
/// - `hook_mentions`：仅提及（不推进）的 hook_id 列表
/// - `hook_resolves`：标记为 resolved 的 hook_id 列表
/// - `hook_defers`：标记为 deferred 的 hook_id 列表
/// - `chapter_summary`：本章摘要（追加到 StoryState.summaries）
/// - `new_facts`：新增事实（追加到 StoryState.facts）
/// - `notes`：结算备注（用于日志/调试）
#[derive(Debug, Clone, Default)]
pub struct SettlementDelta {
    pub updated_hooks: Vec<HookRecord>,
    pub hook_mentions: Vec<String>,
    pub hook_resolves: Vec<String>,
    pub hook_defers: Vec<String>,
    pub chapter_summary: Option<ChapterSummary>,
    pub new_facts: Vec<StoryFact>,
    pub notes: Vec<String>,
}

// ── 强类型 Payload（LLM 输出契约） ───────────────────────────
//
// 与 inkos 的 `RuntimeStateDeltaSchema` 对齐：
// - hook_ops: { upsert, mention, resolve, defer }
// - new_hook_candidates: 候选 hook（不带 hook_id，由 arbiter 决定是否创建）
// - chapter_summary / new_facts / notes
//
// 设计要点：
// - 所有可选字段用 `#[serde(default)]`，避免 LLM 漏字段就报错
// - 必填字段（如 hook_id、fact_id）保持必填，让 LLM 输出不全时显式失败
// - HookPayload 不含 created_at/updated_at（系统在合并时填充）
// - 用 `#[serde(rename = "type")]` 处理 Rust 关键字 `type`

#[derive(Debug, Default, Deserialize)]
struct SettlementDeltaPayload {
    #[serde(default)]
    hook_ops: HookOpsPayload,
    #[serde(default)]
    new_hook_candidates: Vec<NewHookCandidatePayload>,
    #[serde(default)]
    chapter_summary: Option<ChapterSummaryPayload>,
    #[serde(default)]
    new_facts: Vec<StoryFactPayload>,
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HookOpsPayload {
    #[serde(default)]
    upsert: Vec<HookPayload>,
    #[serde(default)]
    mention: Vec<String>,
    #[serde(default)]
    resolve: Vec<String>,
    #[serde(default)]
    defer: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HookPayload {
    hook_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    hook_type: String,
    #[serde(default)]
    start_chapter: Option<u32>,
    #[serde(default = "default_hook_status")]
    status: HookStatus,
    #[serde(default)]
    expected_payoff: String,
    #[serde(default)]
    last_advanced_chapter: u32,
    #[serde(default)]
    core_hook: bool,
    // 注：inkos 的 HookRecord 还有 notes 字段，Mnemosyne 的 HookRecord 暂无此字段。
    // serde 默认会忽略未知字段，LLM 输出 notes 不会报错。后续 arbiter 升级时
    // 再加回此字段并扩展 HookRecord schema。
}

fn default_hook_status() -> HookStatus {
    HookStatus::Open
}

#[derive(Debug, Default, Deserialize)]
struct NewHookCandidatePayload {
    #[serde(default, rename = "type")]
    hook_type: String,
    #[serde(default)]
    expected_payoff: String,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct ChapterSummaryPayload {
    #[serde(default)]
    chapter: Option<u32>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    characters: Vec<String>,
    #[serde(default)]
    events: Vec<String>,
    #[serde(default)]
    state_changes: Vec<String>,
    #[serde(default)]
    hook_activity: Vec<String>,
    #[serde(default)]
    mood: String,
    #[serde(default)]
    chapter_type: String,
}

#[derive(Debug, Deserialize)]
struct StoryFactPayload {
    fact_id: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    predicate: String,
    #[serde(default)]
    object: String,
    #[serde(default)]
    source_chapter: Option<u32>,
}

// ── S5.8: 语义校验（Zod 等价运行时校验） ─────────────────────

impl SettlementDeltaPayload {
    /// 校验 LLM 输出的语义约束，等价于 inkos `RuntimeStateDeltaSchema.parse()`。
    ///
    /// serde 只能保证结构正确（字段类型、必填字段），无法保证语义约束如：
    /// - hook_id / fact_id 非空字符串（LLM 可能输出 `""`）
    /// - hook_ops 的 mention/resolve/defer id 非空
    /// - chapter_summary.chapter 与 expected_chapter 一致
    fn validate(&self, expected_chapter: u32) -> Result<(), AppError> {
        // hook_ops.upsert[].hook_id 非空
        for h in &self.hook_ops.upsert {
            if h.hook_id.trim().is_empty() {
                return Err(AppError::internal(
                    "Reflector validation failed: hook_ops.upsert[].hook_id must be non-empty"
                ));
            }
        }
        // hook_ops.mention/resolve/defer 的 id 非空
        for id in self.hook_ops.mention.iter()
            .chain(self.hook_ops.resolve.iter())
            .chain(self.hook_ops.defer.iter())
        {
            if id.trim().is_empty() {
                return Err(AppError::internal(
                    "Reflector validation failed: hook_ops.{mention,resolve,defer}[] must be non-empty strings"
                ));
            }
        }
        // new_facts[].fact_id 非空
        for f in &self.new_facts {
            if f.fact_id.trim().is_empty() {
                return Err(AppError::internal(
                    "Reflector validation failed: new_facts[].fact_id must be non-empty"
                ));
            }
        }
        // chapter_summary.chapter 一致性（如果 LLM 输出了 chapter）
        if let Some(ref summary) = self.chapter_summary {
            if let Some(ch) = summary.chapter {
                if ch != expected_chapter {
                    return Err(AppError::internal(format!(
                        "Reflector validation failed: chapter_summary.chapter ({}) != expected ({})",
                        ch, expected_chapter
                    )));
                }
            }
        }
        Ok(())
    }
}

// ── 解析逻辑 ────────────────────────────────────────────────

/// 解析 LLM 输出为 SettlementDelta。
///
/// 失败时显式返回 AppError（不静默 fallback），符合"no silent fallback"规则。
/// 错误信息包含原始内容前 200 字符，便于调试。
fn parse_settlement_delta(
    content: &str,
    expected_chapter: u32,
    expected_title: &str,
) -> Result<SettlementDelta, AppError> {
    let json = extract_json(content)?;
    let payload: SettlementDeltaPayload = serde_json::from_str(&json).map_err(|e| {
        AppError::internal(format!(
            "Reflector output failed serde validation: {} | raw JSON (first 200 chars): {}",
            e,
            &json[..json.len().min(200)]
        ))
    })?;

    // S5.8: 语义校验（Zod 等价运行时校验）
    payload.validate(expected_chapter)?;

    Ok(payload_to_delta(payload, expected_chapter, expected_title))
}

/// 把强类型 payload 转换为 SettlementDelta（填充系统字段）。
fn payload_to_delta(
    payload: SettlementDeltaPayload,
    expected_chapter: u32,
    expected_title: &str,
) -> SettlementDelta {
    let now = chrono::Utc::now().to_rfc3339();

    // hook_ops.upsert → updated_hooks（填充 created_at/updated_at）
    let mut updated_hooks: Vec<HookRecord> = payload.hook_ops.upsert.into_iter().map(|h| {
        HookRecord {
            hook_id: h.hook_id,
            name: h.name,
            hook_type: if h.hook_type.is_empty() {
                "foreshadowing".to_string()
            } else {
                h.hook_type
            },
            start_chapter: h.start_chapter.unwrap_or(expected_chapter),
            status: h.status,
            expected_payoff: if h.expected_payoff.is_empty() {
                "mid-arc".to_string()
            } else {
                h.expected_payoff
            },
            last_advanced_chapter: h.last_advanced_chapter,
            core_hook: h.core_hook,
            created_at: now.clone(),
            updated_at: now.clone(),
        }
    }).collect();

    // new_hook_candidates → 简单合并到 updated_hooks（暂用占位 hook_id）。
    //
    // 注：完整的 inkos arbiter（admission 评估 + canonical hook id 生成 +
    // pure-restate 检测）作为后续独立任务，S3 仅做最小合并。
    for (idx, candidate) in payload.new_hook_candidates.into_iter().enumerate() {
        let hook_id = format!("hook-{}-candidate-{}", expected_chapter, idx + 1);
        updated_hooks.push(HookRecord {
            hook_id,
            name: candidate.notes.clone(),
            hook_type: if candidate.hook_type.is_empty() {
                "foreshadowing".to_string()
            } else {
                candidate.hook_type
            },
            start_chapter: expected_chapter,
            status: HookStatus::Open,
            expected_payoff: if candidate.expected_payoff.is_empty() {
                "mid-arc".to_string()
            } else {
                candidate.expected_payoff
            },
            last_advanced_chapter: expected_chapter,
            core_hook: false,
            created_at: now.clone(),
            updated_at: now.clone(),
        });
    }

    let chapter_summary = payload.chapter_summary.map(|s| ChapterSummary {
        chapter: s.chapter.unwrap_or(expected_chapter),
        title: s.title.unwrap_or_else(|| expected_title.to_string()),
        characters: s.characters,
        events: s.events,
        state_changes: s.state_changes,
        hook_activity: s.hook_activity,
        mood: s.mood,
        chapter_type: if s.chapter_type.is_empty() {
            "other".to_string()
        } else {
            s.chapter_type
        },
        created_at: now.clone(),
    });

    let new_facts = payload.new_facts.into_iter().map(|f| StoryFact {
        fact_id: f.fact_id,
        subject: f.subject,
        predicate: f.predicate,
        object: f.object,
        valid_from_chapter: expected_chapter,
        valid_until_chapter: None,
        source_chapter: f.source_chapter.unwrap_or(expected_chapter),
        created_at: now.clone(),
    }).collect();

    SettlementDelta {
        updated_hooks,
        hook_mentions: payload.hook_ops.mention,
        hook_resolves: payload.hook_ops.resolve,
        hook_defers: payload.hook_ops.defer,
        chapter_summary,
        new_facts,
        notes: payload.notes,
    }
}

/// 从 LLM 输出中提取 JSON 字符串。
///
/// 处理两种情况：
/// 1. 整个输出就是 JSON（首选）
/// 2. JSON 被 markdown 代码围栏包裹（```json ... ```）
fn extract_json(content: &str) -> Result<String, AppError> {
    let trimmed = content.trim();

    // 直接解析
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    // 尝试从代码围栏中提取
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return Ok(after_fence[..end].trim().to_string());
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        // 跳过可能的语言标识行
        let after_lang = if after_fence.starts_with('\n') {
            after_fence
        } else if let Some(nl) = after_fence.find('\n') {
            &after_fence[nl + 1..]
        } else {
            after_fence
        };
        if let Some(end) = after_lang.find("```") {
            return Ok(after_lang[..end].trim().to_string());
        }
    }

    // 尝试找到第一个 { 到最后一个 }
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    Err(AppError::internal(
        format!("Reflector output does not contain parseable JSON. First 200 chars: {}",
            &trimmed[..trimmed.len().min(200)])
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_settlement_delta_hook_ops_format() {
        // 与 inkos RuntimeStateDeltaSchema 对齐的 hook_ops 格式
        let json = r#"{
            "hook_ops": {
                "upsert": [
                    {
                        "hook_id": "hook-5-mentor-oath",
                        "name": "导师誓言",
                        "hook_type": "promise",
                        "start_chapter": 5,
                        "status": "progressing",
                        "expected_payoff": "mid-arc",
                        "last_advanced_chapter": 5,
                        "core_hook": true
                    }
                ],
                "mention": ["hook-3-old-thread"],
                "resolve": ["hook-2-resolved-mystery"],
                "defer": ["hook-4-slow-burn"]
            },
            "new_hook_candidates": [
                {
                    "type": "mystery",
                    "expected_payoff": "endgame",
                    "notes": "新的神秘线索"
                }
            ],
            "chapter_summary": {
                "chapter": 5,
                "title": "第五章 启程",
                "characters": ["主角", "导师"],
                "events": ["主角告别导师"],
                "state_changes": ["主角离开家乡"],
                "hook_activity": ["导师誓言推进"],
                "mood": "庄重",
                "chapter_type": "setup"
            },
            "new_facts": [
                {
                    "fact_id": "fact-5-protagonist-departure",
                    "subject": "主角",
                    "predicate": "离开",
                    "object": "家乡",
                    "source_chapter": 5
                }
            ],
            "notes": ["本章建立了主角的启程动机"]
        }"#;
        let delta = parse_settlement_delta(json, 5, "第五章 启程").unwrap();

        // upsert → updated_hooks
        assert_eq!(delta.updated_hooks.len(), 2); // 1 upsert + 1 candidate
        assert_eq!(delta.updated_hooks[0].hook_id, "hook-5-mentor-oath");
        assert!(matches!(delta.updated_hooks[0].status, HookStatus::Progressing));

        // new_hook_candidates → updated_hooks[1]（占位 hook_id）
        assert_eq!(delta.updated_hooks[1].hook_id, "hook-5-candidate-1");
        assert!(matches!(delta.updated_hooks[1].status, HookStatus::Open));

        // hook_ops.mention/resolve/defer
        assert_eq!(delta.hook_mentions, vec!["hook-3-old-thread"]);
        assert_eq!(delta.hook_resolves, vec!["hook-2-resolved-mystery"]);
        assert_eq!(delta.hook_defers, vec!["hook-4-slow-burn"]);

        // chapter_summary
        let summary = delta.chapter_summary.unwrap();
        assert_eq!(summary.chapter, 5);
        assert_eq!(summary.characters, vec!["主角", "导师"]);

        // new_facts
        assert_eq!(delta.new_facts.len(), 1);
        assert_eq!(delta.new_facts[0].fact_id, "fact-5-protagonist-departure");

        assert_eq!(delta.notes, vec!["本章建立了主角的启程动机"]);
    }

    #[test]
    fn test_parse_settlement_delta_minimal_payload() {
        // 最小合法 payload：只有 hook_ops（其他字段缺省）
        let json = r#"{"hook_ops": {"upsert": [], "mention": [], "resolve": [], "defer": []}}"#;
        let delta = parse_settlement_delta(json, 1, "").unwrap();
        assert!(delta.updated_hooks.is_empty());
        assert!(delta.hook_mentions.is_empty());
        assert!(delta.hook_resolves.is_empty());
        assert!(delta.hook_defers.is_empty());
        assert!(delta.chapter_summary.is_none());
        assert!(delta.new_facts.is_empty());
    }

    #[test]
    fn test_parse_settlement_delta_empty_object_uses_defaults() {
        // 空 JSON 对象：所有字段用 serde default
        let json = r#"{}"#;
        let delta = parse_settlement_delta(json, 7, "ch7").unwrap();
        assert!(delta.updated_hooks.is_empty());
        assert!(delta.chapter_summary.is_none());
    }

    #[test]
    fn test_parse_settlement_delta_markdown_fenced_json() {
        let content = "Some preamble text\n\n```json\n{\"hook_ops\": {\"upsert\": [], \"mention\": [], \"resolve\": [], \"defer\": []}}\n```\n\nTrailing text";
        let delta = parse_settlement_delta(content, 1, "").unwrap();
        assert!(delta.updated_hooks.is_empty());
        assert!(delta.chapter_summary.is_none());
    }

    #[test]
    fn test_parse_settlement_delta_invalid_json_returns_error() {
        let content = "this is not json at all";
        let result = parse_settlement_delta(content, 1, "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not contain parseable JSON"));
    }

    #[test]
    fn test_parse_settlement_delta_serde_validation_error() {
        // 合法 JSON 但 schema 错误：upsert 中 hook_id 缺失（必填字段）
        let json = r#"{
            "hook_ops": {
                "upsert": [
                    {"name": "missing hook_id"}
                ]
            }
        }"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed serde validation"));
        assert!(err.contains("hook_id"));
    }

    #[test]
    fn test_parse_settlement_delta_wrong_type_in_array() {
        // mention 应为字符串数组，给数字数组应失败
        let json = r#"{
            "hook_ops": {
                "mention": [123, 456]
            }
        }"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed serde validation"));
    }

    #[test]
    fn test_parse_settlement_delta_hook_defaults() {
        // hook 必填字段只有 hook_id；其他字段缺省时使用合理默认值
        let json = r#"{
            "hook_ops": {
                "upsert": [
                    {"hook_id": "hook-min"}
                ]
            }
        }"#;
        let delta = parse_settlement_delta(json, 9, "").unwrap();
        assert_eq!(delta.updated_hooks.len(), 1);
        let hook = &delta.updated_hooks[0];
        assert_eq!(hook.hook_id, "hook-min");
        assert_eq!(hook.hook_type, "foreshadowing");
        assert_eq!(hook.start_chapter, 9); // 用 expected_chapter
        assert!(matches!(hook.status, HookStatus::Open));
        assert_eq!(hook.expected_payoff, "mid-arc");
        assert_eq!(hook.last_advanced_chapter, 0);
        assert!(!hook.core_hook);
    }

    #[test]
    fn test_parse_settlement_delta_fact_defaults() {
        // fact 必填字段只有 fact_id；source_chapter 缺省时用 expected_chapter
        let json = r#"{
            "new_facts": [
                {"fact_id": "fact-min"}
            ]
        }"#;
        let delta = parse_settlement_delta(json, 11, "").unwrap();
        assert_eq!(delta.new_facts.len(), 1);
        let fact = &delta.new_facts[0];
        assert_eq!(fact.fact_id, "fact-min");
        assert_eq!(fact.source_chapter, 11);
        assert_eq!(fact.valid_from_chapter, 11);
        assert!(fact.valid_until_chapter.is_none());
    }

    #[test]
    fn test_extract_json_direct() {
        let result = extract_json(r#"{"key": "value"}"#).unwrap();
        assert!(result.contains(r#""key": "value""#));
    }

    #[test]
    fn test_extract_json_from_braces() {
        let content = "Here is the delta: {\"a\": 1} done.";
        let result = extract_json(content).unwrap();
        assert_eq!(result, r#"{"a": 1}"#);
    }

    // ── S5.8: 语义校验测试 ──────────────────────────────────

    #[test]
    fn test_validate_empty_hook_id_rejected() {
        let json = r#"{"hook_ops": {"upsert": [{"hook_id": ""}]}}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("hook_id must be non-empty"));
    }

    #[test]
    fn test_validate_whitespace_hook_id_rejected() {
        let json = r#"{"hook_ops": {"upsert": [{"hook_id": "   "}]}}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hook_id must be non-empty"));
    }

    #[test]
    fn test_validate_empty_mention_id_rejected() {
        let json = r#"{"hook_ops": {"mention": ["valid-id", ""]}}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be non-empty strings"));
    }

    #[test]
    fn test_validate_empty_resolve_id_rejected() {
        let json = r#"{"hook_ops": {"resolve": [""]}}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be non-empty strings"));
    }

    #[test]
    fn test_validate_empty_defer_id_rejected() {
        let json = r#"{"hook_ops": {"defer": [""]}}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be non-empty strings"));
    }

    #[test]
    fn test_validate_empty_fact_id_rejected() {
        let json = r#"{"new_facts": [{"fact_id": ""}]}"#;
        let result = parse_settlement_delta(json, 1, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("fact_id must be non-empty"));
    }

    #[test]
    fn test_validate_chapter_summary_mismatch_rejected() {
        let json = r#"{"chapter_summary": {"chapter": 3, "title": "ch3"}}"#;
        let result = parse_settlement_delta(json, 5, "ch5");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("chapter_summary.chapter (3) != expected (5)"));
    }

    #[test]
    fn test_validate_chapter_summary_match_accepted() {
        let json = r#"{"chapter_summary": {"chapter": 5, "title": "ch5"}}"#;
        let result = parse_settlement_delta(json, 5, "ch5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_chapter_summary_without_chapter_accepted() {
        // chapter_summary 不含 chapter 字段时不校验一致性
        let json = r#"{"chapter_summary": {"title": "ch5"}}"#;
        let result = parse_settlement_delta(json, 5, "ch5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_payload_passes() {
        let json = r#"{
            "hook_ops": {
                "upsert": [{"hook_id": "hook-1-test"}],
                "mention": ["hook-2"],
                "resolve": ["hook-3"],
                "defer": ["hook-4"]
            },
            "new_facts": [{"fact_id": "fact-1"}],
            "chapter_summary": {"chapter": 5, "title": "ch5"}
        }"#;
        let result = parse_settlement_delta(json, 5, "ch5");
        assert!(result.is_ok());
    }
}
