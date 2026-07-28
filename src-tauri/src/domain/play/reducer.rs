//! ═══════════════════════════════════════════════════════════════════════════
//! Play 变更应用 - Mutation 写入 DB
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! apply_play_mutation 把一个 PlayMutation 原子地写入 DB：
//! 1. canonicalize_player_entity_ids —— "player" 统一为 "actor_player"
//! 2. 构造 PlayEvent
//! 3. validate_mutation —— stateSlots 引用实体存在 + 证据变迁不可倒退
//! 4. transaction 内：recordEvent + applyGraphChanges（非 blocked 才写图）
//! 5. normalize_holding_edge —— holding 边目标必须是物理实体，否则丢弃并记 note
//!
//! seed_play_graph 用于播种第一幕（不创建 event，仅写图）。

use rusqlite::Connection;

use crate::shared::error::AppError;
use crate::shared::utils::json::extract_json_block;

use super::db::PlayDb;
use super::types::{
    PlayActionKind, PlayEdge, PlayEntity, PlayEvent, PlayEvidenceStatus, PlayMutation,
    PlayStateSlot,
};

/// 玩家固定实体 ID
const PLAYER_CANONICAL_ID: &str = "actor_player";

/// 把 mutation 中所有 "player" 引用统一为 "actor_player"。
fn canonicalize_player_entity_ids(mutation: &mut PlayMutation) {
    let fix = |id: &str| -> String {
        if id == "player" {
            PLAYER_CANONICAL_ID.to_string()
        } else {
            id.to_string()
        }
    };
    for e in &mut mutation.entities_upsert {
        if e.id == "player" {
            e.id = PLAYER_CANONICAL_ID.to_string();
        }
    }
    for e in &mut mutation.edges_upsert {
        e.from_id = fix(&e.from_id);
        e.to_id = fix(&e.to_id);
    }
    for s in &mut mutation.state_slots_upsert {
        if let Some(owner) = s.owner_entity_id.take() {
            s.owner_entity_id = Some(fix(&owner));
        }
    }
}

/// 生成 event_id（turn + 时间戳，避免单回合内碰撞）
fn make_event_id(turn: u32) -> String {
    let ts = chrono::Utc::now().timestamp_millis();
    format!("evt-{}-{}", turn, ts)
}

/// 校验 mutation：
/// - stateSlots 引用的 owner_entity 必须已存在（DB 或本次 upsert）
/// - 证据变迁 from_status 必须等于当前状态，且 to_status 不可倒退
fn validate_mutation(
    conn: &Connection,
    mutation: &PlayMutation,
) -> Result<(), AppError> {
    // 收集本次 upsert 的实体 id（增量），与 DB 已有实体合并
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in (conn
        .prepare("SELECT id FROM entities")
        .map_err(db_err)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?).flatten()
    {
        known.insert(id);
    }
    for e in &mutation.entities_upsert {
        known.insert(e.id.clone());
    }

    // stateSlots owner_entity_id 引用校验
    for s in &mutation.state_slots_upsert {
        if let Some(owner) = &s.owner_entity_id {
            if !known.contains(owner) {
                return Err(AppError::invalid_state(format!(
                    "state_slot {} 引用了不存在的实体 {}",
                    s.id, owner
                )));
            }
        }
    }

    // 证据变迁校验：from_status 必须等于当前状态，to_status 不可倒退
    for t in &mutation.evidence_transitions {
        let current = current_evidence_status(conn, &t.claim_id)?;
        if current != t.from_status {
            return Err(AppError::invalid_state(format!(
                "证据变迁 claim={} 的 from_status({:?}) 与当前状态({:?})不一致",
                t.claim_id, t.from_status, current
            )));
        }
        if t.to_status.rank() < t.from_status.rank() {
            return Err(AppError::invalid_state(format!(
                "证据变迁 claim={} 不可倒退：{:?} -> {:?}",
                t.claim_id, t.from_status, t.to_status
            )));
        }
    }
    Ok(())
}

/// 读取某 claim 当前的证据状态（state_slots 中 slot_kind=evidence, key=claim_id 的 value 反查）
fn current_evidence_status(
    conn: &Connection,
    claim_id: &str,
) -> Result<PlayEvidenceStatus, AppError> {
    let mut stmt = conn
        .prepare("SELECT value FROM state_slots WHERE slot_kind = 'evidence' AND key = ?1")
        .map_err(db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![claim_id], |row| row.get::<_, f64>(0))
        .map_err(db_err)?;
    let mut val: Option<f64> = None;
    for r in rows {
        val = Some(r.map_err(db_err)?);
    }
    Ok(match val {
        Some(v) => status_from_rank(v as u8),
        None => PlayEvidenceStatus::Unknown,
    })
}

/// 把 state_slot.value（rank）映射回 PlayEvidenceStatus
fn status_from_rank(rank: u8) -> PlayEvidenceStatus {
    match rank {
        1 => PlayEvidenceStatus::Hinted,
        2 => PlayEvidenceStatus::Seen,
        3 => PlayEvidenceStatus::Collected,
        4 => PlayEvidenceStatus::Verified,
        5 => PlayEvidenceStatus::Weaponized,
        6 => PlayEvidenceStatus::Exposed,
        7 => PlayEvidenceStatus::Exhausted,
        _ => PlayEvidenceStatus::Unknown,
    }
}

/// 规范化 holding 边：to_id 必须是物理实体，否则丢弃并记 note。
fn normalize_holding_edges(
    conn: &Connection,
    mutation: &mut PlayMutation,
) -> Result<(), AppError> {
    if mutation.edges_upsert.is_empty() {
        return Ok(());
    }
    // 收集 DB + 本次 upsert 中物理实体的 id
    let mut physical_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in (conn
        .prepare("SELECT id FROM entities WHERE physical = 1")
        .map_err(db_err)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?).flatten()
    {
        physical_ids.insert(id);
    }
    for e in &mutation.entities_upsert {
        if e.physical.unwrap_or(false) {
            physical_ids.insert(e.id.clone());
        }
    }
    let mut kept = Vec::new();
    for edge in mutation.edges_upsert.drain(..) {
        if edge.edge_type == "holding" && !physical_ids.contains(&edge.to_id) {
            mutation.notes.push(format!(
                "丢弃 holding 边 {}：目标 {} 不是物理实体",
                edge.id, edge.to_id
            ));
            continue;
        }
        kept.push(edge);
    }
    mutation.edges_upsert = kept;
    Ok(())
}

/// 应用图变更（在已 BEGIN 的事务内调用，不单独开事务）
fn apply_graph_changes_on(
    conn: &Connection,
    mutation: &PlayMutation,
    event_id: &str,
) -> Result<(), AppError> {
    for e in &mutation.entities_upsert {
        upsert_entity_on(conn, e)?;
    }
    for e in &mutation.edges_upsert {
        upsert_edge_on(conn, e)?;
    }
    for exp in &mutation.edges_expire {
        expire_edge_on(conn, &exp.edge_id, event_id)?;
    }
    for s in &mutation.state_slots_upsert {
        upsert_state_slot_on(conn, s)?;
    }
    // 证据变迁：把目标 claim 的 evidence 状态槽更新为 to_status 的 rank
    for t in &mutation.evidence_transitions {
        let slot = PlayStateSlot {
            id: format!("evidence-{}", t.claim_id),
            owner_entity_id: None,
            slot_kind: super::types::PlayStateSlotKind::Evidence,
            key: t.claim_id.clone(),
            value: t.to_status.rank() as f64,
            min: Some(0.0),
            max: Some(7.0),
            unit: None,
        };
        upsert_state_slot_on(conn, &slot)?;
    }
    Ok(())
}

/// 应用 Mutation 到 DB，返回创建的 PlayEvent。
///
/// 流程：canonicalize → 构造 event → validate → transaction(recordEvent + applyGraphChanges)
pub fn apply_play_mutation(db: &PlayDb, mutation: &PlayMutation) -> Result<PlayEvent, AppError> {
    let mut mutation = mutation.clone();
    canonicalize_player_entity_ids(&mut mutation);

    let turn = mutation.turn.unwrap_or_else(|| {
        db.current_turn().unwrap_or(0).saturating_add(1)
    });
    let action_kind = mutation.action_kind.clone().unwrap_or(PlayActionKind::Do);
    let event_id = mutation
        .event_id
        .clone()
        .unwrap_or_else(|| make_event_id(turn));
    let timestamp = chrono::Utc::now().to_rfc3339();

    let event = PlayEvent {
        event_id: event_id.clone(),
        turn,
        action_kind: action_kind.clone(),
        summary: mutation.summary.clone().unwrap_or_default(),
        time_advance: mutation.time_advance.clone(),
        timestamp,
    };

    db.transaction(|conn| {
        validate_mutation(conn, &mutation)?;
        normalize_holding_edges(conn, &mut mutation)?;
        record_event_on(conn, &event)?;
        if !mutation.blocked {
            apply_graph_changes_on(conn, &mutation, &event_id)?;
        }
        Ok(())
    })?;

    Ok(event)
}

/// 播种图谱（不创建 event，仅写实体/边/状态槽）。
pub fn seed_play_graph(db: &PlayDb, mutation: &PlayMutation) -> Result<(), AppError> {
    let mut mutation = mutation.clone();
    canonicalize_player_entity_ids(&mut mutation);
    db.transaction(|conn| {
        normalize_holding_edges(conn, &mut mutation)?;
        apply_graph_changes_on(conn, &mutation, "seed")?;
        Ok(())
    })
}

// ── *_on 辅助：与 db.rs 同款实现（保持事务内可见性） ──
// 这些与 db.rs 私有函数重复，但跨模块无法复用私有项；
// 为避免循环依赖与可见性膨胀，这里内联最小实现。

fn db_err(e: rusqlite::Error) -> AppError {
    AppError::db_query(e.to_string())
}

fn upsert_entity_on(conn: &Connection, e: &PlayEntity) -> Result<(), AppError> {
    let physical = e.physical.map(|b| b as i64);
    let attrs = serde_json::to_string(&e.attributes).unwrap_or_else(|_| "{}".to_string());
    let entity_type = serde_json::to_string(&e.entity_type)?
        .trim_matches('"')
        .to_string();
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, label, entity_type, summary, physical, attributes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![e.id, e.label, entity_type, e.summary, physical, attrs],
    )
    .map_err(db_err)?;
    Ok(())
}

fn upsert_edge_on(conn: &Connection, e: &PlayEdge) -> Result<(), AppError> {
    let attrs = serde_json::to_string(&e.attributes).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO edges (id, from_id, to_id, edge_type, role, valid_from_event_id, valid_until_event_id, attributes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            e.id, e.from_id, e.to_id, e.edge_type, e.role, e.valid_from_event_id,
            e.valid_until_event_id, attrs,
        ],
    )
    .map_err(db_err)?;
    Ok(())
}

fn expire_edge_on(conn: &Connection, edge_id: &str, event_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE edges SET valid_until_event_id = ?2 WHERE id = ?1 AND valid_until_event_id IS NULL",
        rusqlite::params![edge_id, event_id],
    )
    .map_err(db_err)?;
    Ok(())
}

fn upsert_state_slot_on(conn: &Connection, s: &PlayStateSlot) -> Result<(), AppError> {
    let slot_kind = serde_json::to_string(&s.slot_kind)?
        .trim_matches('"')
        .to_string();
    conn.execute(
        "INSERT OR REPLACE INTO state_slots (id, owner_entity_id, slot_kind, key, value, min, max, unit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![s.id, s.owner_entity_id, slot_kind, s.key, s.value, s.min, s.max, s.unit],
    )
    .map_err(db_err)?;
    Ok(())
}

fn record_event_on(conn: &Connection, ev: &PlayEvent) -> Result<(), AppError> {
    let time_advance = match &ev.time_advance {
        Some(t) => Some(serde_json::to_string(t)?),
        None => None,
    };
    let action_kind = serde_json::to_string(&ev.action_kind)?
        .trim_matches('"')
        .to_string();
    conn.execute(
        "INSERT OR REPLACE INTO events (event_id, turn, action_kind, summary, time_advance, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![ev.event_id, ev.turn, action_kind, ev.summary, time_advance, ev.timestamp],
    )
    .map_err(db_err)?;
    Ok(())
}

// extract_json_block 仅用于未来可能的 LLM mutation 解析兜底，当前 reducer 暂未用到。
// 显式标记避免 unused import 警告。
#[allow(unused_imports)]
use extract_json_block as _extract_json_block;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::play::types::{
        PlayActionKind, PlayEntity, PlayEntityType, PlayMutation,
    };

    fn in_memory_db() -> PlayDb {
        let dir = std::env::temp_dir().join("mnemosyne_play_reducer_test");
        let _ = std::fs::remove_dir_all(&dir);
        let db = PlayDb::new(&dir.join("play.db")).unwrap();
        db.migrate().unwrap();
        db
    }

    #[test]
    fn seed_then_step_records_event() {
        let db = in_memory_db();
        let mut seed = PlayMutation::default();
        seed.entities_upsert.push(PlayEntity {
            id: "actor_player".into(),
            label: "玩家".into(),
            entity_type: PlayEntityType::Actor,
            summary: "主角".into(),
            physical: Some(true),
            attributes: serde_json::json!({}),
        });
        seed_play_graph(&db, &seed).unwrap();
        assert_eq!(db.get_all_entities().unwrap().len(), 1);

        let mut m = PlayMutation::default();
        m.action_kind = Some(PlayActionKind::Look);
        m.summary = Some("环顾四周".into());
        let ev = apply_play_mutation(&db, &m).unwrap();
        assert_eq!(ev.turn, 1);
        assert_eq!(db.get_events(None).unwrap().len(), 1);
    }

    #[test]
    fn canonicalize_player_id() {
        let mut m = PlayMutation::default();
        m.entities_upsert.push(PlayEntity {
            id: "player".into(),
            label: "p".into(),
            entity_type: PlayEntityType::Actor,
            summary: "".into(),
            physical: None,
            attributes: serde_json::json!({}),
        });
        canonicalize_player_entity_ids(&mut m);
        assert_eq!(m.entities_upsert[0].id, "actor_player");
    }
}
