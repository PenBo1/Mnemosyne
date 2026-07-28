//! ═══════════════════════════════════════════════════════════════════════════
//! Play 命令 - IPC 命令处理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 命令列表（前端使用 camelCase 调用）：
//! - play_create_world: 创建互动小说世界
//! - play_list_worlds: 列出所有世界
//! - play_seed_opening: 播种第一幕
//! - play_step: 执行一回合
//! - play_regenerate_last_turn: 重写上一回合
//! - play_get_state: 获取当前图谱快照
//! - play_get_history: 获取事件历史
//!
//! 约定：命令仅做参数提取 + 校验 + 委派，不含业务逻辑。
//! 路径操作通过 DataDir.play_dir() getters。

use crate::core::agent::commands::AgentState;
use crate::domain::play::runner::PlayRunner;
use crate::domain::play::store::PlayStore;
use crate::domain::play::types::{PlayEvent, PlayStepResult, PlayWorld};
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::{AppError, IpcResponse};
use std::time::Instant;
use tauri::State;

// ── 请求体 ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorldRequest {
    pub premise: String,
    #[serde(default)]
    pub world_contract: serde_json::Value,
    #[serde(default)]
    pub visual_contract: Option<serde_json::Value>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub player_persona: Option<serde_json::Value>,
    #[serde(default)]
    pub world_id: Option<String>,
}

fn default_mode() -> String {
    "open".to_string()
}

fn default_language() -> String {
    "zh".to_string()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTarget {
    pub world_id: String,
    pub run_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepRequest {
    pub world_id: String,
    pub run_id: String,
    pub player_input: String,
}

// ── 辅助 ────────────────────────────────────────────────

fn build_store(data_dir: &DataDir) -> PlayStore {
    PlayStore::new(data_dir.play_dir())
}

/// 构造 PlayRunner（基于 world + run 的 graph.db 路径）
fn build_runner(
    store: &PlayStore,
    world: &PlayWorld,
    world_id: &str,
    run_id: &str,
) -> Result<PlayRunner, AppError> {
    store.ensure_run(world_id, run_id)?;
    let db_path = store.graph_db_path(world_id, run_id)?;
    PlayRunner::new(world.clone(), &db_path)
}

// ── 创建世界 ────────────────────────────────────────────

#[tauri::command]
pub async fn play_create_world(
    data_dir: State<'_, DataDir>,
    request: CreateWorldRequest,
) -> Result<IpcResponse<PlayWorld>, AppError> {
    let start = Instant::now();
    tracing::info!(premise_len = request.premise.len(), "play_create_world: enter");
    
    if request.premise.trim().is_empty() {
        tracing::error!("play_create_world: premise is empty");
        return Err(AppError::invalid_input("premise 不能为空"));
    }
    if !matches!(request.mode.as_str(), "open" | "guided") {
        tracing::error!(mode = %request.mode, "play_create_world: invalid mode");
        return Err(AppError::invalid_input(format!(
            "mode 必须是 open/guided，收到: {}",
            request.mode
        )));
    }
    tracing::debug!(mode = %request.mode, "play_create_world: mode validated");

    let store = build_store(&data_dir);
    let now = chrono::Utc::now().to_rfc3339();
    let world_id = request.world_id.unwrap_or_else(|| {
        format!("world-{}", chrono::Utc::now().timestamp_millis())
    });
    // 校验 world_id 合法性（防路径穿越）
    crate::infrastructure::fs::fs_utils::validate_id_component(&world_id, "world_id")?;
    tracing::debug!(world_id = %world_id, "play_create_world: world_id validated");

    let world = PlayWorld {
        world_id: world_id.clone(),
        premise: request.premise,
        world_contract: request.world_contract,
        visual_contract: request.visual_contract,
        mode: request.mode,
        language: request.language,
        player_persona: request.player_persona,
        created_at: now.clone(),
        updated_at: now,
    };
    store.create_world(&world)?;
    
    tracing::info!(
        world_id = %world.world_id,
        duration_ms = start.elapsed().as_millis(),
        "play_create_world: exit"
    );
    Ok(IpcResponse::created(world))
}

// ── 列出世界 ────────────────────────────────────────────

#[tauri::command]
pub async fn play_list_worlds(
    data_dir: State<'_, DataDir>,
) -> Result<IpcResponse<Vec<PlayWorld>>, AppError> {
    let store = build_store(&data_dir);
    let worlds = store.list_worlds()?;
    Ok(IpcResponse::ok(worlds))
}

// ── 播种第一幕 ──────────────────────────────────────────

#[tauri::command]
pub async fn play_seed_opening(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    target: RunTarget,
) -> Result<IpcResponse<Option<PlayStepResult>>, AppError> {
    let store = build_store(&data_dir);
    let world = store.load_world(&target.world_id)?;
    let runner = build_runner(&store, &world, &target.world_id, &target.run_id)?;
    let result = runner.seed_opening(&agent_state.engine).await?;
    if let Some(ref step) = result {
        // 持久化开场事件与 transcript
        if let Some(ev) = build_event_from_step(step) {
            let _ = store.append_event(&target.world_id, &target.run_id, &ev);
        }
        let _ = store.append_transcript_turn(
            &target.world_id,
            &target.run_id,
            "narrator",
            &step.scene_text,
        );
    }
    Ok(IpcResponse::ok(result))
}

// ── 执行一回合 ──────────────────────────────────────────

#[tauri::command]
pub async fn play_step(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    request: StepRequest,
) -> Result<IpcResponse<PlayStepResult>, AppError> {
    if request.player_input.trim().is_empty() {
        return Err(AppError::invalid_input("playerInput 不能为空"));
    }
    let store = build_store(&data_dir);
    let world = store.load_world(&request.world_id)?;
    let runner = build_runner(&store, &world, &request.world_id, &request.run_id)?;

    // 记录玩家输入 transcript
    let _ = store.append_transcript_turn(
        &request.world_id,
        &request.run_id,
        "player",
        &request.player_input,
    );

    let step = runner.step(&agent_state.engine, &request.player_input).await?;

    // 持久化事件 + narrator 正文
    if let Some(ev) = build_event_from_step(&step) {
        let _ = store.append_event(&request.world_id, &request.run_id, &ev);
    }
    let _ = store.append_transcript_turn(
        &request.world_id,
        &request.run_id,
        "narrator",
        &step.scene_text,
    );
    // 保存当前状态投影
    let _ = store.save_current_state(
        &request.world_id,
        &request.run_id,
        &serde_json::to_value(&step.mutation).unwrap_or(serde_json::Value::Null),
    );
    Ok(IpcResponse::ok(step))
}

// ── 重写上一回合 ────────────────────────────────────────

#[tauri::command]
pub async fn play_regenerate_last_turn(
    agent_state: State<'_, AgentState>,
    data_dir: State<'_, DataDir>,
    request: StepRequest,
) -> Result<IpcResponse<PlayStepResult>, AppError> {
    if request.player_input.trim().is_empty() {
        return Err(AppError::invalid_input("playerInput 不能为空"));
    }
    let store = build_store(&data_dir);
    let world = store.load_world(&request.world_id)?;
    let runner = build_runner(&store, &world, &request.world_id, &request.run_id)?;
    let step = runner
        .regenerate_last_turn(&agent_state.engine, &request.player_input)
        .await?;
    if let Some(ev) = build_event_from_step(&step) {
        let _ = store.append_event(&request.world_id, &request.run_id, &ev);
    }
    let _ = store.append_transcript_turn(
        &request.world_id,
        &request.run_id,
        "narrator",
        &step.scene_text,
    );
    Ok(IpcResponse::ok(step))
}

// ── 获取当前状态 ────────────────────────────────────────

#[tauri::command]
pub async fn play_get_state(
    data_dir: State<'_, DataDir>,
    target: RunTarget,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let store = build_store(&data_dir);
    let world = store.load_world(&target.world_id)?;
    let runner = build_runner(&store, &world, &target.world_id, &target.run_id)?;
    // snapshot 通过 runner 内部 db 暴露
    // PlayRunner 没有直接暴露 snapshot，这里通过 get_state 包装
    Ok(IpcResponse::ok(runner.snapshot_state()?))
}

// ── 获取事件历史 ────────────────────────────────────────

#[tauri::command]
pub async fn play_get_history(
    data_dir: State<'_, DataDir>,
    target: RunTarget,
    limit: Option<u32>,
) -> Result<IpcResponse<Vec<PlayEvent>>, AppError> {
    let store = build_store(&data_dir);
    let world = store.load_world(&target.world_id)?;
    let runner = build_runner(&store, &world, &target.world_id, &target.run_id)?;
    let events = runner.history(limit)?;
    Ok(IpcResponse::ok(events))
}

// ── 内部辅助 ────────────────────────────────────────────

fn build_event_from_step(step: &PlayStepResult) -> Option<PlayEvent> {
    let m = &step.mutation;
    Some(PlayEvent {
        event_id: m
            .event_id
            .clone()
            .unwrap_or_else(|| format!("evt-{}", step.turn)),
        turn: step.turn,
        action_kind: step.action.action_kind.clone(),
        summary: m.summary.clone().unwrap_or_default(),
        time_advance: m.time_advance.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
