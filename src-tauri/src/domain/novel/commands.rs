//! ═══════════════════════════════════════════════════════════════════════════
//! Novel Commands - 小说模块 IPC 命令
//! ═══════════════════════════════════════════════════════════════════════════

use crate::shared::error::{AppError, IpcResponse};
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::validation::validate_id;
use crate::domain::novel::crawler;
use crate::domain::novel::source;
use crate::domain::novel::types::{BookSource, LocalBookItem, SearchBookResult};
use tauri::State;
use tokio::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

const MAX_TITLE_LEN: usize = 500;
const MAX_GENRE_LEN: usize = 100;
const MAX_KEYWORD_LEN: usize = 200;

/// 全局串行化 novel_sources.json 的读改写，防止 TOCTOU 竞态。
fn novel_sources_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn validate_novel_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "novel_id").map_err(AppError::invalid_input)
}

fn validate_workspace_id(id: &str) -> Result<(), AppError> {
    validate_id(id, "workspace_id").map_err(AppError::invalid_input)
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::invalid_input("Title cannot be empty"));
    }
    if title.len() > MAX_TITLE_LEN {
        return Err(AppError::invalid_input(format!(
            "Title too long (max {} chars)", MAX_TITLE_LEN
        )));
    }
    Ok(())
}

fn validate_genre(genre: &str) -> Result<(), AppError> {
    if genre.len() > MAX_GENRE_LEN {
        return Err(AppError::invalid_input(format!(
            "Genre too long (max {} chars)", MAX_GENRE_LEN
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn novel_create(
    state: State<'_, DbState>,
    workspace_id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    let start = Instant::now();
    tracing::info!(workspace_id = %workspace_id, title_len = title.len(), "novel_create: enter");
    
    validate_workspace_id(&workspace_id)?;
    validate_title(&title)?;
    validate_genre(&genre)?;
    tracing::debug!(workspace_id = %workspace_id, "Input validated");

    let req = crate::infrastructure::db::types::CreateNovelRequest {
        workspace_id,
        title,
        genre,
        platform: "local".to_string(),
        language: "zh".to_string(),
        target_chapters: 100,
        chapter_words: 3000,
    };

    let novel = state.db.create_novel(&req)?;
    tracing::info!(
        novel_id = %novel.id,
        duration_ms = start.elapsed().as_millis(),
        "novel_create: exit"
    );
    Ok(IpcResponse::created(novel))
}

#[tauri::command]
pub async fn novel_update(
    state: State<'_, DbState>,
    id: String,
    title: String,
    genre: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    validate_novel_id(&id)?;
    validate_title(&title)?;
    validate_genre(&genre)?;

    let req = crate::infrastructure::db::types::UpdateNovelRequest {
        title: Some(title),
        genre: Some(genre),
        platform: None,
        language: None,
        target_chapters: None,
        chapter_words: None,
    };

    let novel = state.db.update_novel(&id, &req)?;
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn novel_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<crate::infrastructure::db::types::Novel>>, AppError> {
    let novels = state.db.list_novels()?;
    Ok(IpcResponse::ok(novels))
}

#[tauri::command]
pub async fn novel_get(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<crate::infrastructure::db::types::Novel>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!(id = %id, "novel_get: enter");
    
    validate_novel_id(&id)?;
    let novel = state.db.get_novel_by_id(&id)?
        .ok_or_else(|| AppError::not_found("Novel not found"))?;
    
    tracing::info!(
        novel_id = %novel.id,
        duration_ms = start.elapsed().as_millis(),
        "novel_get: exit"
    );
    Ok(IpcResponse::ok(novel))
}

#[tauri::command]
pub async fn novel_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    validate_novel_id(&id)?;
    let deleted = state.db.delete_novel(&id)?;
    Ok(IpcResponse::ok(deleted))
}

#[tauri::command]
pub async fn novel_source_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<BookSource>>, AppError> {
    let sources_path = state.data_dir.root().join("novel_sources.json");

    if !sources_path.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    // 文件 I/O 卸载到阻塞线程池
    let sources = tokio::task::spawn_blocking(move || -> Result<Vec<BookSource>, AppError> {
        let content = std::fs::read_to_string(&sources_path)
            .map_err(|e| AppError::internal(format!("Failed to read novel sources: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse novel sources: {}", e)))
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(sources))
}

#[tauri::command]
pub async fn novel_source_toggle(
    state: State<'_, DbState>,
    name: String,
    enabled: bool,
) -> Result<IpcResponse<bool>, AppError> {
    // 用全局 Mutex 串行化读改写，防止 TOCTOU 竞态
    let _guard = novel_sources_lock().lock().await;

    let sources_path = state.data_dir.root().join("novel_sources.json");

    if !sources_path.exists() {
        return Err(AppError::not_found("Novel sources file not found"));
    }

    // 读改写整体卸载到阻塞线程池，保证原子性
    let name_clone = name.clone();
    tokio::task::spawn_blocking(move || -> Result<(), AppError> {
        let content = std::fs::read_to_string(&sources_path)
            .map_err(|e| AppError::internal(format!("Failed to read novel sources: {}", e)))?;
        let mut sources: Vec<BookSource> = serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse novel sources: {}", e)))?;

        let mut updated = false;
        for source in &mut sources {
            if source.name == name_clone {
                source.enabled = enabled;
                updated = true;
                break;
            }
        }
        if !updated {
            return Err(AppError::not_found(format!("Source '{}' not found", name_clone)));
        }

        let json = serde_json::to_string_pretty(&sources)
            .map_err(|e| AppError::internal(format!("Failed to serialize novel sources: {}", e)))?;
        std::fs::write(&sources_path, json)
            .map_err(|e| AppError::internal(format!("Failed to write novel sources: {}", e)))
    })
    .await
    .map_err(|e| AppError::internal(format!("spawn_blocking join failed: {}", e)))??;

    Ok(IpcResponse::ok(true))
}

/// 加载书源列表的内部辅助。文件不存在时返回空 Vec。
fn load_sources(state: &DbState) -> Result<Vec<BookSource>, AppError> {
    let path = state.data_dir.root().join("novel_sources.json");
    source::load_sources(&path)
}

/// 搜索小说。`source_name = "all"` 时聚合所有未禁用且支持搜索的书源。
#[tauri::command]
pub async fn novel_search(
    state: State<'_, DbState>,
    source_name: String,
    keyword: String,
) -> Result<IpcResponse<Vec<SearchBookResult>>, AppError> {
    let start = Instant::now();
    tracing::info!(source_name = %source_name, keyword_len = keyword.len(), "novel_search: enter");
    
    if source_name.trim().is_empty() {
        tracing::error!("novel_search: source_name is empty");
        return Err(AppError::invalid_input("source_name cannot be empty"));
    }
    let kw = keyword.trim();
    if kw.is_empty() {
        tracing::error!("novel_search: keyword is empty");
        return Err(AppError::invalid_input("keyword cannot be empty"));
    }
    if kw.len() > MAX_KEYWORD_LEN {
        tracing::error!(keyword_len = kw.len(), max = MAX_KEYWORD_LEN, "novel_search: keyword too long");
        return Err(AppError::invalid_input(format!(
            "keyword too long (max {} chars)",
            MAX_KEYWORD_LEN
        )));
    }
    tracing::debug!(source_name = %source_name, keyword = %kw, "Input validated");

    let sources = load_sources(&state)?;
    let results = crawler::search(&sources, &source_name, kw).await?;
    tracing::info!(
        result_count = results.len(),
        duration_ms = start.elapsed().as_millis(),
        "novel_search: exit"
    );
    Ok(IpcResponse::ok(results))
}

/// 下载整本小说。返回最终 TXT 文件的绝对路径。
#[tauri::command]
pub async fn novel_download(
    state: State<'_, DbState>,
    source_name: String,
    book_url: String,
    book_name: String,
) -> Result<IpcResponse<String>, AppError> {
    if source_name.trim().is_empty() {
        return Err(AppError::invalid_input("source_name cannot be empty"));
    }
    if book_url.trim().is_empty() {
        return Err(AppError::invalid_input("book_url cannot be empty"));
    }
    if book_name.trim().is_empty() {
        return Err(AppError::invalid_input("book_name cannot be empty"));
    }

    let sources = load_sources(&state)?;
    let source = source::find_source(&sources, &source_name).ok_or_else(|| {
        AppError::not_found(format!("book source `{}` not found", source_name))
    })?;
    if !source.enabled {
        return Err(AppError::invalid_input(format!(
            "book source `{}` is disabled",
            source_name
        )));
    }

    let novels_dir = state.data_dir.novels_dir();
    let path = crawler::download(source, &book_url, &novels_dir).await?;
    Ok(IpcResponse::ok(path.display().to_string()))
}

/// 列出本地已下载的小说文件。
#[tauri::command]
pub async fn novel_list_local(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<LocalBookItem>>, AppError> {
    let start = std::time::Instant::now();
    tracing::info!("novel_list_local: enter");
    
    let novels_dir = state.data_dir.novels_dir();
    let items = crawler::list_local(&novels_dir)?;
    
    tracing::info!(
        count = items.len(),
        duration_ms = start.elapsed().as_millis(),
        "novel_list_local: exit"
    );
    Ok(IpcResponse::ok(items))
}