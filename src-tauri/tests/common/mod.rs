//! 公共测试 fixture 模块
//!
//! 提供集成测试所需的测试环境：
//! - GitRepoFixture: Git 仓库测试 fixture
//! - FsTestFixture: 文件系统测试 fixture

use tempfile::TempDir;
use std::path::PathBuf;

use mnemosyne_lib::domain::git::operations::GitOperations;
use mnemosyne_lib::domain::git::types::GitConfig;

/// Git 仓库测试 fixture
///
/// 提供一个临时 Git 仓库环境，用于测试 Git 操作：
/// - 自动创建临时目录
/// - 自动初始化 Git 仓库
/// - 提供基本配置（user.name / user.email）
/// - 测试结束后自动清理
pub struct GitRepoFixture {
    /// 仓库路径
    pub repo_path: PathBuf,
    /// 临时目录（保持存活以防止清理）
    _tmp: TempDir,
}

impl GitRepoFixture {
    /// 创建新的 Git 仓库 fixture
    ///
    /// # 返回
    /// 包含已初始化 Git 仓库的 fixture
    pub async fn new() -> Self {
        let tmp = TempDir::new().expect("无法创建临时目录");
        let repo_path = tmp.path().to_path_buf();

        GitOperations::init(&repo_path)
            .await
            .expect("Git 初始化失败");

        GitOperations::set_config(&repo_path, &GitConfig {
            user_name: Some("Test User".into()),
            user_email: Some("test@example.com".into()),
            auto_stage: false,
            commit_message_template: None,
            enable_remote: false,
        })
        .await
        .expect("Git 配置失败");

        Self { repo_path, _tmp: tmp }
    }

    /// 创建带有初始文件的 fixture
    pub async fn with_initial_file(filename: &str, content: &str) -> Self {
        let fixture = Self::new().await;
        let file_path = fixture.repo_path.join(filename);
        std::fs::write(&file_path, content).expect("写入文件失败");
        fixture
    }
}

/// 文件系统测试 fixture
///
/// 提供一个临时文件系统环境：
/// - 创建临时工作目录
/// - 支持创建测试文件结构
/// - 自动清理
pub struct FsTestFixture {
    /// 工作目录路径
    pub work_dir: PathBuf,
    /// 临时目录
    _tmp: TempDir,
}

impl FsTestFixture {
    /// 创建新的文件系统 fixture
    pub fn new() -> Self {
        let tmp = TempDir::new().expect("无法创建临时目录");
        let work_dir = tmp.path().to_path_buf();
        Self { work_dir, _tmp: tmp }
    }

    /// 创建子目录
    pub fn create_subdir(&self, name: &str) -> PathBuf {
        let subdir = self.work_dir.join(name);
        std::fs::create_dir_all(&subdir).expect("创建子目录失败");
        subdir
    }

    /// 创建测试文件
    pub fn create_file(&self, relative_path: &str, content: &str) -> PathBuf {
        let file_path = self.work_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("创建父目录失败");
        }
        std::fs::write(&file_path, content).expect("写入文件失败");
        file_path
    }
}

impl Default for FsTestFixture {
    fn default() -> Self {
        Self::new()
    }
}