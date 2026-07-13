pub mod application;
pub mod core;
pub mod domain;
pub mod infrastructure;
pub mod security_kernel;
pub mod shared;

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::llm::state::LlmState;
use crate::application::skill::state::SkillState;
use crate::infrastructure::sandbox::state::SandboxState;
use crate::infrastructure::memory::state::MemoryState;
use crate::domain::feedback::state::FeedbackState;
use crate::infrastructure::workspace::state::WorkspaceState;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use crate::security_kernel::SecurityKernelState;
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
            let data_dir = DataDir::new(app_dir);
            data_dir.initialize().expect("failed to initialize data directory");

            crate::application::init::initialize_app_business_state(&data_dir)
                .expect("failed to initialize app business state");

            crate::infrastructure::fs::fs_utils::init_logging(&data_dir.logs_dir(), &data_dir);
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Mnemosyne starting");
            tracing::info!(root = %data_dir.root().display(), "App data directory");

            // 先创建 DbState,以便克隆 Database 给 AgentEngine 与 SecurityKernel 持有
            let db_state = DbState::new(data_dir.clone());
            // DataDir 作为 State 共享给 pipeline 命令
            app.manage(data_dir.clone());
            let db_for_agent = db_state.db.clone();
            let db_for_kernel = db_state.db.clone();
            app.manage(db_state);
            app.manage(LlmState::new(data_dir.clone()));
            app.manage(SkillState::new(&data_dir));
            app.manage(SandboxState::new(data_dir.root().to_path_buf()));
            app.manage(MemoryState::new(data_dir.root().to_path_buf()));
            app.manage(FeedbackState::new());
            app.manage(crate::infrastructure::secrets::SecretsState::default());
            app.manage(WorkspaceState::new());
            app.manage(WorkspaceRegistry::new());
            app.manage(SecurityKernelState::with_db(db_for_kernel));

            // Initialize agent engine from LLM provider registry
            let agent_engine = {
                let registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&data_dir);
                let workspace_root = std::env::current_dir().unwrap_or_else(|_| data_dir.root().to_path_buf());
                crate::core::agent::engine::AgentEngine::new(registry, db_for_agent, workspace_root)
            };

            // Pipeline SchedulerState（需要 AgentEngine + DataDir.books_dir）
            let scheduler_config = crate::domain::pipeline::scheduler::SchedulerConfig::default();
            let pipeline_config = crate::domain::pipeline::runner::PipelineConfig {
                books_dir: data_dir.books_dir(),
                ..Default::default()
            };
            let scheduler_state = crate::domain::pipeline::scheduler::SchedulerState::new(
                pipeline_config,
                scheduler_config,
                agent_engine.clone(),
            );
            app.manage(scheduler_state);

            app.manage(crate::core::agent::commands::AgentState::new(agent_engine));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::infrastructure::settings::set_window_theme,
            crate::infrastructure::settings::get_data_dir_path,
            crate::infrastructure::settings::get_log_level,
            crate::infrastructure::settings::set_log_level,
            crate::infrastructure::settings::get_git_enabled,
            crate::infrastructure::settings::set_git_enabled,
            crate::infrastructure::settings::list_log_files,
            crate::infrastructure::settings::read_log_file,
            crate::infrastructure::settings::clear_log_file,
            crate::infrastructure::prompts::create_prompt,
            crate::infrastructure::prompts::list_prompts,
            crate::infrastructure::prompts::get_prompt,
            crate::infrastructure::prompts::update_prompt,
            crate::infrastructure::prompts::delete_prompt,
            crate::infrastructure::db::commands::create_trend,
            crate::infrastructure::db::commands::list_trends,
            crate::infrastructure::db::commands::delete_trend,
            crate::domain::novel::commands::novel_create,
            crate::domain::novel::commands::novel_update,
            crate::domain::novel::commands::novel_list,
            crate::domain::novel::commands::novel_get,
            crate::domain::novel::commands::novel_delete,
            crate::domain::novel::commands::novel_source_list,
            crate::domain::novel::commands::novel_source_toggle,
            crate::infrastructure::stats::get_stats,
            crate::infrastructure::stats::get_daily_activity,
            crate::infrastructure::stats::get_ai_stats,
            crate::security_kernel::commands::audit_events_query,
            crate::security_kernel::commands::audit_event_stats,
            crate::application::workspace::commands::create_workspace,
            crate::application::workspace::commands::list_workspaces,
            crate::application::workspace::commands::get_workspace,
            crate::application::workspace::commands::delete_workspace,
            crate::application::workspace::commands::touch_workspace,
            crate::application::session::commands::session_create,
            crate::application::session::commands::session_list,
            crate::application::session::commands::session_get,
            crate::application::session::commands::session_delete,
            crate::application::session::commands::session_messages,
            crate::application::session::commands::message_create,
            crate::application::skill::commands::skill_list,
            crate::application::skill::commands::skill_get,
            crate::application::skill::commands::skill_create,
            crate::application::skill::commands::skill_update,
            crate::application::skill::commands::skill_delete,
            crate::application::skill::commands::skill_index,
            crate::application::skill::commands::skill_refresh,
            crate::domain::story::commands::story_state_get,
            crate::domain::story::commands::story_state_save,
            crate::domain::story::commands::hook_update_status,
            crate::domain::story::commands::query_facts_at_chapter,
            crate::domain::story::commands::list_recent_chapter_summaries,
            crate::domain::story::commands::list_chapter_summaries_range,
            crate::infrastructure::notifications::send_notification,
            crate::domain::radar::commands::radar_scan_list,
            crate::domain::radar::commands::radar_scan_delete,
            crate::domain::radar::commands::radar_scan_create,
            crate::domain::radar::commands::radar_scan,
            crate::infrastructure::sandbox::commands::sandbox_status,
            crate::infrastructure::sandbox::commands::sandbox_validate_path,
            crate::infrastructure::sandbox::commands::sandbox_validate_command,
            crate::infrastructure::sandbox::commands::sandbox_validate_url,
            crate::infrastructure::sandbox::commands::sandbox_get_policy,
            crate::infrastructure::llm::commands::llm_model_list,
            crate::infrastructure::providers::provider_list,
            crate::infrastructure::providers::provider_models,
            crate::infrastructure::providers::provider_test_connection,
            crate::infrastructure::providers::provider_refresh,
            crate::infrastructure::llm::embedding::commands::embedding_get_config,
            crate::infrastructure::llm::embedding::commands::embedding_set_config,
            crate::infrastructure::llm::embedding::commands::embedding_test,
            crate::infrastructure::llm::embedding::commands::embedding_index_doc,
            crate::infrastructure::llm::embedding::commands::embedding_search,
            crate::infrastructure::llm::embedding::commands::embedding_delete_doc,
            crate::infrastructure::llm::embedding::commands::embedding_stats,
            crate::domain::wiki::commands::wiki_list_entries,
            crate::domain::wiki::commands::wiki_get_entry,
            crate::domain::wiki::commands::wiki_create_entry,
            crate::domain::wiki::commands::wiki_update_entry,
            crate::domain::wiki::commands::wiki_delete_entry,
            crate::domain::wiki::commands::wiki_get_graph,
            crate::domain::wiki::commands::wiki_create_link,
            crate::domain::wiki::commands::wiki_delete_link,
            crate::domain::wiki::commands::wiki_search,
            crate::domain::version::commands::version_list,
            crate::domain::version::commands::version_get,
            crate::domain::version::commands::version_get_latest,
            crate::domain::version::commands::version_save,
            crate::domain::version::commands::version_diff,
            crate::domain::version::commands::version_diff_latest,
            crate::domain::version::commands::version_restore,
            crate::infrastructure::memory::commands::memory_list,
            crate::infrastructure::memory::commands::memory_search,
            crate::infrastructure::memory::commands::memory_stats,
            crate::infrastructure::memory::commands::memory_format_context,
            crate::infrastructure::memory::commands::memory_create,
            crate::infrastructure::memory::commands::memory_update,
            crate::infrastructure::memory::commands::memory_delete,
            crate::infrastructure::fs::commands::fs_read_file,
            crate::infrastructure::fs::commands::fs_write_file,
            crate::infrastructure::fs::commands::fs_list_directory,
            crate::infrastructure::fs::commands::fs_create_directory,
            crate::infrastructure::fs::commands::fs_delete_file,
            crate::infrastructure::fs::commands::fs_exists,
            crate::infrastructure::fs::commands::fs_copy_file,
            crate::domain::git::commands::git_check_installed,
            crate::domain::git::commands::git_install,
            crate::domain::git::commands::git_init,
            crate::domain::git::commands::git_status,
            crate::domain::git::commands::git_log,
            crate::domain::git::commands::git_diff,
            crate::domain::git::commands::git_stage,
            crate::domain::git::commands::git_commit,
            crate::domain::git::commands::git_rollback,
            crate::domain::git::commands::git_get_config,
            crate::domain::git::commands::git_set_config,
            crate::infrastructure::net::lm_ping,
            crate::core::agent::commands::chat_send_message,
            crate::core::agent::commands::chat_stop,
            crate::core::agent::commands::chat_tool_respond,
            crate::domain::pipeline::commands::pipeline_init_book,
            crate::domain::pipeline::commands::pipeline_revise_foundation,
            crate::domain::pipeline::commands::pipeline_plan_chapter,
            crate::domain::pipeline::commands::pipeline_compose_chapter,
            crate::domain::pipeline::commands::pipeline_write_draft,
            crate::domain::pipeline::commands::pipeline_audit_draft,
            crate::domain::pipeline::commands::pipeline_revise_draft,
            crate::domain::pipeline::commands::pipeline_write_next_chapter,
            crate::domain::pipeline::commands::pipeline_list_chapters,
            crate::domain::pipeline::commands::pipeline_get_chapter,
            crate::domain::pipeline::commands::pipeline_consolidate,
            crate::domain::pipeline::commands::pipeline_list_books,
            crate::domain::pipeline::commands::pipeline_get_book,
            crate::domain::pipeline::commands::pipeline_read_truth_file,
            crate::domain::pipeline::commands::pipeline_scheduler_start,
            crate::domain::pipeline::commands::pipeline_scheduler_stop,
            crate::domain::pipeline::commands::pipeline_scheduler_status,
            crate::domain::pipeline::commands::pipeline_scheduler_trigger_write,
            crate::domain::pipeline::commands::pipeline_scheduler_trigger_radar,
            crate::domain::pipeline::commands::pipeline_scheduler_resume_book,
            crate::domain::pipeline::commands::pipeline_scheduler_is_book_paused,
            crate::domain::pipeline::commands::pipeline_scheduler_subscribe,
            // Phase 6 扩展命令
            crate::domain::pipeline::commands::pipeline_short_fiction_run,
            crate::domain::pipeline::commands::pipeline_fanfic_import,
            crate::domain::pipeline::commands::pipeline_script_run,
            crate::domain::pipeline::commands::pipeline_storyboard_run,
            crate::domain::pipeline::commands::pipeline_interactive_film_run,
            crate::domain::pipeline::commands::pipeline_story_graph_validate,
            crate::domain::pipeline::commands::pipeline_story_graph_paths,
            crate::domain::pipeline::commands::pipeline_story_graph_apply_delta,
            crate::infrastructure::secrets::secrets_get,
            crate::infrastructure::secrets::secrets_set,
            crate::infrastructure::secrets::secrets_delete,
            crate::infrastructure::secrets::secrets_get_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}