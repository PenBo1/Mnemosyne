// 全局 clippy 抑制配置
// - IPC 命令和数据库操作常有 8-12 个参数（Tauri 命令模式）
// - rusqlite 映射函数返回复杂元组类型
// - 预留的 from_str 方法命名
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::should_implement_trait)]

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
use crate::infrastructure::project_memory::state::ProjectMemoryState;
use crate::infrastructure::tool_limits::state::ToolLimitsState;
use crate::domain::feedback::state::FeedbackState;
use crate::infrastructure::workspace::state::WorkspaceState;
use crate::infrastructure::workspace::registry::WorkspaceRegistry;
use crate::infrastructure::mcp::state::McpState;
use crate::security_kernel::SecurityKernelState;
use crate::security_kernel::hooks::HookEngineState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
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

            // 先创建 DbState,以便克隆 Database 给 AgentEngine 与 SecurityKernel 持有
            let db_state = DbState::new(data_dir.clone())?;
            // DataDir 作为 State 共享给 pipeline 命令
            app.manage(data_dir.clone());
            let db_for_agent = db_state.db.clone();
            let db_for_kernel = db_state.db.clone();
            let db_for_memory = db_state.db.clone();
            let db_for_summary = db_state.db.clone();
            let db_for_registry = db_state.db.clone();
            // 初始化 builtin loop patterns(仅当不存在时写入)
            // 注：seed 失败属非致命——builtin patterns 缺失只影响 loop-engineering 默认模板，
            // 应用仍可正常启动。用 error 级别记录，确保问题不被静默吞掉。
            if let Err(e) = crate::application::init::seed_builtin_loop_patterns(&db_state.db) {
                tracing::error!(error = %e, "Failed to seed builtin loop patterns (non-fatal, continuing startup)");
            }
            app.manage(db_state);
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
            // 创建 WorkspaceRegistry 并预授权：
            // 1. 应用数据目录（agent 身份文件、config 等始终可读）
            // 2. 所有已存在的 workspace 路径（重启后恢复授权）
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
            app.manage(SecurityKernelState::with_db(db_for_kernel));
            // 注册 Tauri 事件桥 —— 把 SecurityEvent 实时推送到前端 listen("security://event")
            crate::security_kernel::audit::register_tauri_emit_handler(app.handle());

            // 从 SecurityKernel 提取共享 HookEngine：
            // 1. 注入为独立 Tauri State（供 hook_* IPC 命令操作）
            // 2. 注入 AgentEngine（供 SessionStart/UserPromptSubmit/Stop 等生命周期 hook 派发）
            let hook_engine_arc = {
                let kernel_state = app.state::<SecurityKernelState>();
                kernel_state.kernel().hook_engine().clone()
            };
            app.manage(HookEngineState::from_arc(hook_engine_arc.clone()));

            // Initialize agent engine from LLM provider registry
            let agent_engine = {
                let registry = crate::infrastructure::llm::registry::ProviderRegistry::new(&data_dir);
                // current_dir 失败时回退到 data_dir.root()，并显式 log warning
                // 避免静默回退掩盖环境异常（如 cwd 被删除）
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
                // I1.5/I1.6：注入真实的 tool ops 实现（application/bridges 桥接 domain），
                // 使 core/agent 工具不直接依赖 domain（架构约束）。
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

            // Pipeline SchedulerState（需要 AgentEngine + DataDir.books_dir + RadarScanOps）
            let scheduler_config = crate::domain::pipeline::scheduler::SchedulerConfig::default();
            let pipeline_config = crate::domain::pipeline::runner::PipelineConfig {
                books_dir: data_dir.books_dir(),
                ..Default::default()
            };
            // I2.2：注入 RadarScanOps 真实实现，消除 pipeline → radar 横向依赖
            let radar_ops: std::sync::Arc<dyn crate::domain::pipeline::radar_ops::RadarScanOps> =
                std::sync::Arc::new(crate::application::bridges::RadarScanOpsImpl::new());
            let scheduler_state = crate::domain::pipeline::scheduler::SchedulerState::new(
                pipeline_config,
                scheduler_config,
                agent_engine.clone(),
                radar_ops,
            );
            app.manage(scheduler_state);

            // I2.1：注入 InteractionPipelineOps 真实实现（application/bridges 桥接 domain::pipeline），
            // 使 domain/interaction/runtime 不直接依赖 domain::pipeline（架构约束）。
            app.manage(crate::domain::interaction::pipeline_ops::InteractionPipelineOpsState {
                ops: std::sync::Arc::new(
                    crate::application::bridges::InteractionPipelineOpsImpl::new(data_dir.clone()),
                ),
            });

            // 每日摘要任务 State(默认不启动,需用户在设置页开启)
            app.manage(crate::core::agent::daily_summary::DailySummaryState::new(
                agent_engine.clone(),
                db_for_summary,
                data_dir.clone(),
            ));

            // 用户画像提供者：application/ 层读取 domain::user 并转换为 core/agent 快照，
            // 注入 AgentState，使 core/agent 不直接依赖 domain::user（架构约束）。
            let user_profile_provider =
                crate::application::init::build_user_profile_provider(data_dir.clone());
            app.manage(crate::core::agent::commands::AgentState::new(
                agent_engine,
                Some(user_profile_provider),
            ));

            // Agent Registry —— 统一 Agent 元数据注册表（main + 15 pipeline + 3 subagent + 3 loopskill）
            app.manage(crate::core::agent::registry::AgentRegistryState::new());

            // SecurityKernel 定期清理（每 5 分钟清理过期的 policy/rate_limiter/audit/approval 条目）
            // 防止 RateStore / ApprovalStore 等无限增长导致内存泄漏
            //
            // 注：setup 闭包在主线程同步执行，不在 Tokio runtime context 内，
            // 直接 tokio::spawn 会 panic（"no reactor running"）。
            // 必须用 tauri::async_runtime::spawn —— 它内部走 Tauri 管理的 runtime，
            // 可在任意线程调用。
            //
            // 此任务为 fire-and-forget，无 shutdown signal。Tauri 桌面应用退出时
            // runtime 会被 Builder::Drop 终止，所有 spawn 的任务随之丢弃。
            // cleanup 是幂等的纯内存操作，被强行中断不会留下不一致状态。
            {
                let kernel_state = app.state::<SecurityKernelState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    interval.tick().await; // 跳过首次立即触发
                    loop {
                        interval.tick().await;
                        kernel_state.cleanup();
                    }
                });
            }

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
            crate::domain::novel::commands::novel_search,
            crate::domain::novel::commands::novel_download,
            crate::domain::novel::commands::novel_list_local,
            crate::infrastructure::stats::get_stats,
            crate::infrastructure::stats::get_daily_activity,
            crate::infrastructure::stats::get_ai_stats,
            crate::infrastructure::stats::get_usage_stats,
            crate::security_kernel::commands::audit_events_query,
            crate::security_kernel::commands::audit_event_stats,
            crate::security_kernel::commands::audit_events_query_filtered,
            crate::security_kernel::commands::audit_event_histogram,
            crate::security_kernel::commands::kernel_stats,
            crate::security_kernel::commands::approval_list_pending,
            crate::security_kernel::commands::approval_stats,
            crate::security_kernel::commands::approval_grant,
            crate::security_kernel::commands::approval_reject,
            crate::security_kernel::commands::approval_cleanup_expired,
            // PVE 命令（Prompt Validator Executor）
            crate::security_kernel::commands::pve_validate_tool_call,
            crate::security_kernel::commands::pve_get_injection_patterns,
            crate::security_kernel::commands::pve_get_sensitive_surfaces,
            crate::security_kernel::commands::pve_check_sensitive_path,
            crate::security_kernel::commands::pve_scan_content,
            // Hook 系统命令（配置型 hook 管理 + 测试派发）
            crate::security_kernel::hooks::commands::hook_list,
            crate::security_kernel::hooks::commands::hook_register,
            crate::security_kernel::hooks::commands::hook_unregister,
            crate::security_kernel::hooks::commands::hook_test_dispatch,
            crate::application::workspace::commands::create_workspace,
            crate::application::workspace::commands::list_workspaces,
            crate::application::workspace::commands::get_workspace,
            crate::application::workspace::commands::delete_workspace,
            crate::application::workspace::commands::touch_workspace,
            crate::application::session::commands::session_create,
            crate::application::session::commands::session_split,
            crate::application::session::commands::session_list,
            crate::application::session::commands::session_get,
            crate::application::session::commands::session_delete,
            crate::application::session::commands::session_messages,
            crate::application::session::commands::message_create,
            crate::application::session::commands::session_search,
            crate::domain::user::commands::user_get_profile,
            crate::domain::user::commands::user_update_profile,
            crate::domain::user::learned_commands::learned_preferences_list,
            crate::domain::user::learned_commands::learned_preferences_list_by_key,
            crate::domain::user::learned_commands::learned_preferences_list_high_confidence,
            crate::domain::user::learned_commands::learned_preferences_delete,
            crate::domain::user::learned_commands::learned_preferences_analyze,
            crate::domain::user::learned_commands::learned_preferences_decay_stale,
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
            crate::domain::radar::commands::radar_list_sources,
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
            crate::infrastructure::llm::commands::llm_model_list,
            crate::infrastructure::llm::commands::llm_list_provider_presets,
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
            crate::infrastructure::llm::embedding::commands::embedding_ingest_file,
            crate::infrastructure::notify::notify_dispatch,
            crate::infrastructure::notify::notify_test,
            // 辅助系统模块命令
            crate::domain::materials::commands::material_ingest,
            crate::domain::materials::commands::material_retrieve,
            crate::domain::materials::commands::material_list,
            crate::domain::materials::commands::material_delete,
            crate::domain::detection::commands::detection_scan,
            crate::domain::detection::commands::detection_stats,
            crate::domain::detection::commands::detection_history,
            crate::domain::researcher::commands::researcher_run,
            crate::domain::style::commands::style_analyze,
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
            crate::infrastructure::project_memory::commands::project_memory_get,
            crate::infrastructure::project_memory::commands::project_memory_update,
            crate::infrastructure::project_memory::commands::project_memory_append,
            crate::infrastructure::project_memory::commands::project_memory_clear,
            crate::infrastructure::project_memory::commands::project_memory_delete,
            crate::infrastructure::project_memory::commands::project_memory_stats,
            crate::infrastructure::tool_limits::commands::tool_limits_get,
            crate::infrastructure::tool_limits::commands::tool_limits_update,
            crate::infrastructure::tool_limits::commands::tool_limits_reset,
            crate::infrastructure::fs::commands::fs_read_file,
            crate::infrastructure::fs::commands::fs_write_file,
            crate::infrastructure::fs::commands::fs_list_directory,
            crate::infrastructure::fs::commands::fs_create_directory,
            crate::infrastructure::fs::commands::fs_delete_file,
            crate::infrastructure::fs::commands::fs_exists,
            crate::infrastructure::fs::commands::fs_copy_file,
            crate::infrastructure::fs::commands::editor_read_file,
            crate::infrastructure::fs::commands::editor_write_file,
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
            crate::infrastructure::net::lm_ping,
            crate::core::agent::commands::chat_send_message,
            crate::core::agent::commands::chat_stop,
            crate::core::agent::commands::chat_tool_respond,
            crate::core::agent::daily_summary_commands::daily_summary_trigger,
            crate::core::agent::daily_summary_commands::daily_summary_get_config,
            crate::core::agent::daily_summary_commands::daily_summary_update_config,
            crate::core::agent::daily_summary_commands::daily_summary_start,
            crate::core::agent::daily_summary_commands::daily_summary_stop,
            crate::core::agent::daily_summary_commands::daily_summary_is_running,
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
            // Interaction Runtime 命令（domain/interaction）
            crate::domain::interaction::commands::interaction_run_request,
            crate::domain::interaction::commands::interaction_list_sessions,
            crate::domain::interaction::commands::interaction_get_session,
            crate::domain::interaction::commands::interaction_delete_session,
            crate::domain::interaction::commands::interaction_update_automation_mode,
            crate::domain::interaction::commands::interaction_edit_chapter,
            crate::infrastructure::secrets::secrets_set,
            crate::infrastructure::secrets::secrets_delete,
            crate::infrastructure::secrets::secrets_exists,
            crate::infrastructure::mcp::commands::mcp_list_servers,
            crate::infrastructure::mcp::commands::mcp_add_server,
            crate::infrastructure::mcp::commands::mcp_update_server,
            crate::infrastructure::mcp::commands::mcp_remove_server,
            crate::infrastructure::mcp::commands::mcp_test_server,
            crate::infrastructure::mcp::commands::mcp_list_tools,
            crate::infrastructure::mcp::commands::mcp_call_tool,
            // Loop-Engineering 命令(application/loop_engine)
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
            // Telemetry 命令（traces + metrics）
            crate::infrastructure::telemetry::commands::telemetry_list_traces,
            crate::infrastructure::telemetry::commands::telemetry_get_trace,
            crate::infrastructure::telemetry::commands::telemetry_list_spans,
            crate::infrastructure::telemetry::commands::telemetry_query_metrics,
            crate::infrastructure::telemetry::commands::telemetry_aggregate_metrics,
            crate::infrastructure::telemetry::commands::telemetry_stats,
            // Agent Registry 命令（统一 agent 元数据查询）
            crate::core::agent::registry::commands::agent_list_all,
            crate::core::agent::registry::commands::agent_list_by_category,
            crate::core::agent::registry::commands::agent_get,
            crate::core::agent::registry::commands::agent_list_categories,
            // Play 模式命令（互动小说引擎）
            crate::domain::play::commands::play_create_world,
            crate::domain::play::commands::play_list_worlds,
            crate::domain::play::commands::play_seed_opening,
            crate::domain::play::commands::play_step,
            crate::domain::play::commands::play_regenerate_last_turn,
            crate::domain::play::commands::play_get_state,
            crate::domain::play::commands::play_get_history,
            // 互动电影 film_* 命令
            crate::domain::pipeline::interactive_film::commands::film_generate_graph,
            crate::domain::pipeline::interactive_film::commands::film_export_html,
            crate::domain::pipeline::interactive_film::commands::film_export_ink,
            crate::domain::pipeline::interactive_film::commands::film_apply_delta,
            // Process Monitor 命令
            crate::infrastructure::process_monitor::commands::process_monitor_list,
            crate::infrastructure::process_monitor::commands::process_monitor_summary,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            // 不用 `.expect(...)`：避免 panic 输出被 Tauri 内部 panic hook 吞掉。
            // 改为 stderr + 退出码 1，确保错误对用户/CI 可见。
            // 注：tracing 日志 guard 由 `init_logging` 注册的 `tracing_subscriber`
            // 在 Drop 时 flush；此处 process::exit 前需要显式 flush。
            eprintln!("error while running tauri application: {e:?}");
            // 强制 flush tracing 日志（tracing-appender 的 NonBlocking guard 在
            // process::exit 时不会执行 Drop，需手动 flush）
            // 注：若未启用 NonBlocking writer，此调用是 no-op
            // 这里不直接访问 guard，依赖 tracing-subscriber 的全局 flush
            std::process::exit(1);
        });
}