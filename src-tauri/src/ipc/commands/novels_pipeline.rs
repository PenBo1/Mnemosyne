use crate::shared::errors::{AppError, IpcResponse};
use crate::core::agent::pipeline::{PipelineConfig, PipelineRunner};
use crate::features::version::{VersionService, RevisionMode};
use crate::infrastructure::db::models::CreateNovelRequest;
use crate::infrastructure::file_storage::fs_utils::validate_id_component;
use crate::AppState;
use tauri::State;

fn build_runner(
    provider_registry: &crate::infrastructure::llm_client::ProviderRegistry,
    workspace_path: std::path::PathBuf,
    memory_store: Option<std::sync::Arc<crate::infrastructure::state_store::memory::MemoryStore>>,
    data_dir: crate::infrastructure::file_storage::data_dir::DataDir,
) -> Result<PipelineRunner, AppError> {
    let provider = provider_registry.default()?;
    let model = provider_registry.default_model().to_string();
    let config = PipelineConfig {
        provider,
        model,
        project_root: workspace_path,
        model_overrides: std::collections::HashMap::new(),
        memory_store,
        data_dir,
        user_profile: None,
    };
    Ok(PipelineRunner::new(config))
}

async fn resolve_workspace_path(
    state: &AppState,
    workspace_id: &str,
) -> Result<std::path::PathBuf, AppError> {
    let ws = state.db.get_workspace(workspace_id).await?
        .ok_or_else(|| AppError::not_found("Workspace not found"))?;
    Ok(std::path::PathBuf::from(ws.path))
}

#[tauri::command]
pub async fn novel_create(
    state: State<'_, AppState>,
    workspace_id: String,
    title: String,
    genre: String,
    brief: Option<String>,
) -> Result<IpcResponse<crate::features::story::BookConfig>, AppError> {
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store, state.data_dir.clone())?;
    drop(registry);

    let config = runner.create_book(&title, &genre, brief.as_deref()).await?;

    {
        state.db.insert_novel(&config.id, &CreateNovelRequest {
            workspace_id: workspace_id.clone(),
            title: title.clone(),
            genre: genre.clone(),
            platform: "local".to_string(),
            language: "zh".to_string(),
            target_chapters: 100,
            chapter_words: 3000,
        }).await?;
    }

    Ok(IpcResponse::created(config))
}

#[tauri::command]
pub async fn novel_write_next(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    target_words: Option<u32>,
) -> Result<IpcResponse<crate::features::story::WriteResult>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store, state.data_dir.clone())?;
    drop(registry);

    let result = runner.write_next_chapter(&book_id, target_words).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_plan(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    context: Option<String>,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store, state.data_dir.clone())?;
    drop(registry);

    let result = runner.plan_chapter(&book_id, context.as_deref()).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_audit(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<crate::features::story::AuditResult>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store, state.data_dir.clone())?;
    drop(registry);

    let result = runner.audit_chapter(&book_id, chapter_number).await?;
    Ok(IpcResponse::ok(result))
}

fn read_chapter_content(
    workspace_path: &std::path::Path,
    book_id: &str,
    chapter_number: u32,
) -> Result<String, AppError> {
    validate_id_component(book_id, "book_id")?;
    let book_dir = workspace_path.join("books").join(book_id);
    let chapters_dir = book_dir.join("chapters");
    let prefix = format!("{:04}_", chapter_number);

    let mut chapter_content = String::new();
    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                chapter_content = std::fs::read_to_string(entry.path())
                    .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
                break;
            }
        }
    }
    Ok(chapter_content)
}

#[tauri::command]
pub async fn novel_revise(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<String>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let content_before = read_chapter_content(&workspace_path, &book_id, chapter_number)?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store, state.data_dir.clone())?;
    drop(registry);

    if !content_before.is_empty() {
        let version_service = VersionService::new(state.db.clone());
        let _ = version_service.save_version(
            &book_id,
            chapter_number,
            &content_before,
            RevisionMode::Manual,
            "Pre-revision snapshot",
        ).await;
    }

    let result = runner.revise_chapter(&book_id, chapter_number, Default::default()).await?;

    let version_service = VersionService::new(state.db.clone());
    let _ = version_service.save_version(
        &book_id,
        chapter_number,
        &result,
        RevisionMode::Auto,
        "AI revision",
    ).await;

    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_observe(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let memory_store = Some(state.memory_store.clone());
    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path, memory_store.clone(), state.data_dir.clone())?;
    drop(registry);

    let book_dir = runner.config.project_root.join("books").join(&book_id);
    let chapters_dir = book_dir.join("chapters");
    let prefix = format!("{:04}_", chapter_number);

    let mut chapter_content = String::new();
    if let Ok(entries) = std::fs::read_dir(&chapters_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                chapter_content = std::fs::read_to_string(entry.path())
                    .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
                break;
            }
        }
    }

    if chapter_content.is_empty() {
        return Err(AppError::not_found(format!("Chapter {} not found", chapter_number)));
    }

    let chapter_title = chapter_content.lines().next()
        .and_then(|line| line.strip_prefix("# ").or_else(|| line.strip_prefix("## ")))
        .unwrap_or("")
        .to_string();

    let observer = crate::core::agent::ObserverAgent::new();
    let ctx = runner.agent_ctx_for("observer", Some(&book_id)).await;
    let language = "zh";

    match observer.observe_chapter(&ctx, chapter_number, &chapter_title, &chapter_content, language, &state.data_dir).await {
        Ok(output) => {
            let facts_json: Vec<serde_json::Value> = output.facts.iter().map(|f| {
                serde_json::json!({
                    "subject": f.subject,
                    "predicate": f.predicate,
                    "object": f.object,
                    "category": f.category,
                })
            }).collect();

            let hooks_new_json: Vec<serde_json::Value> = output.hooks_new.iter().map(|h| {
                serde_json::json!({
                    "name": h.name,
                    "type": h.hook_type,
                    "status": h.status,
                    "description": h.description,
                })
            }).collect();

            let hooks_advanced_json: Vec<serde_json::Value> = output.hooks_advanced.iter().map(|h| {
                serde_json::json!({
                    "name": h.name,
                    "type": h.hook_type,
                    "status": h.status,
                    "description": h.description,
                })
            }).collect();

            let summary_json = output.chapter_summary.as_ref().map(|s| {
                serde_json::json!({
                    "chapter": s.chapter,
                    "title": s.title,
                    "characters": s.characters,
                    "events": s.events,
                    "state_changes": s.state_changes,
                    "hook_activity": s.hook_activity,
                    "mood": s.mood,
                    "chapter_type": s.chapter_type,
                })
            }).unwrap_or_else(|| serde_json::json!({
                "chapter": chapter_number,
                "title": chapter_title,
                "characters": [],
                "events": [],
                "state_changes": [],
                "hook_activity": [],
                "mood": "",
                "chapter_type": ""
            }));

            let observation = serde_json::json!({
                "chapter": chapter_number,
                "facts": facts_json,
                "hooks_new": hooks_new_json,
                "hooks_advanced": hooks_advanced_json,
                "chapter_summary": summary_json,
            });

            if let Some(ref mem_store) = memory_store {
                for fact in &output.facts {
                    mem_store.archive_fact(
                        &book_id, chapter_number,
                        &fact.subject, &fact.predicate, &fact.object, &fact.category,
                    ).await;
                }
                for hook in output.hooks_new.iter().chain(output.hooks_advanced.iter()) {
                    mem_store.archive_hook(
                        &book_id, chapter_number,
                        &hook.name, &hook.hook_type, &hook.status, &hook.description,
                    ).await;
                }
                if let Some(ref summary) = output.chapter_summary {
                    mem_store.archive_summary(
                        &book_id, chapter_number,
                        &summary.title, &summary.characters, &summary.events,
                    ).await;
                }
            }

            Ok(IpcResponse::ok(observation))
        }
        Err(e) => {
            tracing::warn!(error = %e, "ObserverAgent failed, returning basic observation");
            let observation = serde_json::json!({
                "chapter": chapter_number,
                "facts": [],
                "hooks_new": [],
                "hooks_advanced": [],
                "chapter_summary": {
                    "chapter": chapter_number,
                    "title": chapter_title,
                    "characters": [],
                    "events": [],
                    "state_changes": [],
                    "hook_activity": [],
                    "mood": "",
                    "chapter_type": ""
                }
            });
            Ok(IpcResponse::ok(observation))
        }
    }
}

#[tauri::command]
pub async fn novel_reflect(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    validate_id_component(&book_id, "book_id")?;
    let workspace_path = resolve_workspace_path(&state, &workspace_id).await?;

    let book_dir = workspace_path.join("books").join(&book_id);
    let state_path = book_dir.join("story").join("state.json");

    let mut story_state: crate::features::story::StoryState = if state_path.exists() {
        serde_json::from_str(
            &std::fs::read_to_string(&state_path).unwrap_or_default()
        ).unwrap_or_default()
    } else {
        crate::features::story::StoryState::default()
    };

    story_state.current_chapter = chapter_number;

    let observation_path = book_dir.join("observations").join(format!("{:04}.json", chapter_number));
    let mut facts_count = 0u32;
    let mut hooks_new_count = 0u32;
    let mut hooks_advanced_count = 0u32;

    if observation_path.exists() {
        if let Ok(obs_data) = std::fs::read_to_string(&observation_path) {
            if let Ok(obs) = serde_json::from_str::<serde_json::Value>(&obs_data) {
                facts_count = obs.get("facts").and_then(|v| v.as_array()).map(|a| a.len() as u32).unwrap_or(0);
                hooks_new_count = obs.get("hooks_new").and_then(|v| v.as_array()).map(|a| a.len() as u32).unwrap_or(0);
                hooks_advanced_count = obs.get("hooks_advanced").and_then(|v| v.as_array()).map(|a| a.len() as u32).unwrap_or(0);
            }
        }
    }

    let state_json = serde_json::to_string_pretty(&story_state)
        .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;
    std::fs::write(&state_path, &state_json)
        .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))?;

    let snapshots_dir = book_dir.join("story").join("snapshots");
    let _ = std::fs::create_dir_all(&snapshots_dir);
    let snapshot_path = snapshots_dir.join(format!("{:04}.json", chapter_number));
    let _ = std::fs::write(&snapshot_path, &state_json);

    let result = serde_json::json!({
        "status": "ok",
        "chapter": chapter_number,
        "facts_extracted": facts_count,
        "hooks_new": hooks_new_count,
        "hooks_advanced": hooks_advanced_count,
        "state_updated": true,
    });

    Ok(IpcResponse::ok(result))
}
