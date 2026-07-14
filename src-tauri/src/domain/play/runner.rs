// Play 主控：4-agent 流水线编排。
//
// step 流程：
// 1. interpret  归一玩家动作 → PlayActionIntent
// 2. mutate     起草 PlayMutation（context 来自当前 DB 快照）
// 3. render     渲染场景正文（在 commit 前完成，保证失败不污染 DB）
// 4. reconcile  补抓遗漏实体
// 5. merge      把 reconcile 增量并入主 mutation
// 6. apply       提交到 DB（apply_play_mutation）

use std::path::Path;

use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;

use super::agents::{
    PlayActionInterpreterAgent, PlaySceneReconcilerAgent, PlaySceneRendererAgent,
    PlayWorldMutatorAgent,
};
use super::db::PlayDb;
use super::reducer::{apply_play_mutation, seed_play_graph};
use super::types::{PlayMutation, PlayStepResult, PlayWorld};

pub struct PlayRunner {
    db: PlayDb,
    world: PlayWorld,
}

impl PlayRunner {
    pub fn new(world: PlayWorld, db_path: &Path) -> Result<Self, AppError> {
        let db = PlayDb::new(db_path)?;
        db.migrate()?;
        Ok(Self { db, world })
    }

    /// 播种第一幕：基于 premise 生成初始 mutation → seed 图 → 渲染开场白。
    pub async fn seed_opening(
        &self,
        engine: &AgentEngine,
    ) -> Result<Option<PlayStepResult>, AppError> {
        // 已有事件说明已播种，不重复
        if self.db.current_turn()? > 0 {
            return Ok(None);
        }

        let context = format!(
            "世界前提：{}\n世界契约：{}\n模式：{}",
            self.world.premise,
            self.world.world_contract,
            self.world.mode
        );
        let action =
            PlayActionInterpreterAgent::interpret(engine, "开场：世界展开", &self.world.language)
                .await?;
        let mutation = PlayWorldMutatorAgent::propose_mutation(
            engine,
            &action,
            &context,
            &self.world.language,
        )
        .await?;

        // 渲染开场正文（在 commit 前完成）
        let state_json = serde_json::to_string_pretty(&mutation).unwrap_or_default();
        let scene_text = PlaySceneRendererAgent::render(
            engine,
            &state_json,
            &action,
            &self.world.mode,
            &self.world.language,
        )
        .await?;

        // 对账补抓
        let extra = PlaySceneReconcilerAgent::reconcile(
            engine,
            &scene_text,
            &mutation,
            &self.world.language,
        )
        .await?;
        let mut merged = mutation.clone();
        merge_mutation(&mut merged, &extra);

        seed_play_graph(&self.db, &merged)?;

        Ok(Some(PlayStepResult {
            turn: 0,
            scene_text,
            action,
            mutation: merged,
            suggested_actions: extract_suggested_actions(&self.world.mode),
            blocked: false,
            blocked_reason: None,
        }))
    }

    /// 一回合完整流程
    pub async fn step(
        &self,
        engine: &AgentEngine,
        player_input: &str,
    ) -> Result<PlayStepResult, AppError> {
        // 1. interpret
        let action = PlayActionInterpreterAgent::interpret(
            engine,
            player_input,
            &self.world.language,
        )
        .await?;

        // 2. mutate（context 来自当前 DB 快照）
        let context = self.build_context();
        let mutation =
            PlayWorldMutatorAgent::propose_mutation(engine, &action, &context, &self.world.language)
                .await?;

        // 3. render（commit 前完成，失败不污染 DB）
        let state_json = serde_json::to_string_pretty(&mutation).unwrap_or_default();
        let scene_text = PlaySceneRendererAgent::render(
            engine,
            &state_json,
            &action,
            &self.world.mode,
            &self.world.language,
        )
        .await?;

        // 4. reconcile
        let extra = PlaySceneReconcilerAgent::reconcile(
            engine,
            &scene_text,
            &mutation,
            &self.world.language,
        )
        .await?;

        // 5. merge
        let mut merged = mutation.clone();
        merge_mutation(&mut merged, &extra);

        // 6. apply
        let event = apply_play_mutation(&self.db, &merged)?;

        Ok(PlayStepResult {
            turn: event.turn,
            scene_text,
            action,
            mutation: merged,
            suggested_actions: extract_suggested_actions(&self.world.mode),
            blocked: event_summary_blocked(&event.summary),
            blocked_reason: None,
        })
    }

    /// 重写上一回合：snapshot 回滚保护 + 用 new_input 重新生成。
    ///
    /// 语义：本实现为"重投"——保留历史，生成一个新的替代回合。
    /// 失败时回滚到 snapshot，保证 DB 一致性。
    /// 真正的历史回退应通过 checkpoint/restore 完成。
    pub async fn regenerate_last_turn(
        &self,
        engine: &AgentEngine,
        new_input: &str,
    ) -> Result<PlayStepResult, AppError> {
        let snapshot = self.db.snapshot()?;
        match self.step(engine, new_input).await {
            Ok(r) => Ok(r),
            Err(e) => {
                // 回滚以保证一致性
                tracing::warn!(error = %e, "regenerate_last_turn 失败，回滚 snapshot");
                let _ = self.db.replace_with_snapshot(&snapshot);
                Err(e)
            }
        }
    }

    /// 构建当前世界状态上下文（compact JSON）
    fn build_context(&self) -> String {
        let snap = match self.db.snapshot() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "PlayRunner 读取 snapshot 失败，使用空上下文");
                return format!("世界前提：{}\n（状态读取失败）", self.world.premise);
            }
        };
        // 限制事件数量避免上下文膨胀（最近 10 条）
        let mut events = snap.events;
        if events.len() > 10 {
            events = events.split_off(events.len() - 10);
        }
        let ctx = serde_json::json!({
            "premise": self.world.premise,
            "entities": snap.entities,
            "edges": snap.edges,
            "stateSlots": snap.state_slots,
            "recentEvents": events,
        });
        serde_json::to_string_pretty(&ctx).unwrap_or_default()
    }

    /// 当前图谱快照（供 IPC 读取）
    pub fn snapshot_state(&self) -> Result<serde_json::Value, AppError> {
        let snap = self.db.snapshot()?;
        Ok(serde_json::to_value(&snap)?)
    }

    /// 事件历史（默认按 turn 升序）
    pub fn history(&self, limit: Option<u32>) -> Result<Vec<super::types::PlayEvent>, AppError> {
        self.db.get_events(limit)
    }
}

/// 把 extra 的增量并入 target（追加，不覆盖已有条目）。
fn merge_mutation(target: &mut PlayMutation, extra: &PlayMutation) {
    target.entities_upsert.extend(extra.entities_upsert.clone());
    target.edges_upsert.extend(extra.edges_upsert.clone());
    target.edges_expire.extend(extra.edges_expire.clone());
    target.state_slots_upsert.extend(extra.state_slots_upsert.clone());
    target.evidence_transitions.extend(extra.evidence_transitions.clone());
    target.notes.extend(extra.notes.clone());
    if extra.blocked {
        target.blocked = true;
        if target.blocked_reason.is_none() {
            target.blocked_reason = extra.blocked_reason.clone();
        }
    }
}

/// guided 模式给出建议动作占位（实际建议由渲染 agent 在正文中自然给出）。
fn extract_suggested_actions(mode: &str) -> Vec<String> {
    if mode == "guided" {
        vec!["观察周围".to_string(), "与某人交谈".to_string(), "前往别处".to_string()]
    } else {
        Vec::new()
    }
}

fn event_summary_blocked(summary: &str) -> bool {
    summary.contains("阻塞")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_appends_extra() {
        let mut target = PlayMutation::default();
        target.entities_upsert.push(super::super::types::PlayEntity {
            id: "a".into(),
            label: "A".into(),
            entity_type: super::super::types::PlayEntityType::Actor,
            summary: "".into(),
            physical: None,
            attributes: serde_json::json!({}),
        });
        let mut extra = PlayMutation::default();
        extra.entities_upsert.push(super::super::types::PlayEntity {
            id: "b".into(),
            label: "B".into(),
            entity_type: super::super::types::PlayEntityType::Item,
            summary: "".into(),
            physical: Some(true),
            attributes: serde_json::json!({}),
        });
        merge_mutation(&mut target, &extra);
        assert_eq!(target.entities_upsert.len(), 2);
    }
}
