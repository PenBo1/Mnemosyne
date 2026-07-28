//! ═══════════════════════════════════════════════════════════════════════════
//! Play 运行器 - 4-agent 流水线编排
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! step 流程：
//! 1. interpret  归一玩家动作 → PlayActionIntent
//! 2. mutate     起草 PlayMutation（context 来自当前 DB 快照）
//! 3. render     渲染场景正文（在 commit 前完成，保证失败不污染 DB）
//! 4. reconcile  补抓遗漏实体
//! 5. merge      把 reconcile 增量并入主 mutation
//! 6. apply       提交到 DB（apply_play_mutation）

use std::path::Path;
use std::time::Instant;

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
        let start = Instant::now();
        tracing::info!(function = "PlayRunner::seed_opening", "入口");

        if self.db.current_turn()? > 0 {
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::info!(function = "PlayRunner::seed_opening", duration_ms, skipped = true, reason = "already_seeded", "出口");
            return Ok(None);
        }

        let context = format!(
            "世界前提：{}\n世界契约：{}\n模式：{}",
            self.world.premise,
            self.world.world_contract,
            self.world.mode
        );

        match async {
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

            let state_json = serde_json::to_string_pretty(&mutation)
                .map_err(|e| AppError::internal(format!("Failed to serialize mutation: {}", e)))?;
            let scene_text = PlaySceneRendererAgent::render(
                engine,
                &state_json,
                &action,
                &self.world.mode,
                &self.world.language,
            )
            .await?;

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

            Ok(PlayStepResult {
                turn: 0,
                scene_text,
                action,
                mutation: merged,
                suggested_actions: extract_suggested_actions(&self.world.mode),
                blocked: false,
                blocked_reason: None,
            })
        }.await {
            Ok(result) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "PlayRunner::seed_opening", duration_ms, "出口");
                Ok(Some(result))
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "PlayRunner::seed_opening", duration_ms, error = %e, "错误");
                Err(e)
            }
        }
    }

    /// 一回合完整流程
    pub async fn step(
        &self,
        engine: &AgentEngine,
        player_input: &str,
    ) -> Result<PlayStepResult, AppError> {
        let start = Instant::now();
        tracing::info!(function = "PlayRunner::step", player_input_len = player_input.len(), "入口");

        let result = async {
            let action = PlayActionInterpreterAgent::interpret(
                engine,
                player_input,
                &self.world.language,
            )
            .await?;

            let context = self.build_context()?;
            let mutation =
                PlayWorldMutatorAgent::propose_mutation(engine, &action, &context, &self.world.language)
                    .await?;

            let state_json = serde_json::to_string_pretty(&mutation)
                .map_err(|e| AppError::internal(format!("Failed to serialize mutation: {}", e)))?;
            let scene_text = PlaySceneRendererAgent::render(
                engine,
                &state_json,
                &action,
                &self.world.mode,
                &self.world.language,
            )
            .await?;

            let extra = PlaySceneReconcilerAgent::reconcile(
                engine,
                &scene_text,
                &mutation,
                &self.world.language,
            )
            .await?;

            let mut merged = mutation.clone();
            merge_mutation(&mut merged, &extra);

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
        }.await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "PlayRunner::step", duration_ms, turn = r.turn, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "PlayRunner::step", duration_ms, error = %e, "错误");
            }
        }
        result
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
        let start = Instant::now();
        tracing::info!(function = "PlayRunner::regenerate_last_turn", new_input_len = new_input.len(), "入口");

        let snapshot = self.db.snapshot()?;
        let result = self.step(engine, new_input).await;

        match &result {
            Ok(r) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!(function = "PlayRunner::regenerate_last_turn", duration_ms, turn = r.turn, "出口");
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::error!(function = "PlayRunner::regenerate_last_turn", duration_ms, error = %e, "错误");
                tracing::warn!(error = %e, "regenerate_last_turn 失败，回滚 snapshot");
                match self.db.replace_with_snapshot(&snapshot) {
                    Ok(()) => {}
                    Err(rollback_err) => {
                        tracing::error!(error = %rollback_err, "回滚失败");
                    }
                }
            }
        }

        result.map_err(|e| {
            match self.db.replace_with_snapshot(&snapshot) {
                Ok(()) => e,
                Err(rollback_err) => AppError::internal(format!(
                    "regenerate_last_turn failed: {}; rollback also failed: {}",
                    e.message, rollback_err.message
                )),
            }
        })
    }

    /// 构建当前世界状态上下文（compact JSON）
    fn build_context(&self) -> Result<String, AppError> {
        let snap = match self.db.snapshot() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "PlayRunner 读取 snapshot 失败，使用空上下文");
                return Ok(format!("世界前提：{}\n（状态读取失败）", self.world.premise));
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
        serde_json::to_string_pretty(&ctx)
            .map_err(|e| AppError::internal(format!("Failed to serialize play context: {}", e)))
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
