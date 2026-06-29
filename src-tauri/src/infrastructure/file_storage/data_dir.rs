use std::path::PathBuf;
use crate::shared::errors::AppError;

/// 集中式应用数据目录管理器。
///
/// 目录结构：
///
/// app_data_dir/
/// - config.json                   # 应用设置（UI、系统、AI 模型）
/// - data/
///   - state.sqlite                # 核心状态（novel、chapter、session）
///   - feedback.sqlite             # error event、lesson、gate 评估
///   - logs.sqlite                 # 结构化日志
/// - logs/
///   - mnemosyne.log              # 按日轮转的日志文件
/// - skills/                      # 本地 skill 定义
/// - book_sources/                # 书源 JSON 文件
/// - agents/                      # 各 agent 的 identity 文件（SOUL.md、CONTEXT.md、MEMORY.md）
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

/// 所有拥有 identity 文件的 agent 角色。
pub const AGENT_ROLES: &[&str] = &[
    "architect", "planner", "composer", "writer",
    "auditor", "reviser", "observer", "reflector",
    "foundation-reviewer", "length-normalizer", "radar", "detector",
];

impl DataDir {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// 初始化所有目录结构和默认 config.json。
    ///
    /// 注意：本方法只负责基础设施级初始化（目录 + config.json）。
    /// 业务级初始化（agent 身份文件、内置 novel sources）由
    /// `core::init::initialize_app_business_state` 编排，避免 infrastructure
    /// 反向依赖 features/ 和 core/agent/。
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

        self.ensure_config_json()?;

        Ok(())
    }

    // --- 目录 getter ---

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

    /// 获取特定 agent 角色的 identity 目录。
    pub fn agent_dir(&self, role: &str) -> PathBuf {
        self.agents_dir().join(role)
    }

    // --- agent identity 文件 getter ---

    /// 获取 agent 角色的 SOUL.md 路径。
    pub fn agent_soul_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("SOUL.md")
    }

    /// 获取 agent 角色的 CONTEXT.md 路径。
    pub fn agent_context_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("CONTEXT.md")
    }

    /// 获取 agent 角色的 MEMORY.md 路径。
    pub fn agent_memory_path(&self, role: &str) -> PathBuf {
        self.agent_dir(role).join("MEMORY.md")
    }

    // --- 文件 getter ---

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

    #[test]
    fn test_agent_identity_paths() {
        let root = PathBuf::from("/tmp/test_app_data");
        let data_dir = DataDir::new(root.clone());

        assert_eq!(data_dir.agent_dir("writer"), root.join("agents").join("writer"));
        assert_eq!(data_dir.agent_soul_path("writer"), root.join("agents").join("writer").join("SOUL.md"));
        assert_eq!(data_dir.agent_context_path("writer"), root.join("agents").join("writer").join("CONTEXT.md"));
        assert_eq!(data_dir.agent_memory_path("writer"), root.join("agents").join("writer").join("MEMORY.md"));
    }

    #[test]
    fn test_agent_roles_list() {
        assert!(AGENT_ROLES.contains(&"architect"));
        assert!(AGENT_ROLES.contains(&"writer"));
        assert!(AGENT_ROLES.contains(&"auditor"));
        assert!(AGENT_ROLES.contains(&"observer"));
    }
}
