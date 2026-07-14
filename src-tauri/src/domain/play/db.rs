// Play 世界图谱的 SQLite 持久化。
//
// 4 张表：entities / edges / state_slots / events。
// 使用独立的 rusqlite::Connection（每个世界一个 play.db 文件），
// 内部用 Mutex 包裹以支持 &self 方法的多线程访问。
//
// transaction 方法把 &Connection 传给闭包，保证多操作原子性。
// 注意：任务规格里 transaction 闭包无参数签名会无法访问连接，
// 这里修正为 FnOnce(&Connection) -> Result<R, AppError>，是正确性必需。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::shared::error::AppError;

use super::types::{
    PlayActionKind, PlayEdge, PlayEntity, PlayEntityType, PlayEvent, PlayEvidenceStatus,
    PlayGraphSnapshot, PlayStateSlot, PlayStateSlotKind, PlayTimeAdvance,
};

/// Play 图谱数据库
pub struct PlayDb {
    conn: Mutex<Connection>,
}

fn db_err(e: rusqlite::Error) -> AppError {
    AppError::db_query(e.to_string())
}

fn lock_err(e: std::sync::PoisonError<std::sync::MutexGuard<'_, Connection>>) -> AppError {
    AppError::internal(format!("PlayDb mutex poisoned: {}", e))
}

impl PlayDb {
    pub fn new(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::internal(format!("Failed to create play db directory: {}", e))
            })?;
        }
        let conn = Connection::open(path).map_err(db_err)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;\
             PRAGMA foreign_keys = ON;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 建表（幂等）
    pub fn migrate(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        migrate_on(&conn)
    }

    // ── 实体 ──────────────────────────────────────────

    pub fn upsert_entity(&self, entity: &PlayEntity) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        upsert_entity_on(&conn, entity)
    }

    pub fn get_all_entities(&self) -> Result<Vec<PlayEntity>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_all_entities_on(&conn)
    }

    // ── 边 ────────────────────────────────────────────

    pub fn upsert_edge(&self, edge: &PlayEdge) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        upsert_edge_on(&conn, edge)
    }

    pub fn expire_edge(&self, edge_id: &str, event_id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        expire_edge_on(&conn, edge_id, event_id)
    }

    /// 当前有效边（valid_until_event_id IS NULL）
    pub fn get_current_edges_for_entity(&self, entity_id: &str) -> Result<Vec<PlayEdge>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_current_edges_for_entity_on(&conn, entity_id)
    }

    /// 指向某 claim 的有效边（to_id = claim_id）
    pub fn get_evidence_for_claim(&self, claim_id: &str) -> Result<Vec<PlayEdge>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_evidence_for_claim_on(&conn, claim_id)
    }

    pub fn get_all_edges(&self) -> Result<Vec<PlayEdge>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_all_edges_on(&conn)
    }

    // ── 状态槽 ────────────────────────────────────────

    pub fn upsert_state_slot(&self, slot: &PlayStateSlot) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        upsert_state_slot_on(&conn, slot)
    }

    pub fn get_all_state_slots(&self) -> Result<Vec<PlayStateSlot>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_all_state_slots_on(&conn)
    }

    /// 读取某 claim 当前证据状态槽（slot_kind=evidence, key=claim_id）
    pub fn get_evidence_status(&self, claim_id: &str) -> Result<Option<PlayEvidenceStatus>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_evidence_status_on(&conn, claim_id)
    }

    // ── 事件 ──────────────────────────────────────────

    pub fn record_event(&self, event: &PlayEvent) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        record_event_on(&conn, event)
    }

    pub fn get_events(&self, limit: Option<u32>) -> Result<Vec<PlayEvent>, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        get_events_on(&conn, limit)
    }

    pub fn current_turn(&self) -> Result<u32, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let mut stmt = conn
            .prepare("SELECT COALESCE(MAX(turn), 0) FROM events")
            .map_err(db_err)?;
        let turn: u32 = stmt.query_row([], |row| row.get(0)).map_err(db_err)?;
        Ok(turn)
    }

    // ── 快照 ──────────────────────────────────────────

    pub fn snapshot(&self) -> Result<PlayGraphSnapshot, AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        let entities = get_all_entities_on(&conn)?;
        let edges = get_all_edges_on(&conn)?;
        let state_slots = get_all_state_slots_on(&conn)?;
        let events = get_events_on(&conn, None)?;
        Ok(PlayGraphSnapshot {
            entities,
            edges,
            state_slots,
            events,
        })
    }

    pub fn replace_with_snapshot(&self, snapshot: &PlayGraphSnapshot) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch(
            "DELETE FROM entities; DELETE FROM edges; DELETE FROM state_slots; DELETE FROM events;",
        )
        .map_err(db_err)?;
        for e in &snapshot.entities {
            upsert_entity_on(&conn, e)?;
        }
        for e in &snapshot.edges {
            upsert_edge_on(&conn, e)?;
        }
        for s in &snapshot.state_slots {
            upsert_state_slot_on(&conn, s)?;
        }
        for ev in &snapshot.events {
            record_event_on(&conn, ev)?;
        }
        Ok(())
    }

    /// 事务：在单个 BEGIN/COMMIT 中执行闭包，失败 ROLLBACK。
    /// 闭包接收已 BEGIN 的 &Connection，可直接调用 *_on 辅助函数。
    pub fn transaction<F, R>(&self, f: F) -> Result<R, AppError>
    where
        F: FnOnce(&Connection) -> Result<R, AppError>,
    {
        let conn = self.conn.lock().map_err(lock_err)?;
        conn.execute_batch("BEGIN").map_err(db_err)?;
        match f(&conn) {
            Ok(r) => {
                conn.execute_batch("COMMIT").map_err(db_err)?;
                Ok(r)
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}

// ── 建表 ──────────────────────────────────────────────

fn migrate_on(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entities (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            physical INTEGER,
            attributes TEXT NOT NULL DEFAULT '{}'
        );
        CREATE TABLE IF NOT EXISTS edges (
            id TEXT PRIMARY KEY,
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            edge_type TEXT NOT NULL,
            role TEXT,
            valid_from_event_id TEXT,
            valid_until_event_id TEXT,
            attributes TEXT NOT NULL DEFAULT '{}'
        );
        CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
        CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
        CREATE TABLE IF NOT EXISTS state_slots (
            id TEXT PRIMARY KEY,
            owner_entity_id TEXT,
            slot_kind TEXT NOT NULL,
            key TEXT NOT NULL,
            value REAL NOT NULL DEFAULT 0,
            min REAL,
            max REAL,
            unit TEXT
        );
        CREATE TABLE IF NOT EXISTS events (
            event_id TEXT PRIMARY KEY,
            turn INTEGER NOT NULL,
            action_kind TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            time_advance TEXT,
            timestamp TEXT NOT NULL
        );",
    )
    .map_err(db_err)?;
    Ok(())
}

// ── *_on 辅助函数（接收 &Connection，供 transaction 复用） ──

fn upsert_entity_on(conn: &Connection, e: &PlayEntity) -> Result<(), AppError> {
    let physical = e.physical.map(|b| b as i64);
    let attrs = serde_json::to_string(&e.attributes).unwrap_or_else(|_| "{}".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, label, entity_type, summary, physical, attributes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            e.id,
            e.label,
            serde_json::to_string(&e.entity_type)?.trim_matches('"'),
            e.summary,
            physical,
            attrs,
        ],
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
            e.id,
            e.from_id,
            e.to_id,
            e.edge_type,
            e.role,
            e.valid_from_event_id,
            e.valid_until_event_id,
            attrs,
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
    conn.execute(
        "INSERT OR REPLACE INTO state_slots (id, owner_entity_id, slot_kind, key, value, min, max, unit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            s.id,
            s.owner_entity_id,
            serde_json::to_string(&s.slot_kind)?.trim_matches('"'),
            s.key,
            s.value,
            s.min,
            s.max,
            s.unit,
        ],
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

impl PlayActionKind {
    pub fn from_snake(s: &str) -> Self {
        match s {
            "say" => Self::Say,
            "move" => Self::Move,
            "do" => Self::Do,
            "wait" => Self::Wait,
            _ => Self::Look,
        }
    }
}

impl PlayEntityType {
    pub fn from_snake(s: &str) -> Self {
        match s {
            "location" => Self::Location,
            "item" => Self::Item,
            "evidence" => Self::Evidence,
            "clue" => Self::Clue,
            "claim" => Self::Claim,
            "proof_chain" => Self::ProofChain,
            "organization" => Self::Organization,
            "rule" => Self::Rule,
            "scene" => Self::Scene,
            "event" => Self::Event,
            _ => Self::Actor,
        }
    }
}

impl PlayStateSlotKind {
    pub fn from_snake(s: &str) -> Self {
        match s {
            "relation" => Self::Relation,
            "pressure" => Self::Pressure,
            "clue" => Self::Clue,
            "evidence" => Self::Evidence,
            "flag" => Self::Flag,
            "timer" => Self::Timer,
            _ => Self::Resource,
        }
    }
}

impl PlayEvidenceStatus {
    pub fn from_snake(s: &str) -> Self {
        match s {
            "hinted" => Self::Hinted,
            "seen" => Self::Seen,
            "collected" => Self::Collected,
            "verified" => Self::Verified,
            "weaponized" => Self::Weaponized,
            "exposed" => Self::Exposed,
            "exhausted" => Self::Exhausted,
            _ => Self::Unknown,
        }
    }
}

fn get_all_entities_on(conn: &Connection) -> Result<Vec<PlayEntity>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id, label, entity_type, summary, physical, attributes FROM entities")
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            let entity_type_str: String = row.get(2)?;
            let physical: Option<i64> = row.get(4)?;
            let attrs_str: String = row.get(5)?;
            Ok(PlayEntity {
                id: row.get(0)?,
                label: row.get(1)?,
                entity_type: PlayEntityType::from_snake(&entity_type_str),
                summary: row.get(3)?,
                physical: physical.map(|v| v != 0),
                attributes: serde_json::from_str(&attrs_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

fn get_all_edges_on(conn: &Connection) -> Result<Vec<PlayEdge>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, from_id, to_id, edge_type, role, valid_from_event_id, valid_until_event_id, attributes FROM edges",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            let attrs_str: String = row.get(7)?;
            Ok(PlayEdge {
                id: row.get(0)?,
                from_id: row.get(1)?,
                to_id: row.get(2)?,
                edge_type: row.get(3)?,
                role: row.get(4)?,
                valid_from_event_id: row.get(5)?,
                valid_until_event_id: row.get(6)?,
                attributes: serde_json::from_str(&attrs_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

fn get_current_edges_for_entity_on(
    conn: &Connection,
    entity_id: &str,
) -> Result<Vec<PlayEdge>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, from_id, to_id, edge_type, role, valid_from_event_id, valid_until_event_id, attributes
             FROM edges
             WHERE (from_id = ?1 OR to_id = ?1) AND valid_until_event_id IS NULL",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![entity_id], |row| {
            let attrs_str: String = row.get(7)?;
            Ok(PlayEdge {
                id: row.get(0)?,
                from_id: row.get(1)?,
                to_id: row.get(2)?,
                edge_type: row.get(3)?,
                role: row.get(4)?,
                valid_from_event_id: row.get(5)?,
                valid_until_event_id: row.get(6)?,
                attributes: serde_json::from_str(&attrs_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

fn get_evidence_for_claim_on(conn: &Connection, claim_id: &str) -> Result<Vec<PlayEdge>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, from_id, to_id, edge_type, role, valid_from_event_id, valid_until_event_id, attributes
             FROM edges
             WHERE to_id = ?1 AND valid_until_event_id IS NULL",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![claim_id], |row| {
            let attrs_str: String = row.get(7)?;
            Ok(PlayEdge {
                id: row.get(0)?,
                from_id: row.get(1)?,
                to_id: row.get(2)?,
                edge_type: row.get(3)?,
                role: row.get(4)?,
                valid_from_event_id: row.get(5)?,
                valid_until_event_id: row.get(6)?,
                attributes: serde_json::from_str(&attrs_str)
                    .unwrap_or(serde_json::Value::Object(Default::default())),
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

fn get_all_state_slots_on(conn: &Connection) -> Result<Vec<PlayStateSlot>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id, owner_entity_id, slot_kind, key, value, min, max, unit FROM state_slots")
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            let slot_kind_str: String = row.get(2)?;
            Ok(PlayStateSlot {
                id: row.get(0)?,
                owner_entity_id: row.get(1)?,
                slot_kind: PlayStateSlotKind::from_snake(&slot_kind_str),
                key: row.get(3)?,
                value: row.get(4)?,
                min: row.get(5)?,
                max: row.get(6)?,
                unit: row.get(7)?,
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    Ok(out)
}

fn get_evidence_status_on(
    conn: &Connection,
    claim_id: &str,
) -> Result<Option<PlayEvidenceStatus>, AppError> {
    let mut stmt = conn
        .prepare("SELECT key FROM state_slots WHERE slot_kind = 'evidence' AND key = ?1")
        .map_err(db_err)?;
    let rows = stmt
        .query_map(rusqlite::params![claim_id], |row| row.get::<_, String>(0))
        .map_err(db_err)?;
    let mut found: Option<String> = None;
    for r in rows {
        found = Some(r.map_err(db_err)?);
    }
    Ok(found.map(|s| PlayEvidenceStatus::from_snake(&s)))
}

fn get_events_on(conn: &Connection, limit: Option<u32>) -> Result<Vec<PlayEvent>, AppError> {
    let sql = match limit {
        Some(n) => format!(
            "SELECT event_id, turn, action_kind, summary, time_advance, timestamp FROM events ORDER BY turn DESC LIMIT {}",
            n
        ),
        None => "SELECT event_id, turn, action_kind, summary, time_advance, timestamp FROM events ORDER BY turn DESC".to_string(),
    };
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            let action_kind_str: String = row.get(2)?;
            let time_advance_str: Option<String> = row.get(4)?;
            let time_advance = time_advance_str
                .as_deref()
                .and_then(|s| serde_json::from_str::<PlayTimeAdvance>(s).ok());
            Ok(PlayEvent {
                event_id: row.get(0)?,
                turn: row.get(1)?,
                action_kind: PlayActionKind::from_snake(&action_kind_str),
                summary: row.get(3)?,
                time_advance,
                timestamp: row.get(5)?,
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(db_err)?);
    }
    // 倒序取出后翻转为正序（按 turn 升序）
    out.reverse();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_and_roundtrip() {
        let dir = std::env::temp_dir().join("mnemosyne_play_db_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("play.db");
        let db = PlayDb::new(&path).unwrap();
        db.migrate().unwrap();

        let ent = PlayEntity {
            id: "actor_player".into(),
            label: "玩家".into(),
            entity_type: PlayEntityType::Actor,
            summary: "主角".into(),
            physical: Some(true),
            attributes: serde_json::json!({"hp": 10}),
        };
        db.upsert_entity(&ent).unwrap();
        let all = db.get_all_entities().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "actor_player");
        assert_eq!(all[0].entity_type, PlayEntityType::Actor);

        let snap = db.snapshot().unwrap();
        assert_eq!(snap.entities.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn evidence_status_rank_monotonic() {
        assert!(PlayEvidenceStatus::Unknown.rank() < PlayEvidenceStatus::Exhausted.rank());
    }
}
