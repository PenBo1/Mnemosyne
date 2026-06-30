// 应用业务初始化编排 —— 承接 data_dir 的业务初始化上提
//
// 职责边界：在 DataDir 完成目录创建后，执行业务级初始化（提取内置资源、生成默认身份文件）
// 依赖规则：可依赖 infra（data_dir）+ 业务域（novel::source、agent::identity）
//
// 设计理由：原 infra/data_dir.rs 反向调用 features::novel::source 和 core::agent::identity，
// 违反"infra 不依赖业务"原则。本模块将业务初始化上提到 core 层编排，infra 只负责目录结构。

use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::{DataDir, AGENT_ROLES};
use crate::infrastructure::db::Database;

/// 执行业务级初始化（在 data_dir.initialize() 之后调用）。
///
/// 包含：
/// 1. 提取内置 novel sources 到 book_sources_dir
/// 2. 为每个 agent 角色生成默认 identity 文件（SOUL.md、CONTEXT.md、MEMORY.md）
///
/// 幂等：已存在的文件不会被覆盖，用户编辑过的身份文件会保留。
pub fn initialize_app_business_state(data_dir: &DataDir) -> Result<(), AppError> {
    ensure_builtin_book_sources(data_dir)?;
    ensure_agent_identities(data_dir)?;
    Ok(())
}

/// 种子化内置 Loop Pattern 到 DB（应用启动时调用，幂等）。
///
/// 把 `PatternRegistry::built_in_patterns()` 中的 7 个 pattern upsert 到 DB，
/// 让前端 `loop_get_patterns` 能直接列出。已存在的 pattern 会被更新（用户编辑仍可能被覆盖，
/// 因为 built-in pattern 是项目契约；如需保留用户改动，后续应改成"仅 insert 不存在者"）。
pub async fn seed_builtin_loop_patterns(db: &Database) -> Result<(), AppError> {
    use crate::infrastructure::db::models::UpsertLoopPatternRequest;

    let patterns = crate::core::agent::loop_engine::PatternRegistry::built_in_patterns();
    for p in patterns {
        db.upsert_loop_pattern(
            Some(&p.id),
            UpsertLoopPatternRequest {
                name: p.name,
                description: Some(p.description),
                goal: Some(p.goal),
                cadence: Some(p.cadence),
                risk_level: Some(p.risk_level),
                phases: Some(p.phases),
                human_gates: Some(p.human_gates),
                cost_config: Some(p.cost_config),
                skills_required: Some(p.skills_required),
                state_schema: Some(p.state_schema),
                is_active: Some(p.is_active),
            },
        )
        .await?;
    }
    tracing::info!(count = 7, "Seeded builtin loop patterns");
    Ok(())
}

/// 提取内置 novel sources 到 book_sources_dir。
/// 已存在的文件不会被覆盖。
fn ensure_builtin_book_sources(data_dir: &DataDir) -> Result<(), AppError> {
    let dir = data_dir.book_sources_dir();
    crate::features::novel::source::extract_builtin_sources_to_dir(&dir)?;
    Ok(())
}

/// 为每个 agent 角色创建默认 identity 文件（SOUL.md、CONTEXT.md、MEMORY.md）。
/// 已存在的文件不会被覆盖，用户编辑会被保留。
fn ensure_agent_identities(data_dir: &DataDir) -> Result<(), AppError> {
    let agents_dir = data_dir.agents_dir();
    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| AppError::internal(format!("Failed to create agents dir: {}", e)))?;

    for role in AGENT_ROLES {
        let role_dir = agents_dir.join(role);
        std::fs::create_dir_all(&role_dir)
            .map_err(|e| AppError::internal(format!("Failed to create agent dir {}: {}", role, e)))?;

        let soul_path = role_dir.join("SOUL.md");
        if !soul_path.exists() {
            let default = crate::core::agent::identity::default_soul(role);
            std::fs::write(&soul_path, default)
                .map_err(|e| AppError::internal(format!("Failed to write default SOUL.md for {}: {}", role, e)))?;
        }

        let context_path = role_dir.join("CONTEXT.md");
        if !context_path.exists() {
            let default = crate::core::agent::identity::default_context(role);
            std::fs::write(&context_path, default)
                .map_err(|e| AppError::internal(format!("Failed to write default CONTEXT.md for {}: {}", role, e)))?;
        }

        let memory_path = role_dir.join("MEMORY.md");
        if !memory_path.exists() {
            std::fs::write(&memory_path, "# Agent Memory\n\n<!-- Agent accumulates learning notes here across pipeline runs. -->\n")
                .map_err(|e| AppError::internal(format!("Failed to write default MEMORY.md for {}: {}", role, e)))?;
        }

        tracing::debug!(role = role, "Ensured agent identity files");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_data_dir() -> (TempDir, DataDir) {
        let tmp = TempDir::new().unwrap();
        let data_dir = DataDir::new(tmp.path().to_path_buf());
        data_dir.initialize().unwrap();
        (tmp, data_dir)
    }

    #[test]
    fn initialize_creates_all_agent_identity_files() {
        let (_tmp, data_dir) = make_data_dir();
        initialize_app_business_state(&data_dir).unwrap();

        for role in AGENT_ROLES {
            let soul = data_dir.agent_soul_path(role);
            let context = data_dir.agent_context_path(role);
            let memory = data_dir.agent_memory_path(role);
            assert!(soul.exists(), "SOUL.md missing for role {}", role);
            assert!(context.exists(), "CONTEXT.md missing for role {}", role);
            assert!(memory.exists(), "MEMORY.md missing for role {}", role);

            let soul_content = std::fs::read_to_string(&soul).unwrap();
            assert!(!soul_content.is_empty(), "SOUL.md empty for role {}", role);
        }
    }

    #[test]
    fn initialize_is_idempotent_and_preserves_user_edits() {
        let (_tmp, data_dir) = make_data_dir();
        initialize_app_business_state(&data_dir).unwrap();

        // 用户编辑 writer SOUL.md
        let custom = "# My Custom Writer\n\nYou write in noir style.\n";
        std::fs::write(data_dir.agent_soul_path("writer"), custom).unwrap();

        // 再次初始化
        initialize_app_business_state(&data_dir).unwrap();

        let content = std::fs::read_to_string(data_dir.agent_soul_path("writer")).unwrap();
        assert!(content.contains("noir style"), "user edit must be preserved");
    }

    #[test]
    fn initialize_creates_book_sources() {
        let (_tmp, data_dir) = make_data_dir();
        initialize_app_business_state(&data_dir).unwrap();

        let main_sources = data_dir.book_sources_dir().join("main.json");
        assert!(main_sources.exists(), "main.json book source must be created");
    }
}
