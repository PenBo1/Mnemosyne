use tauri::State;
use std::time::Duration;
use crate::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::domain::novel::types::*;
use crate::domain::novel::client::NovelClient;
use crate::domain::novel::source::load_builtin_sources;
use std::path::PathBuf;

fn novels_dir(state: &AppState) -> PathBuf {
    state.data_dir.data_dir().join("novels")
}

#[tauri::command]
pub async fn novel_source_list(
    _state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<BookSource>>, AppError> {
    let sources = load_builtin_sources();
    Ok(IpcResponse::ok(sources))
}

#[tauri::command]
pub async fn novel_search(
    _state: State<'_, AppState>,
    source_name: String,
    keyword: String,
) -> Result<IpcResponse<Vec<SearchBookResult>>, AppError> {
    if keyword.trim().is_empty() {
        return Err(AppError::invalid_input("Search keyword cannot be empty"));
    }
    if keyword.len() > 100 {
        return Err(AppError::invalid_input("Search keyword too long (max 100 chars)"));
    }

    let sources = load_builtin_sources();
    let source = sources.iter().find(|s| s.name == source_name)
        .ok_or_else(|| AppError::not_found(format!("Source '{}' not found", source_name)))?;

    let client = NovelClient::new()?;
    let results = client.search(source, &keyword).await?;
    Ok(IpcResponse::ok(results))
}

#[tauri::command]
pub async fn novel_download(
    state: State<'_, AppState>,
    source_name: String,
    book_url: String,
    book_name: String,
) -> Result<IpcResponse<String>, AppError> {
    if book_name.trim().is_empty() {
        return Err(AppError::invalid_input("Book name cannot be empty"));
    }

    let sources = load_builtin_sources();
    let source = sources.iter().find(|s| s.name == source_name)
        .ok_or_else(|| AppError::not_found(format!("Source '{}' not found", source_name)))?;

    let client = NovelClient::new()?;

    // Get TOC
    let chapters = client.get_toc(source, &book_url).await?;
    if chapters.is_empty() {
        return Err(AppError::internal("No chapters found"));
    }

    // Create book directory
    let safe_name = book_name.replace(|c: char| !c.is_alphanumeric() && c != ' ' && c != '_', "_");
    let book_dir = novels_dir(&state).join(&safe_name);
    std::fs::create_dir_all(&book_dir)
        .map_err(|_| AppError::file_write_error(book_dir.display().to_string()))?;

    // Download chapters
    let mut content = String::new();
    content.push_str(&format!("# {}\n\n", book_name));

    for (i, chapter) in chapters.iter().enumerate() {
        match client.get_chapter_content(source, &chapter.url).await {
            Ok(chapter_content) => {
                content.push_str(&format!("## {}\n\n", chapter.title));
                content.push_str(&chapter_content.content);
                content.push_str("\n\n");
            }
            Err(e) => {
                tracing::warn!(chapter = %chapter.title, error = %e, "Failed to download chapter");
                content.push_str(&format!("## {}\n\n[下载失败]\n\n", chapter.title));
            }
        }

        // Rate limiting: 200ms between requests
        if i < chapters.len() - 1 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    // Write to file
    let file_path = book_dir.join(format!("{}.md", safe_name));
    std::fs::write(&file_path, &content)
        .map_err(|_| AppError::file_write_error(file_path.display().to_string()))?;

    tracing::info!(book = %book_name, chapters = chapters.len(), "Novel downloaded");
    Ok(IpcResponse::ok(file_path.display().to_string()))
}

#[tauri::command]
pub async fn novel_list_local(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<String>>, AppError> {
    let dir = novels_dir(&state);
    if !dir.exists() {
        return Ok(IpcResponse::ok(Vec::new()));
    }

    let entries = std::fs::read_dir(&dir)
        .map_err(|e| AppError::internal(format!("Failed to read novels dir: {}", e)))?;

    let novels: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();

    Ok(IpcResponse::ok(novels))
}
