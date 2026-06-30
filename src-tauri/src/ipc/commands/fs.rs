use crate::shared::errors::{IpcResponse, AppError};
use std::path::PathBuf;

/// Information about a single file or directory entry.
#[derive(serde::Serialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub size: u64,
}

#[tauri::command]
pub async fn fs_read_file(
    path: String,
) -> Result<IpcResponse<String>, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > 4096 {
        return Err(AppError::invalid_input("Path too long (max 4096 chars)"));
    }

    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(AppError::not_found(&format!("File not found: {}", path)));
    }
    if path_buf.is_dir() {
        return Err(AppError::invalid_input("Path is a directory, not a file"));
    }

    match tokio::fs::read_to_string(&path_buf).await {
        Ok(content) => Ok(IpcResponse::ok(content)),
        Err(e) => Err(AppError::internal(&format!("Failed to read file: {}", e))),
    }
}

#[tauri::command]
pub async fn fs_list_directory(
    path: String,
) -> Result<IpcResponse<Vec<FileEntry>>, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("Path cannot be empty"));
    }
    if path.len() > 4096 {
        return Err(AppError::invalid_input("Path too long (max 4096 chars)"));
    }

    let dir = PathBuf::from(&path);
    if !dir.exists() {
        return Err(AppError::not_found(&format!("Directory not found: {}", path)));
    }
    if !dir.is_dir() {
        return Err(AppError::invalid_input("Path is not a directory"));
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut read_dir = match tokio::fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(e) => return Err(AppError::internal(&format!("Failed to read directory: {}", e))),
    };

    // Common ignore patterns
    let ignore_dirs = [
        "node_modules", ".git", "target", "dist", ".next", "__pycache__",
        ".venv", "venv", ".trae-cn", ". uploads",
    ];

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = match entry.file_name().to_str() {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip hidden files and ignored directories
        if name.starts_with('.') {
            // Allow dotfiles but skip dot-directories that are in ignore list
            if entry.path().is_dir() && ignore_dirs.contains(&name.as_str()) {
                continue;
            }
            // Skip other dot-directories (but allow dotfiles like .env)
            if entry.path().is_dir() && name.starts_with('.') {
                continue;
            }
        }
        if entry.path().is_dir() && ignore_dirs.contains(&name.as_str()) {
            continue;
        }

        let is_dir = entry.path().is_dir();
        let extension = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_string());
        let size = if !is_dir {
            entry.metadata().await.map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        entries.push(FileEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir,
            extension,
            size,
        });
    }

    // Sort: directories first, then files, both alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(IpcResponse::ok(entries))
}
