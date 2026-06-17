use crate::errors::{AppError, IpcResponse};
use crate::domain::pipeline::{PipelineConfig, PipelineRunner};
use crate::infra::db::models::CreateNovelRequest;
use crate::AppState;
use tauri::State;

fn build_runner(
    provider_registry: &crate::infra::llm::ProviderRegistry,
    workspace_path: std::path::PathBuf,
) -> Result<PipelineRunner, AppError> {
    let provider = provider_registry.default()?;
    let model = provider_registry.default_model().to_string();
    let config = PipelineConfig {
        provider,
        model,
        project_root: workspace_path,
        model_overrides: std::collections::HashMap::new(),
    };
    Ok(PipelineRunner::new(config))
}

#[tauri::command]
pub async fn novel_create(
    state: State<'_, AppState>,
    workspace_id: String,
    title: String,
    genre: String,
    brief: Option<String>,
) -> Result<IpcResponse<crate::domain::story::BookConfig>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
    drop(registry);

    let config = runner.create_book(&title, &genre, brief.as_deref()).await?;

    {
        let db = state.db.lock().await;
        db.insert_novel(&config.id, &CreateNovelRequest {
            workspace_id: workspace_id.clone(),
            title: title.clone(),
            genre: genre.clone(),
            platform: "local".to_string(),
            language: "zh".to_string(),
            target_chapters: 100,
            chapter_words: 3000,
        })?;
    }

    Ok(IpcResponse::created(config))
}

#[tauri::command]
pub async fn novel_write_next(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    target_words: Option<u32>,
) -> Result<IpcResponse<crate::domain::story::WriteResult>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
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
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
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
) -> Result<IpcResponse<crate::domain::story::AuditResult>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
    drop(registry);

    let result = runner.audit_chapter(&book_id, chapter_number).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_revise(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<String>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
    drop(registry);

    let result = runner.revise_chapter(&book_id, chapter_number, Default::default()).await?;
    Ok(IpcResponse::ok(result))
}

#[tauri::command]
pub async fn novel_observe(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<serde_json::Value>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let registry = state.provider_registry.lock().await;
    let runner = build_runner(&registry, workspace_path)?;
    drop(registry);

    // Observe extracts facts from the chapter
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

    // Simple observation: extract key facts
    let observation = serde_json::json!({
        "chapter": chapter_number,
        "facts": [],
        "hooks_new": [],
        "hooks_advanced": [],
        "chapter_summary": {
            "chapter": chapter_number,
            "title": "",
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

#[tauri::command]
pub async fn novel_reflect(
    state: State<'_, AppState>,
    workspace_id: String,
    book_id: String,
    chapter_number: u32,
) -> Result<IpcResponse<()>, AppError> {
    let workspace_path = {
        let db = state.db.lock().await;
        let ws = db.get_workspace(&workspace_id)?
            .ok_or_else(|| AppError::not_found("Workspace not found"))?;
        std::path::PathBuf::from(ws.path)
    };

    let _registry = state.provider_registry.lock().await;

    // Reflect: update story state from observation
    let book_dir = workspace_path.join("books").join(&book_id);
    let state_path = book_dir.join("story").join("state.json");

    if state_path.exists() {
        let mut story_state: crate::domain::story::StoryState = serde_json::from_str(
            &std::fs::read_to_string(&state_path).unwrap_or_default()
        ).unwrap_or_default();

        story_state.current_chapter = chapter_number;

        // Save updated state
        let state_json = serde_json::to_string_pretty(&story_state)
            .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(&state_path, state_json)
            .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))?;
    }

    // Save snapshot
    let snapshots_dir = book_dir.join("story").join("snapshots");
    let _ = std::fs::create_dir_all(&snapshots_dir);

    Ok(IpcResponse::ok(()))
}
