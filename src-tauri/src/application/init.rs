
//! ═══════════════════════════════════════════════════════════════════════════
//! Init - 应用初始化模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 提供应用启动时的业务状态初始化功能，包括默认身份文件生成、
//! 内置循环模式种子、用户画像提供者构造等。

use std::sync::Arc;
use std::time::Instant;

use crate::core::agent::identity::IdentityKind;
use crate::core::agent::prompts;
use crate::core::agent::user_profile::{UserProfileProvider, UserProfileSnapshot};
use crate::domain::user::store::UserProfileStore;
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::fs::data_dir::DataDir;
use crate::shared::error::AppError;

pub fn initialize_app_business_state(data_dir: &DataDir) -> Result<(), AppError> {
    let start = Instant::now();
    tracing::info!("Initializing app business state");

    let sources_json = include_str!("../../resources/novel_sources.json");
    let sources_path = data_dir.root().join("novel_sources.json");

    if !sources_path.exists() {
        std::fs::write(&sources_path, sources_json)
            .map_err(|e| {
                tracing::error!(
                    path = %sources_path.display(),
                    error = %e,
                    "Failed to extract built-in novel sources"
                );
                AppError::file_write_error(sources_path.display().to_string())
            })?;
        tracing::info!(path = %sources_path.display(), "Extracted built-in novel sources");
    }

    for role in prompts::ALL_ROLES {
        if let Err(e) = ensure_default_identity_files(data_dir, role) {
            tracing::error!(role = %role, error = %e, "Failed to ensure default identity files");
            return Err(e);
        }
    }

    tracing::info!(
        duration_ms = start.elapsed().as_millis() as u64,
        roles_count = prompts::ALL_ROLES.len(),
        "App business state initialized"
    );
    Ok(())
}

pub fn seed_builtin_loop_patterns(db: &Database) -> Result<(), AppError> {
    let start = Instant::now();
    tracing::info!("Seeding builtin loop patterns");

    crate::application::loop_engine::ensure_builtin_patterns(db).map_err(|e| {
        tracing::error!(error = %e, "Failed to ensure builtin loop patterns");
        e
    })?;

    tracing::info!(
        duration_ms = start.elapsed().as_millis() as u64,
        "Builtin loop patterns ensured"
    );
    Ok(())
}

// ── UserProfileProvider 实现 ────────────────────────────────────────────────────────
//
// application 层是 core/agent 与 domain::user 之间的编排边界：
// 此处读取领域 UserProfile 并转换为 core/agent 的快照类型，注入 AgentState。

/// 基于 UserProfileStore 的用户画像提供者。
struct UserProfileProviderImpl {
    data_dir: DataDir,
}

impl UserProfileProvider for UserProfileProviderImpl {
    fn load_user_profile(&self) -> UserProfileSnapshot {
        // UserProfileStore::new 内部读盘，缺失/解析失败时回退到 default profile
        // （与原 engine 内联逻辑行为一致）。
        // 注意：get() 返回 &UserProfile，必须先绑定 store 让临时值生命周期足够。
        let store = UserProfileStore::new(self.data_dir.root());
        let profile = store.get();
        // 两侧 serde 形状一致，通过 Value 转换避免手写字段映射。
        // forward-compatible：domain 新增字段会被 serde 默认忽略，不影响快照。
        serde_json::to_value(profile)
            .and_then(serde_json::from_value)
            .unwrap_or_default()
    }
}

/// 构造用户画像提供者，注入 AgentState。
pub fn build_user_profile_provider(data_dir: DataDir) -> Arc<dyn UserProfileProvider> {
    Arc::new(UserProfileProviderImpl { data_dir })
}

fn ensure_default_identity_files(data_dir: &DataDir, role: &str) -> Result<(), AppError> {
    let start = Instant::now();
    tracing::debug!(role = %role, "Ensuring default identity files");

    let role_dir = data_dir.agents_dir().join(role);
    std::fs::create_dir_all(&role_dir).map_err(|e| {
        tracing::error!(
            role = %role,
            path = %role_dir.display(),
            error = %e,
            "Failed to create agent role dir"
        );
        AppError::internal(format!("Failed to create agent role dir: {}", e))
    })?;

    for kind in [IdentityKind::Soul, IdentityKind::Context, IdentityKind::Memory] {
        let path = role_dir.join(kind.filename());
        if path.exists() {
            tracing::debug!(
                role = %role,
                kind = kind.filename(),
                "Identity file already exists, skipping"
            );
            continue;
        }
        std::fs::write(&path, kind.default_content_for(role)).map_err(|e| {
            tracing::error!(
                role = %role,
                kind = kind.filename(),
                path = %path.display(),
                error = %e,
                "Failed to write identity file"
            );
            AppError::internal(format!("Failed to write {}: {}", path.display(), e))
        })?;
        tracing::info!(
            role = %role,
            kind = kind.filename(),
            path = %path.display(),
            "Generated default agent identity file"
        );
    }

    tracing::debug!(
        role = %role,
        duration_ms = start.elapsed().as_millis() as u64,
        "Identity files ensured"
    );
    Ok(())
}
