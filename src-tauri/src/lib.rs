//! ═══════════════════════════════════════════════════════════════════════════
//! lib - Tauri 应用入口点
//! ═══════════════════════════════════════════════════════════════════════════

// ── 全局 Clippy 抑制配置 ──────────────────────────────────────────────────

#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::should_implement_trait)]

// ── 模块声明 ────────────────────────────────────────────────────────────────

pub mod application;
pub mod core;
pub mod domain;
pub mod infrastructure;
pub mod security_kernel;
pub mod shared;

// ── 导入 ────────────────────────────────────────────────────────────────────

use crate::infrastructure::fs::data_dir::DataDir;
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::llm::state::LlmState;
use crate::application::skill::state::SkillState;
use crate::infrastructure::sandbox::state::SandboxState;
use crate::infrastructure::memory::state::MemoryState;
use crate::infrastructure::project_memory::state::ProjectMemoryState;
use crate::infrastructure::tool_limits::state::ToolLimitsState;
use crate::domain::feedback::state::FeedbackState;
use crate::infrastructure::workspace::state::WorkspaceState;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use crate::infrastructure::mcp::state::McpState;
use crate::security_kernel::SecurityKernelState;
use crate::security_kernel::hooks::HookEngineState;
use tauri::{Manager, Listener};
use tokio_util::sync::CancellationToken;

// ── 关闭令牌 ────────────────────────────────────────────────────────────────

/// 应用关闭信号，用于优雅停止后台任务
pub struct ShutdownToken(pub CancellationToken);

impl Default for ShutdownToken {
    fn default() -> Self {
        Self(CancellationToken::new())
    }
}

// ── 应用入口 ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                window.show().ok();
                window.set_focus().ok();
            }
        }))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label != "main" {
                    return;
                }

                let close_behavior = window
                    .state::<DataDir>()
                    .root()
                    .join("config.json");

                let minimize_to_tray = match std::fs::read_to_string(&close_behavior) {
                    Ok(content) => {
                        serde_json::from_str::<serde_json::Value>(&content)
                            .ok()
                            .and_then(|v| v.get("ui").and_then(|ui| ui.get("closeBehavior")).cloned())
                            .map(|v| v.as_str() == Some("minimizeToTray"))
                            .unwrap_or(false)
                    }
                    Err(_) => false,
                };

                if minimize_to_tray {
                    window.hide().ok();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let data_dir = DataDir::new(app_dir);
            data_dir.initialize()?;

            crate::application::init::initialize_app_business_state(&data_dir)?;

            crate::infrastructure::fs::fs_utils::init_logging(&data_dir.logs_dir());
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "Mnemosyne starting");
            tracing::info!("[agent] chat pipeline v2 initialized");
            tracing::info!(root = %data_dir.root().display(), "App data directory");

            // ── 数据库初始化 ────────────────────────────────────────────────────

            let db_state = DbState::new(data_dir.clone())?;
            app.manage(data_dir.clone());
            let db_for_agent = db_state.db.clone();
            let db_for_kernel = db_state.db.clone();
            let db_for_memory = db_state.db.clone();
            let db_for_summary = db_state.db.clone();
            let db_for_registry = db_state.db.clone();

            if let Err(e) = crate::application::init::seed_builtin_loop_patterns(&db_state.db) {
                tracing::error!(error = %e, "Failed to seed builtin loop patterns (non-fatal, continuing startup)");
            }
            app.manage(db_state);

            // ── 基础设施层 State 注册 ──────────────────────────────────────────

            app.manage(LlmState::new(data_dir.clone()));
            app.manage(SkillState::new(&data_dir));
            app.manage(crate::application::skill::capability_commands::CapabilityState::default());
            app.manage(SandboxState::new(data_dir.root().to_path_buf()));
            app.manage(MemoryState::new(db_for_memory, data_dir.root().to_path_buf()));
            app.manage(ProjectMemoryState::new(data_dir.clone()));
            app.manage(ToolLimitsState::new(data_dir.clone()));
            app.manage(FeedbackState::new());
            app.manage(crate::infrastructure::secrets::SecretsState::default());
            app.manage(McpState::new(data_dir.clone()));
            app.manage(WorkspaceState::new());

            // ── WorkspaceRegistry 初始化与预授权 ──────────────────────────────

            let workspace_registry = WorkspaceRegistry::new();
            if let Err(e) = workspace_registry.authorize(data_dir.root()) {
                tracing::warn!(error = %e, "Failed to authorize app data dir on startup");
            }
            match db_for_registry.list_workspaces() {
                Ok(workspaces) => {
                    for ws in &workspaces {
                        if ws.path.is_empty() {
                            continue;
                        }
                        let path_buf = std::path::PathBuf::from(&ws.path);
                        if path_buf.exists() {
                            if let Err(e) = workspace_registry.authorize(&path_buf) {
                                tracing::warn!(workspace_id = %ws.id, error = %e, "Failed to authorize workspace on startup");
                            }
                        }
                    }
                    tracing::info!(count = workspaces.len(), "Workspaces re-authorized on startup");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to list workspaces for re-authorization");
                }
            }
            app.manage(workspace_registry);

            // ── SecurityKernel 初始化 ──────────────────────────────────────────

            app.manage(SecurityKernelState::with_audit_handler(Box::new(
                crate::infrastructure::db::audit_handler::DbAuditHandler::new(db_for_kernel),
            )));
            crate::security_kernel::audit::register_tauri_emit_handler(app.handle());

            let hook_engine_arc = {
                let kernel_state = app.state::<SecurityKernelState>();
                kernel_state.kernel().hook_engine().clone()
            };
            app.manage(HookEngineState::from_arc(hook_engine_arc.clone()));

            // ── AgentEngine 初始化 ─────────────────────────────────────────────

            let agent_engine = {
                let registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&data_dir);
                let workspace_root = std::env::current_dir().unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "current_dir() failed, falling back to app data dir");
                    data_dir.root().to_path_buf()
                });
                let engine = crate::core::agent::engine::AgentEngine::new(
                    registry,
                    db_for_agent,
                    data_dir.clone(),
                    workspace_root,
                    Some(hook_engine_arc),
                );
                use crate::application::bridges::{
                    BookEditOpsImpl, PipelineDelegateOpsImpl, ResearchOpsImpl,
                };
                use crate::core::agent::tools::book_ops::{
                    BookEditOps, PipelineDelegateOps, ResearchOps,
                };
                let book_edit_ops: std::sync::Arc<dyn BookEditOps> =
                    std::sync::Arc::new(BookEditOpsImpl::new(data_dir.clone()));
                let pipeline_delegate_ops: std::sync::Arc<dyn PipelineDelegateOps> =
                    std::sync::Arc::new(PipelineDelegateOpsImpl::new(data_dir.clone()));
                let research_ops: std::sync::Arc<dyn ResearchOps> =
                    std::sync::Arc::new(ResearchOpsImpl::new(data_dir.clone()));
                engine.with_tool_ops(book_edit_ops, pipeline_delegate_ops, research_ops)
            };

            // ── Pipeline Scheduler 初始化 ────────────────────────────────────

            let scheduler_config = crate::domain::pipeline::scheduler::SchedulerConfig::default();
            let pipeline_config = crate::domain::pipeline::runner::PipelineConfig {
                books_dir: data_dir.books_dir(),
                ..Default::default()
            };
            let radar_ops: std::sync::Arc<dyn crate::domain::pipeline::radar_ops::RadarScanOps> =
                std::sync::Arc::new(crate::application::bridges::RadarScanOpsImpl::new());
            let scheduler_state = crate::domain::pipeline::scheduler::SchedulerState::new(
                pipeline_config,
                scheduler_config,
                agent_engine.clone(),
                radar_ops,
            );
            app.manage(scheduler_state);

            // ── InteractionPipelineOps 注入 ──────────────────────────────────

            app.manage(crate::domain::interaction::pipeline_ops::InteractionPipelineOpsState {
                ops: std::sync::Arc::new(
                    crate::application::bridges::InteractionPipelineOpsImpl::new(data_dir.clone()),
                ),
            });

            // ── 每日摘要任务 State ─────────────────────────────────────────────

            app.manage(crate::core::agent::daily_summary::DailySummaryState::new(
                agent_engine.clone(),
                db_for_summary,
                data_dir.clone(),
            ));

            // ── AgentState 与 UserProfile 注入 ────────────────────────────────

            let user_profile_provider =
                crate::application::init::build_user_profile_provider(data_dir.clone());
            app.manage(crate::core::agent::commands::AgentState::new(
                agent_engine,
                Some(user_profile_provider),
            ));

            // ── Agent Registry ────────────────────────────────────────────────

            app.manage(crate::core::agent::registry::AgentRegistryState::new());

            // ── 关闭令牌注册 ────────────────────────────────────────────────

            let shutdown_token = ShutdownToken::default();
            app.manage(shutdown_token);

            // ── SecurityKernel 定期清理任务 ──────────────────────────────────

            {
                let kernel_state = app.state::<SecurityKernelState>().inner().clone();
                let shutdown = app.state::<ShutdownToken>().inner().0.clone();
                tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    interval.tick().await;
                    loop {
                        tokio::select! {
                            _ = shutdown.cancelled() => {
                                tracing::info!("SecurityKernel cleanup task received shutdown signal");
                                break;
                            }
                            _ = interval.tick() => {
                                kernel_state.cleanup();
                            }
                        }
                    }
                });
            }

            // ── 前端关闭事件监听 ─────────────────────────────────────────────

            {
                let app_handle = app.handle().clone();
                let _ = app.listen("allow-close", move |_event| {
                    if let Some(shutdown_token) = app_handle.try_state::<ShutdownToken>() {
                        shutdown_token.0.cancel();
                    }
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.close();
                    }
                });
            }

            // ── 系统托盘初始化 ───────────────────────────────────────────────

            if let Err(e) = crate::infrastructure::tray::build_tray(app.handle()) {
                tracing::error!(error = %e, "Failed to initialize system tray");
            }

            Ok(())
        })
        // ── IPC 命令注册 ────────────────────────────────────────────────────

        .invoke_handler(tauri::generate_handler![
            // Settings
            crate::infrastructure::settings::set_window_theme,
            crate::infrastructure::settings::get_data_dir_path,
            crate::infrastructure::settings::get_log_level,
            crate::infrastructure::settings::set_log_level,
            crate::infrastructure::settings::get_git_enabled,
            crate::infrastructure::settings::set_git_enabled,
            crate::infrastructure::settings::list_log_files,
            crate::infrastructure::settings::read_log_file,
            crate::infrastructure::settings::clear_log_file,
            // Prompts
            crate::infrastructure::prompts::create_prompt,
            crate::infrastructure::prompts::list_prompts,
            crate::infrastructure::prompts::get_prompt,
            crate::infrastructure::prompts::update_prompt,
            crate::infrastructure::prompts::delete_prompt,
            // Trends
            crate::infrastructure::db::commands::create_trend,
            crate::infrastructure::db::commands::list_trends,
            crate::infrastructure::db::commands::delete_trend,
            // Novel
            crate::domain::novel::commands::novel_create,
            crate::domain::novel::commands::novel_update,
            crate::domain::novel::commands::novel_list,
            crate::domain::novel::commands::novel_get,
            crate::domain::novel::commands::novel_delete,
            crate::domain::novel::commands::novel_source_list,
            crate::domain::novel::commands::novel_source_toggle,
            crate::domain::novel::commands::novel_search,
            crate::domain::novel::commands::novel_download,
            crate::domain::novel::commands::novel_list_local,
            // Stats
            crate::infrastructure::stats::get_stats,
            crate::infrastructure::stats::get_daily_activity,
            crate::infrastructure::stats::get_ai_stats,
            crate::infrastructure::stats::get_usage_stats,
            // Security Kernel - Audit（读模型查询位于基础设施层）
            crate::infrastructure::db::commands::audit_events_query,
            crate::infrastructure::db::commands::audit_event_stats,
            crate::infrastructure::db::commands::audit_events_query_filtered,
            crate::infrastructure::db::commands::audit_event_histogram,
            crate::security_kernel::commands::kernel_stats,
            // Security Kernel - Approval
            crate::security_kernel::commands::approval_list_pending,
            crate::security_kernel::commands::approval_stats,
            crate::security_kernel::commands::approval_grant,
            crate::security_kernel::commands::approval_reject,
            crate::security_kernel::commands::approval_cleanup_expired,
            // Security Kernel - PVE
            crate::security_kernel::commands::pve_validate_tool_call,
            crate::security_kernel::commands::pve_get_injection_patterns,
            crate::security_kernel::commands::pve_get_sensitive_surfaces,
            crate::security_kernel::commands::pve_check_sensitive_path,
            crate::security_kernel::commands::pve_scan_content,
            // Security Kernel - Hooks
            crate::security_kernel::hooks::commands::hook_list,
            crate::security_kernel::hooks::commands::hook_register,
            crate::security_kernel::hooks::commands::hook_unregister,
            crate::security_kernel::hooks::commands::hook_test_dispatch,
            // Workspace
            crate::application::workspace::commands::create_workspace,
            crate::application::workspace::commands::list_workspaces,
            crate::application::workspace::commands::list_archived_workspaces,
            crate::application::workspace::commands::get_workspace,
            crate::application::workspace::commands::delete_workspace,
            crate::application::workspace::commands::touch_workspace,
            crate::application::workspace::commands::archive_workspace,
            crate::application::workspace::commands::restore_workspace,
            crate::application::workspace::commands::update_workspace_sort_order,
            // Session
            crate::application::session::commands::session_create,
            crate::application::session::commands::session_split,
            crate::application::session::commands::session_list,
            crate::application::session::commands::session_list_archived,
            crate::application::session::commands::session_get,
            crate::application::session::commands::session_delete,
            crate::application::session::commands::session_archive,
            crate::application::session::commands::session_restore,
            crate::application::session::commands::update_session_sort_order,
            crate::application::session::commands::session_messages,
            crate::application::session::commands::message_create,
            crate::application::session::commands::session_search,
            // User Profile
            crate::domain::user::commands::user_get_profile,
            crate::domain::user::commands::user_update_profile,
            crate::domain::user::learned_commands::learned_preferences_list,
            crate::domain::user::learned_commands::learned_preferences_list_by_key,
            crate::domain::user::learned_commands::learned_preferences_list_high_confidence,
            crate::domain::user::learned_commands::learned_preferences_delete,
            crate::domain::user::learned_commands::learned_preferences_analyze,
            crate::domain::user::learned_commands::learned_preferences_decay_stale,
            // Skill
            crate::application::skill::commands::skill_list,
            crate::application::skill::commands::skill_get,
            crate::application::skill::commands::skill_create,
            crate::application::skill::commands::skill_update,
            crate::application::skill::commands::skill_delete,
            crate::application::skill::commands::skill_index,
            crate::application::skill::commands::skill_refresh,
            crate::application::skill::capability_commands::capability_skill_list,
            crate::application::skill::capability_commands::capability_skill_get,
            crate::application::skill::capability_commands::capability_skill_resolve,
            crate::application::skill::capability_commands::prompt_pack_list,
            crate::application::skill::capability_commands::prompt_pack_get,
            crate::application::skill::capability_commands::prompt_pack_append_guidance,
            crate::application::skill::evolution_commands::skill_usage_list,
            crate::application::skill::evolution_commands::skill_usage_get,
            crate::application::skill::evolution_commands::skill_usage_record,
            crate::application::skill::evolution_commands::skill_usage_feedback,
            crate::application::skill::evolution_commands::skill_candidate_list,
            crate::application::skill::evolution_commands::skill_candidate_approve,
            crate::application::skill::evolution_commands::skill_candidate_reject,
            crate::application::skill::evolution_commands::skill_candidate_delete,
            // Story
            crate::domain::story::commands::story_state_get,
            crate::domain::story::commands::story_state_save,
            crate::domain::story::commands::hook_update_status,
            crate::domain::story::commands::query_facts_at_chapter,
            crate::domain::story::commands::list_recent_chapter_summaries,
            crate::domain::story::commands::list_chapter_summaries_range,
            // Notifications
            crate::infrastructure::notifications::send_notification,
            // Radar
            crate::domain::radar::commands::radar_scan_list,
            crate::domain::radar::commands::radar_scan_delete,
            crate::domain::radar::commands::radar_scan_create,
            crate::domain::radar::commands::radar_scan,
            crate::domain::radar::commands::radar_list_sources,
            // Sandbox
            crate::infrastructure::sandbox::commands::sandbox_status,
            crate::infrastructure::sandbox::commands::sandbox_validate_path,
            crate::infrastructure::sandbox::commands::sandbox_validate_command,
            crate::infrastructure::sandbox::commands::sandbox_validate_url,
            crate::infrastructure::sandbox::commands::sandbox_get_policy,
            crate::infrastructure::sandbox::commands::sandbox_get_exec_policy,
            crate::infrastructure::sandbox::commands::sandbox_update_exec_policy,
            crate::infrastructure::sandbox::commands::sandbox_reset_exec_policy,
            crate::infrastructure::sandbox::commands::sandbox_evaluate_command,
            crate::infrastructure::sandbox::commands::sandbox_evaluate_path,
            crate::infrastructure::sandbox::commands::sandbox_evaluate_network,
            // LLM
            crate::infrastructure::llm::commands::llm_model_list,
            crate::infrastructure::llm::commands::llm_list_provider_presets,
            crate::application::llm::commands::prompt_optimize,
            crate::infrastructure::providers::provider_list,
            crate::infrastructure::providers::provider_models,
            crate::infrastructure::providers::provider_test_connection,
            crate::infrastructure::providers::provider_refresh,
            // Embedding
            crate::infrastructure::llm::embedding::commands::embedding_get_config,
            crate::infrastructure::llm::embedding::commands::embedding_set_config,
            crate::infrastructure::llm::embedding::commands::embedding_test,
            crate::infrastructure::llm::embedding::commands::embedding_index_doc,
            crate::infrastructure::llm::embedding::commands::embedding_search,
            crate::infrastructure::llm::embedding::commands::embedding_delete_doc,
            crate::infrastructure::llm::embedding::commands::embedding_stats,
            crate::infrastructure::llm::embedding::commands::embedding_ingest_file,
            // Notify
            crate::infrastructure::notify::notify_dispatch,
            crate::infrastructure::notify::notify_test,
            // Materials
            crate::domain::materials::commands::material_ingest,
            crate::domain::materials::commands::material_retrieve,
            crate::domain::materials::commands::material_list,
            crate::domain::materials::commands::material_delete,
            // Detection
            crate::domain::detection::commands::detection_scan,
            crate::domain::detection::commands::detection_stats,
            crate::domain::detection::commands::detection_history,
            // Researcher
            crate::domain::researcher::commands::researcher_run,
            // Style
            crate::domain::style::commands::style_analyze,
            // Wiki
            crate::domain::wiki::commands::wiki_list_entries,
            crate::domain::wiki::commands::wiki_get_entry,
            crate::domain::wiki::commands::wiki_create_entry,
            crate::domain::wiki::commands::wiki_update_entry,
            crate::domain::wiki::commands::wiki_delete_entry,
            crate::domain::wiki::commands::wiki_get_graph,
            crate::domain::wiki::commands::wiki_create_link,
            crate::domain::wiki::commands::wiki_delete_link,
            crate::domain::wiki::commands::wiki_search,
            // Version
            crate::domain::version::commands::version_list,
            crate::domain::version::commands::version_get,
            crate::domain::version::commands::version_get_latest,
            crate::domain::version::commands::version_save,
            crate::domain::version::commands::version_diff,
            crate::domain::version::commands::version_diff_latest,
            crate::domain::version::commands::version_restore,
            // Memory
            crate::infrastructure::memory::short_term_commands::short_term_memory_list_by_date,
            crate::infrastructure::memory::short_term_commands::short_term_memory_for_session,
            crate::infrastructure::memory::short_term_commands::short_term_memory_list_by_book,
            crate::infrastructure::memory::short_term_commands::short_term_memory_list_by_range,
            crate::application::session::commands::short_term_memory_regenerate,
            crate::infrastructure::memory::short_term_commands::short_term_memory_stats,
            crate::infrastructure::memory::commands_v2::core_memory_append,
            crate::infrastructure::memory::commands_v2::core_memory_replace,
            crate::infrastructure::memory::commands_v2::core_memory_load,
            crate::infrastructure::memory::commands_v2::recall_memory_insert,
            crate::infrastructure::memory::commands_v2::recall_memory_search,
            crate::infrastructure::memory::commands_v2::archival_memory_insert,
            crate::infrastructure::memory::commands_v2::archival_memory_search,
            crate::infrastructure::memory::commands_v2::archival_memory_get,
            crate::infrastructure::memory::commands_v2::archival_memory_delete,
            // Project Memory
            crate::infrastructure::project_memory::commands::project_memory_get,
            crate::infrastructure::project_memory::commands::project_memory_update,
            crate::infrastructure::project_memory::commands::project_memory_append,
            crate::infrastructure::project_memory::commands::project_memory_clear,
            crate::infrastructure::project_memory::commands::project_memory_delete,
            crate::infrastructure::project_memory::commands::project_memory_stats,
            // Tool Limits
            crate::infrastructure::tool_limits::commands::tool_limits_get,
            crate::infrastructure::tool_limits::commands::tool_limits_update,
            crate::infrastructure::tool_limits::commands::tool_limits_reset,
            // Filesystem
            crate::infrastructure::fs::commands::fs_read_file,
            crate::infrastructure::fs::commands::fs_write_file,
            crate::infrastructure::fs::commands::fs_list_directory,
            crate::infrastructure::fs::commands::fs_create_directory,
            crate::infrastructure::fs::commands::fs_delete_file,
            crate::infrastructure::fs::commands::fs_exists,
            crate::infrastructure::fs::commands::fs_copy_file,
            crate::infrastructure::fs::commands::editor_read_file,
            crate::infrastructure::fs::commands::editor_write_file,
            // Git
            crate::domain::git::commands::git_init,
            crate::domain::git::commands::git_status,
            crate::domain::git::commands::git_log,
            crate::domain::git::commands::git_diff,
            crate::domain::git::commands::git_stage,
            crate::domain::git::commands::git_unstage,
            crate::domain::git::commands::git_commit,
            crate::domain::git::commands::git_rollback,
            crate::domain::git::commands::git_get_config,
            crate::domain::git::commands::git_set_config,
            crate::domain::git::commands::git_branches,
            // Network
            crate::infrastructure::net::lm_ping,
            // Agent Chat
            crate::core::agent::commands::chat_send_message,
            crate::core::agent::commands::chat_stop,
            crate::core::agent::commands::chat_tool_respond,
            // Daily Summary
            crate::core::agent::daily_summary_commands::daily_summary_trigger,
            crate::core::agent::daily_summary_commands::daily_summary_get_config,
            crate::core::agent::daily_summary_commands::daily_summary_update_config,
            crate::core::agent::daily_summary_commands::daily_summary_start,
            crate::core::agent::daily_summary_commands::daily_summary_stop,
            crate::core::agent::daily_summary_commands::daily_summary_is_running,
            // Pipeline
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
            // Pipeline - Phase 6
            crate::domain::pipeline::commands::pipeline_short_fiction_run,
            crate::domain::pipeline::commands::pipeline_fanfic_import,
            crate::domain::pipeline::commands::pipeline_script_run,
            crate::domain::pipeline::commands::pipeline_storyboard_run,
            crate::domain::pipeline::commands::pipeline_interactive_film_run,
            crate::domain::pipeline::commands::pipeline_story_graph_validate,
            crate::domain::pipeline::commands::pipeline_story_graph_paths,
            crate::domain::pipeline::commands::pipeline_story_graph_apply_delta,
            // Interaction Runtime
            crate::domain::interaction::commands::interaction_run_request,
            crate::domain::interaction::commands::interaction_list_sessions,
            crate::domain::interaction::commands::interaction_get_session,
            crate::domain::interaction::commands::interaction_delete_session,
            crate::domain::interaction::commands::interaction_update_automation_mode,
            crate::domain::interaction::commands::interaction_edit_chapter,
            // Secrets
            crate::infrastructure::secrets::secrets_set,
            crate::infrastructure::secrets::secrets_delete,
            crate::infrastructure::secrets::secrets_exists,
            // MCP
            crate::infrastructure::mcp::commands::mcp_list_servers,
            crate::infrastructure::mcp::commands::mcp_add_server,
            crate::infrastructure::mcp::commands::mcp_update_server,
            crate::infrastructure::mcp::commands::mcp_remove_server,
            crate::infrastructure::mcp::commands::mcp_test_server,
            crate::infrastructure::mcp::commands::mcp_list_tools,
            crate::infrastructure::mcp::commands::mcp_call_tool,
            // Loop Engineering
            crate::application::loop_engine::commands::loop_create_state,
            crate::application::loop_engine::commands::loop_get_states,
            crate::application::loop_engine::commands::loop_get_state,
            crate::application::loop_engine::commands::loop_update_state,
            crate::application::loop_engine::commands::loop_delete_state,
            crate::application::loop_engine::commands::loop_pause,
            crate::application::loop_engine::commands::loop_resume,
            crate::application::loop_engine::commands::loop_get_run_logs,
            crate::application::loop_engine::commands::loop_get_patterns,
            crate::application::loop_engine::commands::loop_upsert_pattern,
            crate::application::loop_engine::commands::loop_delete_pattern,
            // Telemetry
            crate::infrastructure::telemetry::commands::telemetry_list_traces,
            crate::infrastructure::telemetry::commands::telemetry_get_trace,
            crate::infrastructure::telemetry::commands::telemetry_list_spans,
            crate::infrastructure::telemetry::commands::telemetry_query_metrics,
            crate::infrastructure::telemetry::commands::telemetry_aggregate_metrics,
            crate::infrastructure::telemetry::commands::telemetry_stats,
            // Agent Registry
            crate::core::agent::registry::commands::agent_list_all,
            crate::core::agent::registry::commands::agent_list_by_category,
            crate::core::agent::registry::commands::agent_get,
            crate::core::agent::registry::commands::agent_list_categories,
            // Play Mode
            crate::domain::play::commands::play_create_world,
            crate::domain::play::commands::play_list_worlds,
            crate::domain::play::commands::play_seed_opening,
            crate::domain::play::commands::play_step,
            crate::domain::play::commands::play_regenerate_last_turn,
            crate::domain::play::commands::play_get_state,
            crate::domain::play::commands::play_get_history,
            // Interactive Film
            crate::domain::pipeline::interactive_film::commands::film_generate_graph,
            crate::domain::pipeline::interactive_film::commands::film_export_html,
            crate::domain::pipeline::interactive_film::commands::film_export_ink,
            crate::domain::pipeline::interactive_film::commands::film_apply_delta,
            // Process Monitor
            crate::infrastructure::process_monitor::commands::process_monitor_list,
            crate::infrastructure::process_monitor::commands::process_monitor_summary,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("error while running tauri application: {e:?}");
            std::process::exit(1);
        });
}