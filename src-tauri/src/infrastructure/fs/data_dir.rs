//! ═══════════════════════════════════════════════════════════════════════════
//! 数据目录 - 应用数据路径管理
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 集中管理所有应用数据的路径：
//! - data/: SQLite 数据库
//! - logs/: 日志文件
//! - skills/: 技能定义
//! - book_sources/: 书籍来源
//! - agents/: Agent 配置
//! - books/: 书籍工作区
//! - novels/: 下载的小说
//! - materials/: 辅助材料
//! - detection/: AIGC 检测历史
//! - play/: 互动小说
//! - memories/: 记忆系统
//! - workspaces/: 工作区数据

use std::path::PathBuf;
use crate::shared::error::AppError;

/// 数据目录
#[derive(Clone)]
pub struct DataDir {
    /// 根目录
    root: PathBuf,
}

impl DataDir {
    /// 创建数据目录实例
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// 初始化所有子目录
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
        std::fs::create_dir_all(self.memory_root())
            .map_err(|e| AppError::internal(format!("Failed to create memory root: {}", e)))?;

        self.ensure_config_json()?;

        Ok(())
    }

    // ── 目录访问器 ────────────────────────────────────────────────────────────

    /// 根目录
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// 数据目录
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// 日志目录
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// 技能目录
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// 书籍来源目录
    pub fn book_sources_dir(&self) -> PathBuf {
        self.root.join("book_sources")
    }

    /// Agent 配置目录
    pub fn agents_dir(&self) -> PathBuf {
        self.root.join("agents")
    }

    /// 书籍工作区目录
    pub fn books_dir(&self) -> PathBuf {
        self.root.join("books")
    }

    /// 小说存储目录
    pub fn novels_dir(&self) -> PathBuf {
        self.root.join("novels")
    }

    /// 辅助材料目录
    pub fn materials_dir(&self) -> PathBuf {
        self.root.join("materials")
    }

    /// AIGC 检测历史目录
    pub fn detection_dir(&self) -> PathBuf {
        self.root.join("detection")
    }

    /// 互动小说目录
    pub fn play_dir(&self) -> PathBuf {
        self.root.join("play")
    }

    /// 记忆系统根目录
    pub fn memory_root(&self) -> PathBuf {
        self.root.join("memories")
    }

    /// 工作区数据根目录
    pub fn workspaces_dir(&self) -> PathBuf {
        self.root.join("workspaces")
    }

    /// 单个工作区目录
    pub fn workspace_dir(&self, workspace_id: &str) -> PathBuf {
        self.workspaces_dir().join(workspace_id)
    }

    /// 工作区项目记忆路径
    pub fn workspace_memory_path(&self, workspace_id: &str) -> PathBuf {
        self.workspace_dir(workspace_id).join("project_memory.md")
    }

    // ── 文件路径 ──────────────────────────────────────────────────────────────

    /// 配置文件路径
    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    /// 状态数据库路径
    pub fn state_db_path(&self) -> PathBuf {
        self.data_dir().join("state.sqlite")
    }

    /// 日志数据库路径
    pub fn logs_db_path(&self) -> PathBuf {
        self.data_dir().join("logs.sqlite")
    }

    /// 反馈数据库路径
    pub fn feedback_db_path(&self) -> PathBuf {
        self.data_dir().join("feedback.sqlite")
    }

    // ── 配置初始化 ──────────────────────────────────────────────────────────

    /// 确保配置文件存在
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

// ── 测试模块 ────────────────────────────────────────────────────────────────

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