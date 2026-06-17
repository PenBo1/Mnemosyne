pub mod app;
pub mod domain;
pub mod infra;

pub mod errors;
pub mod middleware;

use crate::infra::data_dir::DataDir;
use crate::infra::db::Database;
use crate::infra::llm::ProviderRegistry;
use crate::infra::skill::SkillManager;
use crate::infra::sandbox::enforce::SandboxEnforcer;
use crate::infra::sandbox::policy::SandboxPolicy;
use crate::app::state::AppState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");

            let data_dir = DataDir::new(app_dir.clone());
            data_dir.initialize().expect("failed to initialize data directory");

            middleware::logging::init(&data_dir.logs_dir(), &data_dir);

            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Mnemosyne starting");
            tracing::info!(root = %data_dir.root().display(), "App data directory");

            let db_path = data_dir.state_db_path();
            tracing::info!(path = %db_path.display(), "Opening state database");
            let database = Database::new(db_path.to_str().unwrap())
                .expect("failed to open state database");
            tracing::info!("State database initialized");

            let feedback_db_path = data_dir.feedback_db_path();
            tracing::info!(path = %feedback_db_path.display(), "Opening feedback database");
            let feedback_db = Database::new_feedback(feedback_db_path.to_str().unwrap())
                .expect("failed to open feedback database");
            tracing::info!("Feedback database initialized");

            let provider_registry = ProviderRegistry::new(&data_dir);
            tracing::info!(count = provider_registry.list_providers().len(), "Providers loaded");

            let mut skill_manager = SkillManager::new();
            if let Some(home) = dirs::home_dir() {
                skill_manager.add_dir(home.join(".mnemosyne").join("skills"));
            }
            skill_manager.add_dir(data_dir.skills_dir());
            if let Err(e) = skill_manager.discover() {
                tracing::warn!("Failed to discover skills: {}", e);
            }
            let skill_count = skill_manager.list().len();
            tracing::info!(count = skill_count, "Skills discovered");

            let db_arc = Arc::new(tokio::sync::Mutex::new(database));
            let feedback_db_arc = Arc::new(tokio::sync::Mutex::new(feedback_db));
            let sandbox_policy = SandboxPolicy::restricted();
            let sandbox_enforcer = SandboxEnforcer::new(sandbox_policy, app_dir.clone());
            let app_handle = app.handle().clone();

            app.manage(AppState {
                data_dir,
                db: db_arc,
                feedback_db: feedback_db_arc,
                provider_registry: tokio::sync::Mutex::new(provider_registry),
                skill_manager: tokio::sync::Mutex::new(skill_manager),
                sandbox: tokio::sync::Mutex::new(sandbox_enforcer),
                app_handle,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::greet::greet,
            app::commands::settings::set_window_theme,
            app::commands::settings::get_log_level,
            app::commands::settings::set_log_level,
            app::commands::prompts::create_prompt,
            app::commands::prompts::list_prompts,
            app::commands::prompts::get_prompt,
            app::commands::prompts::update_prompt,
            app::commands::prompts::delete_prompt,
            app::commands::trends::create_trend,
            app::commands::trends::list_trends,
            app::commands::trends::delete_trend,
            app::commands::novels::create_novel,
            app::commands::novels::update_novel,
            app::commands::novels::list_novels,
            app::commands::novels::delete_novel,
            app::commands::stats::get_stats,
            app::commands::stats::get_daily_activity,
            app::commands::workspaces::create_workspace,
            app::commands::workspaces::list_workspaces,
            app::commands::workspaces::get_workspace,
            app::commands::workspaces::delete_workspace,
            app::commands::sessions::session_create,
            app::commands::sessions::session_list,
            app::commands::sessions::session_get,
            app::commands::sessions::session_delete,
            app::commands::sessions::session_messages,
            app::commands::providers::provider_list,
            app::commands::providers::provider_models,
            app::commands::providers::provider_test_connection,
            app::commands::providers::provider_refresh,
            app::commands::skills::skill_list,
            app::commands::skills::skill_get,
            app::commands::skills::skill_create,
            app::commands::skills::skill_update,
            app::commands::skills::skill_delete,
            app::commands::skills::skill_index,
            app::commands::skills::skill_refresh,
            app::commands::novels_pipeline::novel_create,
            app::commands::novels_pipeline::novel_write_next,
            app::commands::novels_pipeline::novel_plan,
            app::commands::novels_pipeline::novel_audit,
            app::commands::novels_pipeline::novel_revise,
            app::commands::notifications::send_notification,
            app::commands::radar::radar_scan,
            app::commands::radar::radar_history,
            app::commands::radar::radar_delete,
            app::commands::sandbox::sandbox_status,
            app::commands::sandbox::sandbox_validate_file,
            app::commands::sandbox::sandbox_validate_command,
            app::commands::sandbox::sandbox_validate_network,
            app::commands::sandbox::sandbox_get_policy,
            app::commands::agent::agent_send_message,
            app::commands::agent::agent_approve_tool,
            app::commands::agent::agent_cancel,
            app::commands::agent::agent_compact,
            app::commands::agent::agent_restart,
            app::commands::agent_config::list_agents,
            app::commands::agent_config::update_agent,
            app::commands::agent_config::toggle_agent_status,
            app::commands::novels_pipeline::novel_observe,
            app::commands::novels_pipeline::novel_reflect,
            app::commands::novel_sources::novel_source_list,
            app::commands::novel_sources::novel_search,
            app::commands::novel_sources::novel_download,
            app::commands::novel_sources::novel_list_local,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
