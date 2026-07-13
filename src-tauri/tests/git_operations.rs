//! Git 操作集成测试
//!
//! 测试 GitOperations 模块的核心功能：
//! - 仓库初始化
//! - 状态查询
//! - 提交日志
//! - 文件暂存和提交
//! - 配置管理
//!
//! 安全性测试：
//! - commit hash 验证
//! - 路径安全

mod common;

use common::GitRepoFixture;
use mnemosyne_lib::domain::git::operations::GitOperations;
use mnemosyne_lib::domain::git::types::{GitConfig, RollbackMode};

/// 测试 Git 仓库初始化
///
/// 验证：
/// - 能成功创建 .git 目录
/// - 重复初始化返回 initialized: false
#[tokio::test]
async fn test_git_init_creates_repository() {
    let fixture = GitRepoFixture::new().await;
    
    assert!(fixture.repo_path.join(".git").exists());
}

/// 测试 Git 仓库重复初始化
///
/// 验证：
/// - 对已存在的仓库调用 init 返回 initialized: false
#[tokio::test]
async fn test_git_init_existing_repository() {
    let fixture = GitRepoFixture::new().await;
    
    let result = GitOperations::init(&fixture.repo_path).await.unwrap();
    assert!(!result.initialized);
}

/// 测试 Git 状态查询 - 空仓库
///
/// 验证：
/// - 新仓库状态为 clean
/// - 没有暂存或未暂存文件
#[tokio::test]
async fn test_git_status_empty_repo() {
    let fixture = GitRepoFixture::new().await;
    
    let status = GitOperations::status(&fixture.repo_path).await.unwrap();
    
    assert!(status.is_clean);
    assert!(status.staged.is_empty());
    assert!(status.unstaged.is_empty());
    assert!(status.untracked.is_empty());
}

/// 测试 Git 状态查询 - 有未跟踪文件
///
/// 验证：
/// - 能正确识别未跟踪文件
/// - is_clean 为 false
#[tokio::test]
async fn test_git_status_with_untracked_file() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容").await;
    
    let status = GitOperations::status(&fixture.repo_path).await.unwrap();
    
    assert!(!status.is_clean);
    assert_eq!(status.untracked.len(), 1);
    assert!(status.untracked.contains(&"test.txt".to_string()));
}

/// 测试 Git 暂存文件
///
/// 验证：
/// - 能正确暂存文件
/// - 暂存后状态反映为 staged
#[tokio::test]
async fn test_git_stage_file() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容").await;
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    let status = GitOperations::status(&fixture.repo_path).await.unwrap();
    
    assert_eq!(status.staged.len(), 1);
    assert_eq!(status.staged[0].path, "test.txt");
    assert_eq!(status.staged[0].status, "added");
}

/// 测试 Git 提交
///
/// 验证：
/// - 能正确创建提交
/// - 提交后仓库状态为 clean
#[tokio::test]
async fn test_git_commit() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容").await;
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    let hash = GitOperations::commit(&fixture.repo_path, "测试提交")
        .await
        .unwrap();
    
    assert!(!hash.is_empty());
    assert!(hash.len() >= 7);
    
    let status = GitOperations::status(&fixture.repo_path).await.unwrap();
    assert!(status.is_clean);
}

/// 测试 Git 日志查询
///
/// 验证：
/// - 能正确获取提交历史
/// - 提交信息正确
#[tokio::test]
async fn test_git_log() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容").await;
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    GitOperations::commit(&fixture.repo_path, "第一次提交")
        .await
        .unwrap();
    
    let commits = GitOperations::log(&fixture.repo_path, 10).await.unwrap();
    
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].message, "第一次提交");
}

/// 测试 Git 配置获取
///
/// 验证：
/// - 能正确读取 user.name 和 user.email
#[tokio::test]
async fn test_git_get_config() {
    let fixture = GitRepoFixture::new().await;
    
    let config = GitOperations::get_config(&fixture.repo_path).await.unwrap();
    
    assert_eq!(config.user_name, Some("Test User".to_string()));
    assert_eq!(config.user_email, Some("test@example.com".to_string()));
}

/// 测试 Git 配置设置
///
/// 验证：
/// - 能正确设置 auto_stage
#[tokio::test]
async fn test_git_set_config() {
    let fixture = GitRepoFixture::new().await;
    
    let new_config = GitConfig {
        user_name: Some("新用户".into()),
        user_email: Some("new@example.com".into()),
        auto_stage: true,
        commit_message_template: Some("{message}".into()),
        enable_remote: false,
    };
    
    GitOperations::set_config(&fixture.repo_path, &new_config)
        .await
        .unwrap();
    
    let config = GitOperations::get_config(&fixture.repo_path).await.unwrap();
    
    assert_eq!(config.user_name, Some("新用户".to_string()));
    assert!(config.auto_stage);
}

/// 测试 Git 回滚 - soft 模式
///
/// 验证：
/// - soft 回滚保留工作目录更改
#[tokio::test]
async fn test_git_rollback_soft() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "初始内容").await;
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    let hash = GitOperations::commit(&fixture.repo_path, "第一次提交")
        .await
        .unwrap();
    
    std::fs::write(fixture.repo_path.join("test.txt"), "修改内容").unwrap();
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    let _second_hash = GitOperations::commit(&fixture.repo_path, "第二次提交")
        .await
        .unwrap();
    
    GitOperations::rollback(&fixture.repo_path, &hash, RollbackMode::Soft)
        .await
        .unwrap();
    
    let status = GitOperations::status(&fixture.repo_path).await.unwrap();
    assert!(!status.is_clean);
}

/// 测试 Git 差异比较
///
/// 验证：
/// - 能正确获取文件差异统计
#[tokio::test]
async fn test_git_diff() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "初始内容").await;
    
    GitOperations::stage(&fixture.repo_path, &["test.txt".to_string()])
        .await
        .unwrap();
    
    GitOperations::commit(&fixture.repo_path, "初始提交")
        .await
        .unwrap();
    
    std::fs::write(fixture.repo_path.join("test.txt"), "修改后的内容\n新增行").unwrap();
    
    let diff = GitOperations::diff(&fixture.repo_path, None).await.unwrap();
    
    assert!(!diff.files.is_empty());
}

/// 测试空文件列表暂存失败
///
/// 验证：
/// - 空文件列表返回错误
#[tokio::test]
async fn test_git_stage_empty_files_error() {
    let fixture = GitRepoFixture::new().await;
    
    let result = GitOperations::stage(&fixture.repo_path, &[]).await;
    
    assert!(result.is_err());
}

/// 测试无效 commit hash 回滚失败
///
/// 验证：
/// - 空 hash 返回错误
/// - 过短 hash 返回错误
/// - 非十六进制 hash 返回错误
#[tokio::test]
async fn test_git_rollback_invalid_hash() {
    let fixture = GitRepoFixture::new().await;
    
    let result = GitOperations::rollback(&fixture.repo_path, "", RollbackMode::Soft).await;
    assert!(result.is_err());
    
    let result = GitOperations::rollback(&fixture.repo_path, "ab", RollbackMode::Soft).await;
    assert!(result.is_err());
    
    let result = GitOperations::rollback(&fixture.repo_path, "xyz123", RollbackMode::Soft).await;
    assert!(result.is_err());
}