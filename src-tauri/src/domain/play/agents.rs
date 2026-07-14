// Play 4-agent 流水线。
//
// - PlayActionInterpreterAgent：自然语言 → PlayActionIntent（永不抛错，降级为 do）
// - PlayWorldMutatorAgent：动作+上下文 → PlayMutation（失败降级为 blocked=true）
// - PlaySceneRendererAgent：渲染场景散文（永不抛错，3 次重试 + 降级为原始 prose）
// - PlaySceneReconcilerAgent：补抓遗漏实体（失败降级为空 mutation）
//
// 每个 agent 通过 AgentEngine.prompt_once 调用 LLM。
// 注意：prompt_once 不支持 temperature 参数，温度引导写入 system prompt。

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::types::{PlayActionIntent, PlayActionKind, PlayMutation};

const MAX_RENDER_RETRIES: u32 = 3;

/// 动作归一 agent：把玩家自然语言归一成 5 类动作。
pub struct PlayActionInterpreterAgent;

impl PlayActionInterpreterAgent {
    pub async fn interpret(
        engine: &AgentEngine,
        input: &str,
        language: &str,
    ) -> Result<PlayActionIntent, AppError> {
        let system_prompt = build_interpreter_prompt(language);
        let user_message = format!("玩家输入：\n{}", input);

        match engine.prompt_once(&system_prompt, &user_message).await {
            Ok(raw) => {
                if let Some(json_str) = extract_json_block(&raw) {
                    if let Ok(intent) = serde_json::from_str::<PlayActionIntent>(json_str) {
                        return Ok(intent);
                    }
                }
                // 解析失败：降级为 do 动作（永不抛错）
                tracing::warn!("PlayActionInterpreter 解析失败，降级为 do");
                Ok(degrade_intent(input))
            }
            Err(e) => {
                // LLM 调用失败：降级为 do 动作（永不抛错）
                tracing::warn!(error = %e, "PlayActionInterpreter LLM 失败，降级为 do");
                Ok(degrade_intent(input))
            }
        }
    }
}

fn degrade_intent(input: &str) -> PlayActionIntent {
    PlayActionIntent {
        action_kind: PlayActionKind::Do,
        target_entity_label: None,
        target_location_label: None,
        intent: truncate(input, 200),
        manner: None,
        risk: None,
        ambiguity: Some("interpreter_degraded".to_string()),
        secondary_actions: Vec::new(),
    }
}

/// 世界变更 agent：根据动作+上下文起草 PlayMutation。
pub struct PlayWorldMutatorAgent;

impl PlayWorldMutatorAgent {
    pub async fn propose_mutation(
        engine: &AgentEngine,
        action: &PlayActionIntent,
        context: &str,
        language: &str,
    ) -> Result<PlayMutation, AppError> {
        let system_prompt = build_mutator_prompt(language);
        let user_message = format!(
            "玩家动作：\n{}\n\n当前世界状态：\n{}",
            serde_json::to_string_pretty(action).unwrap_or_default(),
            context
        );

        match engine.prompt_once(&system_prompt, &user_message).await {
            Ok(raw) => {
                if let Some(json_str) = extract_json_block(&raw) {
                    if let Ok(mutation) = serde_json::from_str::<PlayMutation>(json_str) {
                        return Ok(mutation);
                    }
                }
                // 解析失败：降级为 blocked=true
                tracing::warn!("PlayWorldMutator 解析失败，降级为 blocked");
                Ok(blocked_mutation(action, "mutation_parse_failed"))
            }
            Err(e) => {
                tracing::warn!(error = %e, "PlayWorldMutator LLM 失败，降级为 blocked");
                Ok(blocked_mutation(action, "llm_failed"))
            }
        }
    }
}

fn blocked_mutation(action: &PlayActionIntent, reason: &str) -> PlayMutation {
    PlayMutation {
        action_kind: Some(action.action_kind.clone()),
        summary: Some(format!("动作被阻塞：{}", reason)),
        blocked: true,
        blocked_reason: Some(reason.to_string()),
        ..Default::default()
    }
}

/// 场景渲染 agent：渲染场景散文。
/// 永不抛错：3 次重试，全部失败则降级为原始 prose。
pub struct PlaySceneRendererAgent;

impl PlaySceneRendererAgent {
    pub async fn render(
        engine: &AgentEngine,
        state: &str,
        action: &PlayActionIntent,
        mode: &str,
        language: &str,
    ) -> Result<String, AppError> {
        let system_prompt = build_renderer_prompt(language, mode);
        let user_message = format!(
            "当前状态：\n{}\n\n玩家动作：\n{}",
            state,
            serde_json::to_string_pretty(action).unwrap_or_default()
        );

        let mut last_err: Option<AppError> = None;
        for attempt in 0..MAX_RENDER_RETRIES {
            match engine.prompt_once(&system_prompt, &user_message).await {
                Ok(text) if !text.trim().is_empty() => return Ok(text),
                Ok(_) => {
                    tracing::warn!(attempt, "PlaySceneRenderer 返回空，重试");
                }
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "PlaySceneRenderer 失败，重试");
                    last_err = Some(e);
                }
            }
        }
        // 全部失败：降级为原始 prose
        tracing::warn!("PlaySceneRenderer 全部重试失败，降级为原始 prose");
        let _ = last_err;
        Ok(fallback_prose(action, language))
    }
}

fn fallback_prose(action: &PlayActionIntent, language: &str) -> String {
    let verb = match language {
        "en" => "You ",
        _ => "",
    };
    let action_str = match &action.intent {
        s if !s.is_empty() => s.clone(),
        _ => match action.action_kind {
            PlayActionKind::Look => "环顾四周".to_string(),
            PlayActionKind::Say => "开口说话".to_string(),
            PlayActionKind::Move => "移动".to_string(),
            PlayActionKind::Wait => "等待".to_string(),
            PlayActionKind::Do => "行动".to_string(),
        },
    };
    format!("{}{}", verb, action_str)
}

/// 场景对账 agent：补抓遗漏实体。失败降级为空 mutation。
pub struct PlaySceneReconcilerAgent;

impl PlaySceneReconcilerAgent {
    pub async fn reconcile(
        engine: &AgentEngine,
        scene_text: &str,
        mutation: &PlayMutation,
        language: &str,
    ) -> Result<PlayMutation, AppError> {
        let system_prompt = build_reconciler_prompt(language);
        let user_message = format!(
            "场景正文：\n{}\n\n当前 mutation：\n{}",
            scene_text,
            serde_json::to_string_pretty(mutation).unwrap_or_default()
        );

        match engine.prompt_once(&system_prompt, &user_message).await {
            Ok(raw) => {
                if let Some(json_str) = extract_json_block(&raw) {
                    if let Ok(extra) = serde_json::from_str::<PlayMutation>(json_str) {
                        return Ok(extra);
                    }
                }
                tracing::warn!("PlaySceneReconciler 解析失败，降级为空 mutation");
                Ok(PlayMutation::default())
            }
            Err(e) => {
                tracing::warn!(error = %e, "PlaySceneReconciler LLM 失败，降级为空 mutation");
                Ok(PlayMutation::default())
            }
        }
    }
}

// ── prompt 构建 ────────────────────────────────────────

fn build_interpreter_prompt(language: &str) -> String {
    let lang_hint = lang_instruction(language);
    format!(
        r#"你是互动小说动作归一器。把玩家自然语言归一为恰好一个 PlayActionIntent。

动作类型（actionKind，必填，5 选 1）：
- look：观察/查看/调查
- say：说话/对话/询问
- move：移动/前往/离开
- do：其他具体动作（操作、拾取、使用、攻击等）
- wait：等待/原地不动

输出严格 JSON：
{{
  "actionKind": "look|say|move|do|wait",
  "targetEntityLabel": "目标实体标签（可选）",
  "targetLocationLabel": "目标位置标签（可选）",
  "intent": "用一句话描述意图（必填）",
  "manner": "方式/态度（可选）",
  "risk": "风险等级 low/medium/high（可选）",
  "ambiguity": "歧义说明（可选）",
  "secondaryActions": ["次要动作字符串"]
}}

约束：
- 只输出 JSON，不要任何解释文字
- intent 必填且不超过 200 字
- 永远输出一个完整 JSON 对象，即使输入模糊也降级为 do
{lang_hint}"#
    )
}

fn build_mutator_prompt(language: &str) -> String {
    let lang_hint = lang_instruction(language);
    format!(
        r#"你是互动小说世界变更器。根据玩家动作和当前世界状态，起草 PlayMutation。

世界契约：
- 世界自走：NPC 有自主意志，会按自身动机推进剧情，不依赖玩家
- 不催逼玩家：不强制玩家做选择，不输出菜单式选项作为正文
- 玩家固定 ID：actor_player，引用玩家实体一律用此 ID
- 持有物判定：玩家持有物通过 edge_type=holding 的边表示，目标必须是物理实体

PlayMutation 结构（严格 JSON）：
{{
  "actionKind": "look|say|move|do|wait",
  "summary": "本回合一句话摘要",
  "timeAdvance": {{"elapsed": "10 分钟", "anchor": "可选时间锚点", "rationale": "可选"}},
  "entitiesUpsert": [{{"id":"actor_player","label":"玩家","entityType":"actor","summary":"","physical":true,"attributes":{{}}}}],
  "edgesUpsert": [{{"id":"edge_1","fromId":"actor_player","toId":"item_key","edgeType":"holding","role":null,"attributes":{{}}}}],
  "edgesExpire": [{{"edgeId":"edge_xxx","reason":"过期"}}],
  "stateSlotsUpsert": [{{"id":"hp_player","ownerEntityId":"actor_player","slotKind":"resource","key":"hp","value":8,"min":0,"max":10,"unit":null}}],
  "evidenceTransitions": [{{"claimId":"claim_1","fromStatus":"unknown","toStatus":"hinted"}}],
  "blocked": false,
  "blockedReason": null,
  "notes": []
}}

证据状态枚举（不可倒退）：unknown < hinted < seen < collected < verified < weaponized < exposed < exhausted
实体类型枚举：actor/location/item/evidence/clue/claim/proof_chain/organization/rule/scene/event
状态槽类型枚举：resource/relation/pressure/clue/evidence/flag/timer

约束：
- 只输出 JSON
- 如动作不可执行，设 blocked=true 并填 blockedReason
- 不输出正文散文（那是渲染 agent 的事）
{lang_hint}"#
    )
}

fn build_renderer_prompt(language: &str, mode: &str) -> String {
    let lang_hint = lang_instruction(language);
    let mode_hint = match mode {
        "guided" => "引导模式：可在结尾提供 1-3 个建议动作，但以散文形式自然给出，不要编号菜单。",
        _ => "开放模式：不提供菜单式选项，世界自然推进。",
    };
    format!(
        r#"你是互动小说场景渲染器。根据当前状态和玩家动作，渲染一段场景正文。

渲染原则：
- 第二人称叙事（"你..."）
- 沉浸式散文，不分点列举
- 展现世界对玩家动作的反馈与后果
- 不代替玩家做决定，不催逼选择
- 不输出 JSON，只输出正文
{mode_hint}
{lang_hint}

直接输出场景正文，不要任何前缀说明。"#,
    )
}

fn build_reconciler_prompt(language: &str) -> String {
    let lang_hint = lang_instruction(language);
    format!(
        r#"你是互动小说对账器。对比场景正文与当前 mutation，补抓遗漏的实体/边/状态槽。

只输出一个 PlayMutation JSON，包含需要在原有 mutation 之上补充的增量条目（entitiesUpsert/edgesUpsert/stateSlotsUpsert/evidenceTransitions）。
其余字段留空或默认值即可。
不重复已有条目。只输出 JSON。
{lang_hint}"#
    )
}

fn lang_instruction(language: &str) -> String {
    match language {
        "en" => "用英文输出。".to_string(),
        _ => "用中文输出。".to_string(),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    crate::shared::utils::truncate_string(s, max_len)
}
