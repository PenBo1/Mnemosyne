
use std::path::PathBuf;
use crate::shared::error::AppError;

#[derive(Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

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
        std::fs::create_dir_all(self.agents_dir())
            .map_err(|e| AppError::internal(format!("Failed to create agents dir: {}", e)))?;
        std::fs::create_dir_all(self.books_dir())
            .map_err(|e| AppError::internal(format!("Failed to create books dir: {}", e)))?;

        self.ensure_config_json()?;

        Ok(())
    }

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

    pub fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    /// 创作 pipeline 的书籍工作区目录
    pub fn books_dir(&self) -> PathBuf {
        self.root.join("books")
    }

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

    fn ensure_config_json(&self) -> Result<(), AppError> {
        let path = self.config_path();
        if path.exists() {
            return Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_paths() {
        let root = PathBuf::from("/tmp/test_app_data");
        let data_dir = DataDir::new(root.clone());

        assert_eq!(data_dir.root(), &root);
        assert_eq!(data_dir.data_dir(), root.join("data"));
        assert_eq!(data_dir.logs_dir(), root.join("logs"));
        assert_eq!(data_dir.skills_dir(), root.join("skills"));
        assert_eq!(data_dir.book_sources_dir(), root.join("book_sources"));
        assert_eq!(data_dir.agents_dir(), root.join("agents"));
        assert_eq!(data_dir.config_path(), root.join("config.json"));
        assert_eq!(data_dir.state_db_path(), root.join("data").join("state.sqlite"));
        assert_eq!(data_dir.feedback_db_path(), root.join("data").join("feedback.sqlite"));
    }
}