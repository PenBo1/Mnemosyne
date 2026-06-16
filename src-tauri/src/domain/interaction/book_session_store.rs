use crate::errors::AppError;

/// Book session store for managing book-specific sessions
pub struct BookSessionStore;

impl BookSessionStore {
    /// Create a new book session
    pub fn create_book_session(
        project_root: &str,
        session_id: &str,
        book_id: &str,
    ) -> Result<(), AppError> {
        let dir = format!("{}/sessions/{}", project_root, session_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create session dir: {}", e)))?;

        let meta = serde_json::json!({
            "session_id": session_id,
            "book_id": book_id,
            "kind": "book",
            "created_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        });

        let path = format!("{}/meta.json", dir);
        std::fs::write(&path, serde_json::to_string_pretty(&meta).unwrap())
            .map_err(|e| AppError::internal(format!("Failed to write meta: {}", e)))?;
        Ok(())
    }

    /// Load a book session
    pub fn load_book_session(
        project_root: &str,
        session_id: &str,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let path = format!("{}/sessions/{}/meta.json", project_root, session_id);
        if !std::path::Path::new(&path).exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read: {}", e)))?;
        let value = serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse: {}", e)))?;
        Ok(Some(value))
    }

    /// List all book sessions
    pub fn list_book_sessions(project_root: &str) -> Result<Vec<String>, AppError> {
        let sessions_dir = format!("{}/sessions", project_root);
        if !std::path::Path::new(&sessions_dir).exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(&sessions_dir)
            .map_err(|e| AppError::internal(format!("Failed to read sessions dir: {}", e)))?;

        let sessions: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        Ok(sessions)
    }
}
