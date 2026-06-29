// ============================================================================
// 模块声明 —— 五层分层架构
// ============================================================================
//
// 依赖层次（自上而下，严禁反向）：
//
//   ipc/                 IPC 层（Tauri 命令入口，类型安全契约）
//     ├── core/          核心业务逻辑（agent 引擎、interaction 编排、state、init）
//     │     ├── agent/   AI Agent 核心决策引擎（14 子模块）
//     │     └── interaction/  编排层（session ↔ pipeline 桥接）
//     ├── features/      功能模块层（story/session/version/wiki/novel/radar/user_profile/skill_manager）
//     ├── infrastructure/ 基础设施层（db/llm_client/sandbox/file_storage/state_store/ai_services/middleware 等系统访问）
//     └── shared/        跨层共享类型与纯函数（含 errors 错误处理）
//
// 规则：
// - core/agent 不依赖任何 features（features 编排 agent，agent 不反向依赖）
// - features 之间不横向依赖（跨域编排放 core/interaction/ 或 ipc/commands/）
// - infrastructure 只依赖 shared，不依赖任何 features 或 core/agent
// - shared 只依赖 std（errors 模块仅含纯数据类型）
// - ipc/commands 只做参数提取、验证、委托，不含业务逻辑

pub mod core;
pub mod features;
pub mod infrastructure;
pub mod ipc;
pub mod shared;
use crate::infrastructure::file_storage::data_dir::DataDir;
use crate::infrastructure::db::Database;
use crate::infrastructure::llm_client::ProviderRegistry;
use crate::features::skill_manager::SkillManager;
use crate::infrastructure::sandbox::enforce::SandboxEnforcer;
use crate::infrastructure::sandbox::policy::SandboxPolicy;
use crate::core::state::AppState;
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

            // 业务级初始化（agent 身份文件、内置 novel sources）。
            // 必须在 data_dir.initialize() 之后调用，因为它需要 agents_dir/ 和 book_sources_dir/ 已存在。
            // 上提到 core 层是为了避免 infrastructure 反向依赖 features/ 和 core/agent/。
            crate::core::init::initialize_app_business_state(&data_dir)
                .expect("failed to initialize app business state");

            crate::infrastructure::middleware::logging::init(&data_dir.logs_dir(), &data_dir);

            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Mnemosyne starting");
            tracing::info!(root = %data_dir.root().display(), "App data directory");

            let db_path = data_dir.state_db_path();
            tracing::info!(path = %db_path.display(), "Opening state database");
            let database = tauri::async_runtime::block_on(Database::new(db_path.to_str().unwrap()))
                .expect("failed to open state database");
            tracing::info!("State database initialized (migrations applied)");

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

            let sandbox_policy = SandboxPolicy::restricted();
            let sandbox_enforcer = SandboxEnforcer::new(sandbox_policy, app_dir.clone());
            let memory_store = crate::infrastructure::state_store::memory::MemoryStore::new(app_dir.clone());
            let feedback_store = crate::infrastructure::state_store::feedback::FeedbackStore::new();
            let mcp_server = crate::infrastructure::ai_services::mcp::McpServer::new();
            let app_handle = app.handle().clone();

            // scheduler 在 workspace 打开时懒加载初始化
            let scheduler = tokio::sync::Mutex::new(None::<Arc<crate::core::agent::pipeline::Scheduler>>);

            app.manage(AppState {
                data_dir,
                db: database,
                provider_registry: tokio::sync::Mutex::new(provider_registry),
                skill_manager: tokio::sync::Mutex::new(skill_manager),
                sandbox: tokio::sync::Mutex::new(sandbox_enforcer),
                memory_store,
                feedback_store: Arc::new(tokio::sync::Mutex::new(feedback_store)),
                mcp_server: tokio::sync::Mutex::new(mcp_server),
                scheduler,
                app_handle,
                sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                agent_states: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                main_agent_states: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::greet::greet,
            ipc::commands::settings::set_window_theme,
            ipc::commands::settings::get_log_level,
            ipc::commands::settings::set_log_level,
            ipc::commands::prompts::create_prompt,
            ipc::commands::prompts::list_prompts,
            ipc::commands::prompts::get_prompt,
            ipc::commands::prompts::update_prompt,
            ipc::commands::prompts::delete_prompt,
            ipc::commands::trends::create_trend,
            ipc::commands::trends::list_trends,
            ipc::commands::trends::delete_trend,
            ipc::commands::novels::create_novel,
            ipc::commands::novels::update_novel,
            ipc::commands::novels::list_novels,
            ipc::commands::novels::delete_novel,
            ipc::commands::stats::get_stats,
            ipc::commands::stats::get_daily_activity,
            ipc::commands::workspaces::create_workspace,
            ipc::commands::workspaces::list_workspaces,
            ipc::commands::workspaces::get_workspace,
            ipc::commands::workspaces::delete_workspace,
            ipc::commands::sessions::session_create,
            ipc::commands::sessions::session_list,
            ipc::commands::sessions::session_get,
            ipc::commands::sessions::session_delete,
            ipc::commands::sessions::session_messages,
            ipc::commands::providers::provider_list,
            ipc::commands::providers::provider_models,
            ipc::commands::providers::provider_test_connection,
            ipc::commands::providers::provider_refresh,
            ipc::commands::skills::skill_list,
            ipc::commands::skills::skill_get,
            ipc::commands::skills::skill_create,
            ipc::commands::skills::skill_update,
            ipc::commands::skills::skill_delete,
            ipc::commands::skills::skill_index,
            ipc::commands::skills::skill_refresh,
            ipc::commands::novels_pipeline::novel_create,
            ipc::commands::novels_pipeline::novel_write_next,
            ipc::commands::novels_pipeline::novel_plan,
            ipc::commands::novels_pipeline::novel_audit,
            ipc::commands::novels_pipeline::novel_revise,
            ipc::commands::notifications::send_notification,
            ipc::commands::radar::radar_scan,
            ipc::commands::radar::radar_history,
            ipc::commands::radar::radar_delete,
            ipc::commands::sandbox::sandbox_status,
            ipc::commands::sandbox::sandbox_validate_file,
            ipc::commands::sandbox::sandbox_validate_command,
            ipc::commands::sandbox::sandbox_validate_network,
            ipc::commands::sandbox::sandbox_get_policy,
            ipc::commands::agent::agent_send_message,
            ipc::commands::agent::agent_approve_tool,
            ipc::commands::agent::agent_cancel,
            ipc::commands::agent::agent_compact,
            ipc::commands::agent::agent_restart,
            ipc::commands::agent_session::session_send_message,
            ipc::commands::agent_session::session_cancel,
            ipc::commands::agent_session::session_get_status,
            ipc::commands::agent_session::session_shutdown,
            ipc::commands::agent_session::session_write_next_chapter,
            ipc::commands::agent_session::session_create_book,
            ipc::commands::agent_session::session_approve_tool,
            ipc::commands::agent_session::session_reject_tool,
            ipc::commands::agent_config::list_agents,
            ipc::commands::agent_config::update_agent,
            ipc::commands::agent_config::toggle_agent_status,
            ipc::commands::novels_pipeline::novel_observe,
            ipc::commands::novels_pipeline::novel_reflect,
            ipc::commands::novel_sources::novel_source_list,
            ipc::commands::novel_sources::novel_search,
            ipc::commands::novel_sources::novel_download,
            ipc::commands::novel_sources::novel_list_local,
            ipc::commands::scheduler::scheduler_init,
            ipc::commands::scheduler::scheduler_write_cycle,
            ipc::commands::scheduler::scheduler_status,
            ipc::commands::scheduler::scheduler_list_tasks,
            ipc::commands::scheduler::scheduler_pause,
            ipc::commands::scheduler::scheduler_resume,
            ipc::commands::scheduler::scheduler_stop,
            ipc::commands::scheduler::scheduler_search_rag,
            ipc::commands::scheduler::scheduler_search_memory,
            ipc::commands::scheduler::scheduler_get_lessons,
            ipc::commands::scheduler::scheduler_restore_checkpoint,
            ipc::commands::mcp::mcp_handle_request,
            ipc::commands::mcp::mcp_server_info,
            ipc::commands::mcp::mcp_check_tool_safety,
            ipc::commands::ai_logs::ai_log_llm_calls,
            ipc::commands::ai_logs::ai_log_tool_executions,
            ipc::commands::ai_logs::ai_log_token_usage,
            ipc::commands::ai_logs::ai_log_sandbox_violations,
            ipc::commands::main_agent::main_agent_execute,
            ipc::commands::main_agent::main_agent_respond,
            ipc::commands::main_agent::main_agent_list_sessions,
            ipc::commands::main_agent::main_agent_cancel,
            // Wiki 命令
            ipc::commands::wiki::wiki_list_entries,
            ipc::commands::wiki::wiki_get_entry,
            ipc::commands::wiki::wiki_create_entry,
            ipc::commands::wiki::wiki_update_entry,
            ipc::commands::wiki::wiki_delete_entry,
            ipc::commands::wiki::wiki_get_graph,
            ipc::commands::wiki::wiki_create_link,
            ipc::commands::wiki::wiki_delete_link,
            ipc::commands::wiki::wiki_search,
            // Version 命令
            ipc::commands::version::version_list,
            ipc::commands::version::version_get,
            ipc::commands::version::version_get_latest,
            ipc::commands::version::version_diff,
            ipc::commands::version::version_diff_latest,
            ipc::commands::version::version_restore,
            ipc::commands::version::version_save,
            // Kanban 命令
            ipc::commands::kanban::kanban_create_task,
            ipc::commands::kanban::kanban_get_tasks,
            ipc::commands::kanban::kanban_update_task,
            ipc::commands::kanban::kanban_delete_task,
            ipc::commands::kanban::kanban_reorder_tasks,
            ipc::commands::kanban::kanban_get_columns,
            ipc::commands::kanban::kanban_create_column,
            ipc::commands::kanban::kanban_update_column,
            ipc::commands::kanban::kanban_delete_column,
            // Loop Engineering 命令
            ipc::commands::loop_engine::loop_create_state,
            ipc::commands::loop_engine::loop_get_states,
            ipc::commands::loop_engine::loop_update_state,
            ipc::commands::loop_engine::loop_delete_state,
            ipc::commands::loop_engine::loop_run_cycle,
            ipc::commands::loop_engine::loop_get_run_logs,
            ipc::commands::loop_engine::loop_get_patterns,
            ipc::commands::loop_engine::loop_upsert_pattern,
            ipc::commands::loop_engine::loop_pause,
            ipc::commands::loop_engine::loop_resume,
            ipc::commands::loop_engine::loop_get_budget_status,
            // Memory 命令
            ipc::commands::memory::memory_list,
            ipc::commands::memory::memory_search,
            ipc::commands::memory::memory_stats,
            ipc::commands::memory::memory_format_context,
            ipc::commands::memory::memory_create,
            ipc::commands::memory::memory_update,
            ipc::commands::memory::memory_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
