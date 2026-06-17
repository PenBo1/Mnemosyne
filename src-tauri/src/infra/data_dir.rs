use std::path::PathBuf;
use crate::errors::AppError;

/// Centralized application data directory manager.
///
/// Directory structure:
///
/// app_data_dir/
/// - config.json                   # App settings (UI, system, AI models)
/// - data/
///   - state.sqlite                # Core state (novels, chapters, sessions)
///   - feedback.sqlite             # Error events, lessons, gate evaluations
///   - logs.sqlite                 # Structured logs
/// - logs/
///   - mnemosyne.log              # Rolling daily log files
/// - skills/                      # Local skill definitions
/// - book_sources/                # Book source JSON files
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Initialize all directories and default config files.
    pub fn initialize(&self) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.root)
            .map_err(|e| AppError::internal(format!("Failed to create data root: {}", e)))?;
        std::fs::create_dir_all(self.data_dir())
            .map_err(|e| AppError::internal(format!("Failed to create data dir: {}", e)))?;
        std::fs::create_dir_all(self.logs_dir())
            .map_err(|e| AppError::internal(format!("Failed to create logs dir: {}", e)))?;
        std::fs::create_dir_all(self.skills_dir())
            .map_err(|e| AppError::internal(format!("Failed to create skills dir: {}", e)))?;
        std::fs::create_dir_all(self.book_sources_dir())
            .map_err(|e| AppError::internal(format!("Failed to create book sources dir: {}", e)))?;

        self.ensure_config_json()?;
        self.ensure_default_book_sources()?;

        Ok(())
    }

    // --- Directory getters ---

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn book_sources_dir(&self) -> PathBuf {
        self.root.join("book_sources")
    }

    // --- File getters ---

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn state_db_path(&self) -> PathBuf {
        self.data_dir().join("state.sqlite")
    }

    pub fn logs_db_path(&self) -> PathBuf {
        self.data_dir().join("logs.sqlite")
    }

    pub fn feedback_db_path(&self) -> PathBuf {
        self.data_dir().join("feedback.sqlite")
    }

    // --- Default file creation ---

    fn ensure_config_json(&self) -> Result<(), AppError> {
        let path = self.config_path();
        if path.exists() {
            return Ok(());
        }
        let default = serde_json::json!({
            "ui": {
                "theme": "system",
                "locale": "zh-CN",
                "notifications": true
            },
            "system": {
                "log_level": "info"
            },
            "ai": {
                "models": [],
                "active_model_id": null
            }
        });
        let content = serde_json::to_string_pretty(&default)
            .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;
        tracing::info!(path = %path.display(), "Created default config.json");
        Ok(())
    }

    fn ensure_default_book_sources(&self) -> Result<(), AppError> {
        let dir = self.book_sources_dir();
        crate::domain::novel::source::extract_builtin_sources_to_dir(&dir)?;
        Ok(())
    }
}
