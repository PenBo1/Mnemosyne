
use crate::core::agent::identity::IdentityKind;
use crate::core::agent::prompts;
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

pub fn initialize_app_business_state(data_dir: &DataDir) -> Result<(), AppError> {
    tracing::info!("Initializing app business state");

    let sources_json = include_str!("../../resources/novel_sources.json");
    let sources_path = data_dir.root().join("novel_sources.json");

    if !sources_path.exists() {
        std::fs::write(&sources_path, sources_json)
            .map_err(|_e| AppError::file_write_error(sources_path.display().to_string()))?;
        tracing::info!(path = %sources_path.display(), "Extracted built-in novel sources");
    }

    // 为 main agent 生成默认身份文件(SOUL/CONTEXT/MEMORY.md)
    ensure_default_identity_files(data_dir, prompts::MAIN_ROLE)?;

    tracing::info!("App business state initialized");
    Ok(())
}

/// 初始化 builtin loop patterns(仅当不存在时写入)。
///
/// 在 DbState 创建后、app.manage 前调用,确保前端 loop_get_patterns 能返回 builtin patterns。
pub fn seed_builtin_loop_patterns(db: &Database) -> Result<(), AppError> {
    crate::application::loop_engine::ensure_builtin_patterns(db)?;
    tracing::info!("Builtin loop patterns ensured");
    Ok(())
}

/// 为指定 role 生成默认身份文件(已存在则跳过,不覆盖用户编辑)。
fn ensure_default_identity_files(data_dir: &DataDir, role: &str) -> Result<(), AppError> {
    let role_dir = data_dir.agents_dir().join(role);
    std::fs::create_dir_all(&role_dir)
        .map_err(|e| AppError::internal(format!("Failed to create agent role dir: {}", e)))?;

    for kind in [IdentityKind::Soul, IdentityKind::Context, IdentityKind::Memory] {
        let path = role_dir.join(kind.filename());
        if path.exists() {
            continue;
        }
        std::fs::write(&path, kind.default_content_for(role)).map_err(|e| {
            AppError::internal(format!(
                "Failed to write {}: {}",
                path.display(),
                e
            ))
        })?;
        tracing::info!(
            role = role,
            kind = kind.filename(),
            path = %path.display(),
            "Generated default agent identity file"
        );
    }
    Ok(())
}
