use tauri::State;
use std::time::Duration;
use crate::shared::errors::{IpcResponse, AppError};
use crate::AppState;
use crate::features::novel::types::*;
use crate::features::novel::client::NovelClient;
use crate::features::novel::source::{load_builtin_sources_from_dir, load_sources_from_file};
use crate::infrastructure::file_storage::fs_utils::validate_path_within_root;
use std::path::PathBuf;

fn novels_dir(state: &AppState) -> PathBuf {
    state.data_dir.data_dir().join("novels")
}

#[tauri::command]
pub async fn novel_source_list(
    state: State<'_, AppState>,
) -> Result<IpcResponse<Vec<BookSource>>, AppError> {
    let sources_dir = state.data_dir.book_sources_dir();
    let sources = load_builtin_sources_from_dir(&sources_dir);
    Ok(IpcResponse::ok(sources))
}

#[tauri::command]
pub async fn novel_source_toggle(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<IpcResponse<()>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    if name.len() > 255 {
        return Err(AppError::invalid_input("Source name too long (max 255 chars)"));
    }
    let sources_dir = state.data_dir.book_sources_dir();
    let mut all_sources = load_builtin_sources_from_dir(&sources_dir);
    
    // 查找并切换 source 的启用状态
    let found = all_sources.iter_mut().find(|s| s.name == name);
    match found {
        Some(source) => {
            source.disabled = !enabled;
        }
        None => return Err(AppError::not_found(format!("Source '{}' not found", name))),
    }

    // 将所有 source 保存回文件（按原始文件分组）
    // 为简化实现，将所有 source 保存到单个 custom.json 文件
    let custom_path = sources_dir.join("custom.json");
    let content = serde_json::to_string_pretty(&all_sources)
        .map_err(|e| AppError::internal(format!("Failed to serialize sources: {}", e)))?;
    std::fs::write(&custom_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write sources: {}", e)))?;
    
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn novel_source_add(
    state: State<'_, AppState>,
    source: BookSource,
) -> Result<IpcResponse<()>, AppError> {
    if source.name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    if source.name.len() > 255 {
        return Err(AppError::invalid_input("Source name too long (max 255 chars)"));
    }
    let sources_dir = state.data_dir.book_sources_dir();
    let custom_path = sources_dir.join("custom.json");
    
    // 加载现有自定义 source，没有则为空
    let mut custom_sources = if custom_path.exists() {
        load_sources_from_file(&custom_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // 检查名称是否重复
    if custom_sources.iter().any(|s| s.name == source.name) {
        return Err(AppError::conflict(format!("Source '{}' already exists", source.name)));
    }
    
    custom_sources.push(source);
    
    let content = serde_json::to_string_pretty(&custom_sources)
        .map_err(|e| AppError::internal(format!("Failed to serialize sources: {}", e)))?;
    std::fs::write(&custom_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write sources: {}", e)))?;
    
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn novel_source_update(
    state: State<'_, AppState>,
    source: BookSource,
) -> Result<IpcResponse<()>, AppError> {
    if source.name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    let sources_dir = state.data_dir.book_sources_dir();
    let custom_path = sources_dir.join("custom.json");
    
    let mut custom_sources = if custom_path.exists() {
        load_sources_from_file(&custom_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    
    // 查找并更新
    let found = custom_sources.iter_mut().find(|s| s.name == source.name);
    match found {
        Some(s) => *s = source,
        None => return Err(AppError::not_found(format!("Source '{}' not found", source.name))),
    }
    
    let content = serde_json::to_string_pretty(&custom_sources)
        .map_err(|e| AppError::internal(format!("Failed to serialize sources: {}", e)))?;
    std::fs::write(&custom_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write sources: {}", e)))?;
    
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn novel_source_delete(
    state: State<'_, AppState>,
    name: String,
) -> Result<IpcResponse<()>, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    let sources_dir = state.data_dir.book_sources_dir();
    let custom_path = sources_dir.join("custom.json");
    
    let mut custom_sources = if custom_path.exists() {
        load_sources_from_file(&custom_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    
    let before_len = custom_sources.len();
    custom_sources.retain(|s| s.name != name);
    
    if custom_sources.len() == before_len {
        return Err(AppError::not_found(format!("Source '{}' not found", name)));
    }
    
    let content = serde_json::to_string_pretty(&custom_sources)
        .map_err(|e| AppError::internal(format!("Failed to serialize sources: {}", e)))?;
    std::fs::write(&custom_path, content)
        .map_err(|e| AppError::internal(format!("Failed to write sources: {}", e)))?;
    
    Ok(IpcResponse::ok(()))
}

#[tauri::command]
pub async fn novel_search(
    state: State<'_, AppState>,
    source_name: String,
    keyword: String,
) -> Result<IpcResponse<Vec<SearchBookResult>>, AppError> {
    if source_name != "all" && source_name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    if source_name.len() > 255 {
        return Err(AppError::invalid_input("Source name too long (max 255 chars)"));
    }
    if keyword.trim().is_empty() {
        return Err(AppError::invalid_input("Search keyword cannot be empty"));
    }
    if keyword.len() > 100 {
        return Err(AppError::invalid_input("Search keyword too long (max 100 chars)"));
    }

    let sources_dir = state.data_dir.book_sources_dir();
    let sources = load_builtin_sources_from_dir(&sources_dir);
    let client = NovelClient::new()?;

    if source_name == "all" {
        let searchables: Vec<&BookSource> = sources.iter()
            .filter(|s| !s.disabled)
            .filter(|s| s.search.as_ref().map_or(false, |sr| !sr.disabled))
            .collect();

        let mut all_results = Vec::new();
        for source in searchables {
            match client.search(source, &keyword).await {
                Ok(mut results) => all_results.append(&mut results),
                Err(e) => {
                    tracing::warn!(source = %source.name, error = %e, "Search failed on source");
                }
            }
        }
        Ok(IpcResponse::ok(all_results))
    } else {
        let source = sources.iter().find(|s| s.name == source_name)
            .ok_or_else(|| AppError::not_found(format!("Source '{}' not found", source_name)))?;
        let results = client.search(source, &keyword).await?;
        Ok(IpcResponse::ok(results))
    }
}

#[tauri::command]
pub async fn novel_download(
    state: State<'_, AppState>,
    source_name: String,
    book_url: String,
    book_name: String,
) -> Result<IpcResponse<String>, AppError> {
    if source_name.trim().is_empty() {
        return Err(AppError::invalid_input("Source name cannot be empty"));
    }
    if source_name.len() > 255 {
        return Err(AppError::invalid_input("Source name too long (max 255 chars)"));
    }
    if book_url.trim().is_empty() {
        return Err(AppError::invalid_input("Book URL cannot be empty"));
    }
    if book_url.len() > 2048 {
        return Err(AppError::invalid_input("Book URL too long (max 2048 chars)"));
    }
    if book_name.trim().is_empty() {
        return Err(AppError::invalid_input("Book name cannot be empty"));
    }

    let sources_dir = state.data_dir.book_sources_dir();
    let sources = load_builtin_sources_from_dir(&sources_dir);
    let source = sources.iter().find(|s| s.name == source_name)
        .ok_or_else(|| AppError::not_found(format!("Source '{}' not found", source_name)))?;

    let client = NovelClient::new()?;

    // 获取目录
    let chapters = client.get_toc(source, &book_url).await?;
    if chapters.is_empty() {
        return Err(AppError::internal("No chapters found"));
    }

    // 创建书籍目录
    let safe_name = book_name.replace(|c: char| !c.is_alphanumeric() && c != ' ' && c != '_', "_");
    let book_dir = novels_dir(&state).join(&safe_name);
    let novels_root = novels_dir(&state);
    validate_path_within_root(&book_dir, &novels_root, "book_name")?;
    std::fs::create_dir_all(&book_dir)
        .map_err(|_| AppError::file_write_error(book_dir.display().to_string()))?;

    // 下载章节
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

        // 限速：请求间隔 200ms
        if i < chapters.len() - 1 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    // 写入文件
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
