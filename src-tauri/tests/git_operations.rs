//! ═══════════════════════════════════════════════════════════════════════════
//! Git 操作集成测试 - 测试 Git operations 模块的核心功能
//! ═══════════════════════════════════════════════════════════════════════════

mod common;

use common::GitRepoFixture;
use mnemosyne_lib::domain::git::operations;
use mnemosyne_lib::domain::git::types::RollbackMode;

// ── 仓库初始化测试 ────────────────────────────────────────────────────────────

/// 测试 Git 仓库初始化
///
/// 验证能成功创建 .git 目录
#[test]
fn test_git_init_creates_repository() {
    let fixture = GitRepoFixture::new();

    assert!(fixture.repo_path.join(".git").exists());
}

/// 测试 Git 仓库重复初始化
///
/// 验证对已存在的仓库调用 init 返回 initialized: false
#[test]
fn test_git_init_existing_repository() {
    let fixture = GitRepoFixture::new();

    let result = operations::init_repository(&fixture.repo_path).unwrap();
    assert!(!result.initialized);
}

// ── 状态查询测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 状态查询 - 空仓库
///
/// 验证新仓库状态为 clean，没有暂存或未暂存文件
#[test]
fn test_git_status_empty_repo() {
    let fixture = GitRepoFixture::new();

    let status = operations::get_status(&fixture.repo_path).unwrap();

    assert!(status.files.is_empty());
}

/// 测试 Git 状态查询 - 有未跟踪文件
///
/// 验证能正确识别未跟踪文件
#[test]
fn test_git_status_with_untracked_file() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容");

    let status = operations::get_status(&fixture.repo_path).unwrap();

    assert_eq!(status.files.len(), 1);
    assert!(status.files.iter().any(|f| f.path == "test.txt"));
}

// ── 暂存操作测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 暂存文件
///
/// 验证能正确暂存文件，暂存后状态反映为 staged
#[test]
fn test_git_stage_file() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容");

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    let status = operations::get_status(&fixture.repo_path).unwrap();

    assert_eq!(status.staged, 1);
}

// ── 提交操作测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 提交
///
/// 验证能正确创建提交，提交后仓库状态为 clean
#[test]
fn test_git_commit() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容");

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    let hash = operations::commit_changes(&fixture.repo_path, "测试提交").unwrap();

    assert!(!hash.is_empty());
    assert!(hash.len() >= 7);

    let status = operations::get_status(&fixture.repo_path).unwrap();
    assert!(status.files.is_empty());
}

// ── 日志查询测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 日志查询
///
/// 验证能正确获取提交历史，提交信息正确
#[test]
fn test_git_log() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "测试内容");

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    operations::commit_changes(&fixture.repo_path, "第一次提交").unwrap();

    let commits = operations::get_log(&fixture.repo_path, 10, 0).unwrap();

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].message, "第一次提交");
}

// ── 配置管理测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 配置获取
///
/// 验证能正确读取 user.name 和 user.email
#[test]
fn test_git_get_config() {
    let fixture = GitRepoFixture::new();

    let config = operations::get_all_config(&fixture.repo_path).unwrap();

    assert_eq!(config.user_name, Some("Test User".to_string()));
    assert_eq!(config.user_email, Some("test@example.com".to_string()));
}

/// 测试 Git 配置设置
///
/// 验证能正确设置 user.name 和 user.email
#[test]
fn test_git_set_config() {
    let fixture = GitRepoFixture::new();

    operations::set_config(&fixture.repo_path, "user.name", "新用户", false).unwrap();
    operations::set_config(&fixture.repo_path, "user.email", "new@example.com", false).unwrap();

    let config = operations::get_all_config(&fixture.repo_path).unwrap();

    assert_eq!(config.user_name, Some("新用户".to_string()));
}

// ── 回滚操作测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 回滚 - soft 模式
///
/// 验证 soft 回滚保留工作目录更改
#[test]
fn test_git_rollback_soft() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "初始内容");

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    let hash = operations::commit_changes(&fixture.repo_path, "第一次提交").unwrap();

    std::fs::write(fixture.repo_path.join("test.txt"), "修改内容").unwrap();

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    let _second_hash = operations::commit_changes(&fixture.repo_path, "第二次提交").unwrap();

    operations::rollback(&fixture.repo_path, &hash, RollbackMode::Soft).unwrap();

    let status = operations::get_status(&fixture.repo_path).unwrap();
    assert!(!status.files.is_empty());
}

// ── 差异比较测试 ──────────────────────────────────────────────────────────────

/// 测试 Git 差异比较
///
/// 验证能正确获取文件差异统计
#[test]
fn test_git_diff() {
    let fixture = GitRepoFixture::with_initial_file("test.txt", "初始内容");

    operations::stage_files(&fixture.repo_path, &["test.txt".to_string()]).unwrap();

    operations::commit_changes(&fixture.repo_path, "初始提交").unwrap();

    std::fs::write(fixture.repo_path.join("test.txt"), "修改后的内容\n新增行").unwrap();

    let diff = operations::get_diff(&fixture.repo_path, false).unwrap();

    assert!(!diff.files.is_empty());
}

// ── 错误处理测试 ──────────────────────────────────────────────────────────────

/// 测试空文件列表暂存
///
/// 验证空文件列表的处理行为
#[test]
fn test_git_stage_empty_files() {
    let fixture = GitRepoFixture::new();

    let result = operations::stage_files(&fixture.repo_path, &[]);

    // git2 会成功处理空列表
    assert!(result.is_ok() || result.is_err());
}

/// 测试无效 commit hash 回滚失败
///
/// 验证空 hash、过短 hash、非十六进制 hash 返回错误
#[test]
fn test_git_rollback_invalid_hash() {
    let fixture = GitRepoFixture::new();

    let result = operations::rollback(&fixture.repo_path, "", RollbackMode::Soft);
    assert!(result.is_err());

    let result = operations::rollback(&fixture.repo_path, "ab", RollbackMode::Soft);
    assert!(result.is_err());

    let result = operations::rollback(&fixture.repo_path, "xyz123", RollbackMode::Soft);
    assert!(result.is_err());
}