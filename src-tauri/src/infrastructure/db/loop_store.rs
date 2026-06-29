use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;

use super::models::{LoopState, LoopRunLog, LoopPattern, CreateLoopStateRequest, UpdateLoopStateRequest, UpsertLoopPatternRequest};
use super::Database;
use super::connection::db_err;
use super::error::{json_decode, json_encode};
use crate::shared::errors::AppError;

fn map_loop_state(row: &sqlx::sqlite::SqliteRow) -> Result<LoopState, AppError> {
    let payload_json: String = row.get(5);
    let config_json: String = row.get(6);
    let last_result_json: Option<String> = row.get(10);
    Ok(LoopState {
        id: row.get(0),
        novel_id: row.get(1),
        pattern_id: row.get(2),
        status: row.get(3),
        readiness_level: row.get(4),
        state_payload: json_decode(&payload_json, "state_payload")?,
        config: json_decode(&config_json, "config")?,
        token_usage_today: row.get(7),
        token_cap_daily: row.get(8),
        last_run_at: row.get(9),
        last_run_result: last_result_json
            .and_then(|j| serde_json::from_str(&j).ok()),
        created_at: row.get(11),
        updated_at: row.get(12),
    })
}

fn map_loop_run_log(row: &sqlx::sqlite::SqliteRow) -> Result<LoopRunLog, AppError> {
    Ok(LoopRunLog {
        id: row.get(0),
        loop_state_id: row.get(1),
        pattern_id: row.get(2),
        status: row.get(3),
        phase_results: json_decode(&row.get::<String, usize>(4), "phase_results")?,
        tokens_used: row.get(5),
        duration_ms: row.get(6),
        findings: json_decode(&row.get::<String, usize>(7), "findings")?,
        actions_taken: json_decode(&row.get::<String, usize>(8), "actions_taken")?,
        escalations: json_decode(&row.get::<String, usize>(9), "escalations")?,
        error_message: row.get(10),
        created_at: row.get(11),
    })
}

fn map_loop_pattern(row: &sqlx::sqlite::SqliteRow) -> Result<LoopPattern, AppError> {
    Ok(LoopPattern {
        id: row.get(0),
        name: row.get(1),
        description: row.get(2),
        goal: row.get(3),
        cadence: row.get(4),
        risk_level: row.get(10),
        phases: json_decode(&row.get::<String, usize>(5), "phases")?,
        human_gates: json_decode(&row.get::<String, usize>(6), "human_gates")?,
        cost_config: json_decode(&row.get::<String, usize>(7), "cost_config")?,
        skills_required: json_decode(&row.get::<String, usize>(8), "skills_required")?,
        state_schema: json_decode(&row.get::<String, usize>(9), "state_schema")?,
        is_active: row.get(11),
        created_at: row.get(12),
        updated_at: row.get(13),
    })
}

impl Database {
    pub async fn create_loop_state(&self, novel_id: &str, req: CreateLoopStateRequest) -> Result<LoopState, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let readiness = req.readiness_level.unwrap_or_else(|| "L0".to_string());
        let config = req.config.unwrap_or(serde_json::json!({}));
        let cap = req.token_cap_daily.unwrap_or(50000);
        let config_str = json_encode(&config, "config")?;

        sqlx::query(
            "INSERT INTO loop_states (id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, created_at, updated_at) VALUES (?, ?, ?, 'idle', ?, '{}', ?, 0, ?, ?, ?)"
        )
        .bind(&id).bind(novel_id).bind(&req.pattern_id).bind(&readiness)
        .bind(&config_str).bind(cap).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(LoopState {
            id,
            novel_id: novel_id.to_string(),
            pattern_id: req.pattern_id,
            status: "idle".to_string(),
            readiness_level: readiness,
            state_payload: serde_json::json!({}),
            config,
            token_usage_today: 0,
            token_cap_daily: cap,
            last_run_at: None,
            last_run_result: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn get_loop_states(&self, novel_id: &str) -> Result<Vec<LoopState>, AppError> {
        let rows = sqlx::query(
            "SELECT id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at FROM loop_states WHERE novel_id = ? ORDER BY created_at DESC"
        )
        .bind(novel_id)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(map_loop_state).collect()
    }

    pub async fn get_loop_state_by_id(&self, state_id: &str) -> Result<LoopState, AppError> {
        let row = sqlx::query(
            "SELECT id, novel_id, pattern_id, status, readiness_level, state_payload, config, token_usage_today, token_cap_daily, last_run_at, last_run_result, created_at, updated_at FROM loop_states WHERE id = ?"
        )
        .bind(state_id)
        .fetch_optional(&self.pool).await.map_err(db_err)?
        .ok_or_else(|| AppError::not_found("Loop state"))?;
        map_loop_state(&row)
    }

    pub async fn update_loop_state(&self, state_id: &str, req: UpdateLoopStateRequest) -> Result<LoopState, AppError> {
        let existing = self.get_loop_state_by_id(state_id).await?;
        let now = Utc::now().to_rfc3339();

        let status = req.status.unwrap_or(existing.status);
        let readiness_level = req.readiness_level.unwrap_or(existing.readiness_level);
        let config = req.config.unwrap_or(existing.config);
        let token_cap_daily = req.token_cap_daily.unwrap_or(existing.token_cap_daily);
        let last_run_at = req.last_run_at;
        let last_run_result = req.last_run_result;

        let config_str = json_encode(&config, "config")?;
        let result_str = match &last_run_result {
            Some(r) => Some(json_encode(r, "last_run_result")?),
            None => None,
        };

        sqlx::query(
            "UPDATE loop_states SET status = ?, readiness_level = ?, config = ?, token_cap_daily = ?, last_run_at = ?, last_run_result = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&status).bind(&readiness_level).bind(&config_str)
        .bind(token_cap_daily).bind(&last_run_at).bind(&result_str)
        .bind(&now).bind(state_id)
        .execute(&self.pool).await.map_err(db_err)?;

        self.get_loop_state_by_id(state_id).await
    }

    pub async fn delete_loop_state(&self, state_id: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM loop_states WHERE id = ?")
            .bind(state_id)
            .execute(&self.pool).await.map_err(db_err)?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Loop state"));
        }
        Ok(())
    }

    pub async fn create_loop_run_log(&self, log: &LoopRunLog) -> Result<LoopRunLog, AppError> {
        let phase_json = json_encode(&log.phase_results, "phase_results")?;
        let findings_json = json_encode(&log.findings, "findings")?;
        let actions_json = json_encode(&log.actions_taken, "actions_taken")?;
        let escalations_json = json_encode(&log.escalations, "escalations")?;

        sqlx::query(
            "INSERT INTO loop_run_logs (id, loop_state_id, pattern_id, status, phase_results, tokens_used, duration_ms, findings, actions_taken, escalations, error_message, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&log.id).bind(&log.loop_state_id).bind(&log.pattern_id).bind(&log.status)
        .bind(&phase_json).bind(log.tokens_used).bind(log.duration_ms)
        .bind(&findings_json).bind(&actions_json).bind(&escalations_json)
        .bind(&log.error_message).bind(&log.created_at)
        .execute(&self.pool).await.map_err(db_err)?;
        Ok(log.clone())
    }

    pub async fn get_loop_run_logs(&self, state_id: &str, limit: i64) -> Result<Vec<LoopRunLog>, AppError> {
        let rows = sqlx::query(
            "SELECT id, loop_state_id, pattern_id, status, phase_results, tokens_used, duration_ms, findings, actions_taken, escalations, error_message, created_at FROM loop_run_logs WHERE loop_state_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(state_id).bind(limit)
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(map_loop_run_log).collect()
    }

    pub async fn get_loop_patterns(&self) -> Result<Vec<LoopPattern>, AppError> {
        let rows = sqlx::query(
            "SELECT id, name, description, goal, cadence, phases, human_gates, cost_config, skills_required, state_schema, risk_level, is_active, created_at, updated_at FROM loop_patterns ORDER BY name"
        )
        .fetch_all(&self.pool).await.map_err(db_err)?;
        rows.iter().map(map_loop_pattern).collect()
    }

    pub async fn upsert_loop_pattern(&self, id: Option<&str>, req: UpsertLoopPatternRequest) -> Result<LoopPattern, AppError> {
        let now = Utc::now().to_rfc3339();
        let pattern_id = id.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let phases = match req.phases {
            Some(p) => json_encode(&p, "phases")?,
            None => "[]".to_string(),
        };
        let gates = match req.human_gates {
            Some(g) => json_encode(&g, "human_gates")?,
            None => "[]".to_string(),
        };
        let cost = match req.cost_config {
            Some(c) => json_encode(&c, "cost_config")?,
            None => "{}".to_string(),
        };
        let skills = match req.skills_required {
            Some(s) => json_encode(&s, "skills_required")?,
            None => "[]".to_string(),
        };
        let schema = match req.state_schema {
            Some(s) => json_encode(&s, "state_schema")?,
            None => "{}".to_string(),
        };
        let desc = req.description.unwrap_or_default();
        let goal = req.goal.unwrap_or_default();
        let cadence = req.cadence.unwrap_or_else(|| "1d".to_string());
        let risk = req.risk_level.unwrap_or_else(|| "low".to_string());
        let active = req.is_active.unwrap_or(true);

        sqlx::query(
            "INSERT INTO loop_patterns (id, name, description, goal, cadence, risk_level, phases, human_gates, cost_config, skills_required, state_schema, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, description=excluded.description, goal=excluded.goal, cadence=excluded.cadence, risk_level=excluded.risk_level, phases=excluded.phases, human_gates=excluded.human_gates, cost_config=excluded.cost_config, skills_required=excluded.skills_required, state_schema=excluded.state_schema, is_active=excluded.is_active, updated_at=excluded.updated_at"
        )
        .bind(&pattern_id).bind(&req.name).bind(&desc).bind(&goal).bind(&cadence).bind(&risk)
        .bind(&phases).bind(&gates).bind(&cost).bind(&skills).bind(&schema)
        .bind(active).bind(&now).bind(&now)
        .execute(&self.pool).await.map_err(db_err)?;

        Ok(LoopPattern {
            id: pattern_id,
            name: req.name,
            description: desc,
            goal,
            cadence,
            risk_level: risk,
            phases: json_decode(&phases, "phases")?,
            human_gates: json_decode(&gates, "human_gates")?,
            cost_config: json_decode(&cost, "cost_config")?,
            skills_required: json_decode(&skills, "skills_required")?,
            state_schema: json_decode(&schema, "state_schema")?,
            is_active: active,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}
