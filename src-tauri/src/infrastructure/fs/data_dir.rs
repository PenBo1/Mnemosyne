
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
        std::fs::create_dir_all(self.novels_dir())
            .map_err(|e| AppError::internal(format!("Failed to create novels dir: {}", e)))?;
        std::fs::create_dir_all(self.materials_dir())
            .map_err(|e| AppError::internal(format!("Failed to create materials dir: {}", e)))?;
        std::fs::create_dir_all(self.detection_dir())
            .map_err(|e| AppError::internal(format!("Failed to create detection dir: {}", e)))?;
        std::fs::create_dir_all(self.play_dir())
            .map_err(|e| AppError::internal(format!("Failed to create play dir: {}", e)))?;

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

    /// 下载小说的本地存储目录(单文件 .txt)
    pub fn novels_dir(&self) -> PathBuf {
        self.root.join("novels")
    }

    /// 辅助材料导入目录 —— 存放 ingest 产出的 markdown 正文 + JSON 清单。
    /// 每个 material 一对文件: `<id>.md` 与 `<id>.json`,检索时枚举 .json 还原清单。
    pub fn materials_dir(&self) -> PathBuf {
        self.root.join("materials")
    }

    /// AIGC 检测历史目录 —— 按 book_id 分文件存储检测/改写历史(JSON)。
    pub fn detection_dir(&self) -> PathBuf {
        self.root.join("detection")
    }

    /// Play 模式根目录 —— 存放互动小说世界与 run 数据。
    pub fn play_dir(&self) -> PathBuf {
        self.root.join("play")
    }

    /// 工作区级别的应用数据根目录 —— 存放每个 workspace 的 project_memory.md 等文件。
    ///
    /// 注意:不在 initialize() 中预先创建,改由 ProjectMemoryStore 在首次写入时按需创建,
    /// 避免为已删除的 workspace 留下空目录。
    pub fn workspaces_dir(&self) -> PathBuf {
        self.root.join("workspaces")
    }

    /// 单个 workspace 的数据目录 —— `<root>/workspaces/<workspace_id>/`
    ///
    /// `workspace_id` 由 IPC 层调用方经过 validate_id_component 校验后传入,
    /// 此处不再做路径净化,以保持与 agents_dir() 一致的简洁。
    pub fn workspace_dir(&self, workspace_id: &str) -> PathBuf {
        self.workspaces_dir().join(workspace_id)
    }

    /// workspace 级别的项目记忆文件路径 —— `<root>/workspaces/<workspace_id>/project_memory.md`
    ///
    /// 这里采用 DataDir 集中存储(而非污染 workspace 的外部 path 目录),
    /// 便于 delete_workspace 时确定性清理。
    pub fn workspace_memory_path(&self, workspace_id: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join("project_memory.md")
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
        assert_eq!(data_dir.workspaces_dir(), root.join("workspaces"));
        assert_eq!(
            data_dir.workspace_dir("ws-1"),
            root.join("workspaces").join("ws-1")
        );
        assert_eq!(
            data_dir.workspace_memory_path("ws-1"),
            root.join("workspaces").join("ws-1").join("project_memory.md")
        );
    }
}