
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::shared::error::AppError;

const MIGRATION_COUNT: u32 = 19;

pub fn run_migrate(conn: &mut Connection) -> Result<(), AppError> {
    let sqlx_migrated: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false);

    if sqlx_migrated {
        conn.execute_batch(&format!("PRAGMA user_version = {}", MIGRATION_COUNT))
            .map_err(|e| AppError::internal(format!("Failed to set user_version: {}", e)))?;
        let _ = conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", []);
        tracing::info!(user_version = MIGRATION_COUNT, "Migrated from sqlx history, marked all as applied");
        return Ok(());
    }

    let migrations = Migrations::new(vec![
        M::up(include_str!("../../../migrations/20260628000001_init_state.sql")),
        M::up(include_str!("../../../migrations/20260630000001_drop_kanban.sql")),
        M::up(include_str!("../../../migrations/20260630000002_add_workspace_to_sessions.sql")),
        M::up(include_str!("../../../migrations/20260630000003_story_temporal_memory.sql")),
        M::up(include_str!("../../../migrations/20260701000001_drop_ai_log_tables.sql")),
        M::up(include_str!("../../../migrations/20260703000001_resource_quota.sql")),
        M::up(include_str!("../../../migrations/20260710000001_audit_events.sql")),
        M::up(include_str!("../../../migrations/20260711000001_workspace_last_opened_at.sql")),
        M::up(include_str!("../../../migrations/20260711000002_vector_store.sql")),
        M::up(include_str!("../../../migrations/20260713000001_memory_entries.sql")),
        M::up(include_str!("../../../migrations/20260713000002_loop_runs.sql")),
        M::up(include_str!("../../../migrations/20260713000003_short_term_memory.sql")),
        M::up(include_str!("../../../migrations/20260713000004_learned_preferences.sql")),
        M::up(include_str!("../../../migrations/20260713000005_skill_evolution.sql")),
        M::up(include_str!("../../../migrations/20260713000006_loop_states_patterns.sql")),
        M::up(include_str!("../../../migrations/20260713000007_trace_spans.sql")),
        M::up(include_str!("../../../migrations/20260713000008_metric_points.sql")),
        M::up(include_str!("../../../migrations/20260714000001_memory_archives.sql")),
        M::up(include_str!("../../../migrations/20260714000002_interaction_sessions.sql")),
    ]);

    migrations
        .to_latest(conn)
        .map_err(|e| AppError::internal(format!("Database migration failed: {}", e)))?;
    Ok(())
}