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
/// - agents/                      # Per-agent identity files (SOUL.md, CONTEXT.md, MEMORY.md)
///   - architect/
///   - planner/
///   - composer/
///   - writer/
///   - auditor/
///   - reviser/
///   - observer/
///   - reflector/
#[derive(Clone)]
pub struct DataDir {
    root: PathBuf,
}

/// All agent roles that have identity files.
pub const AGENT_ROLES: &[&str] = &[
    "architect", "planner", "composer", "writer",
    "auditor", "reviser", "observer", "reflector",
    "foundation-reviewer", "length-normalizer", "radar", "detector",
];

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
        self.ensure_agent_identities()?;

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

    pub fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    /// Get the identity directory for a specific agent role.
    pub fn agent_dir(&self, role: &str) -> PathBuf {
        self.agents_dir().join(role)
    }

    // --- Agent identity file getters ---

    /// Get SOUL.md path for an agent role.
    pub fn agent_soul_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("SOUL.md")
    }

    /// Get CONTEXT.md path for an agent role.
    pub fn agent_context_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("CONTEXT.md")
    }

    /// Get MEMORY.md path for an agent role.
    pub fn agent_memory_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("MEMORY.md")
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

    /// Create default agent identity files (SOUL.md, CONTEXT.md, MEMORY.md)
    /// for each agent role. Existing files are never overwritten.
    fn ensure_agent_identities(&self) -> Result<(), AppError> {
        let agents_dir = self.agents_dir();
        std::fs::create_dir_all(&agents_dir)
            .map_err(|e| AppError::internal(format!("Failed to create agents dir: {}", e)))?;

        for role in AGENT_ROLES {
            let role_dir = agents_dir.join(role);
            std::fs::create_dir_all(&role_dir)
                .map_err(|e| AppError::internal(format!("Failed to create agent dir {}: {}", role, e)))?;

            let soul_path = role_dir.join("SOUL.md");
            if !soul_path.exists() {
                let default = crate::domain::agents::identity::default_soul(role);
                std::fs::write(&soul_path, default)
                    .map_err(|e| AppError::internal(format!("Failed to write default SOUL.md for {}: {}", role, e)))?;
            }

            let context_path = role_dir.join("CONTEXT.md");
            if !context_path.exists() {
                let default = crate::domain::agents::identity::default_context(role);
                std::fs::write(&context_path, default)
                    .map_err(|e| AppError::internal(format!("Failed to write default CONTEXT.md for {}: {}", role, e)))?;
            }

            let memory_path = role_dir.join("MEMORY.md");
            if !memory_path.exists() {
                std::fs::write(&memory_path, "# Agent Memory\n\n<!-- Agent accumulates learning notes here across pipeline runs. -->\n")
                    .map_err(|e| AppError::internal(format!("Failed to write default MEMORY.md for {}: {}", role, e)))?;
            }

            tracing::debug!(role = role, "Ensured agent identity files");
        }

        Ok(())
    }
}
