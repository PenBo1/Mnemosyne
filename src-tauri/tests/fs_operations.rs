//! 文件系统操作集成测试
//!
//! 测试文件系统模块的核心功能：
//! - DataDir 目录管理
//! - 文件名安全处理

mod common;

use common::FsTestFixture;
use mnemosyne_lib::infrastructure::fs::data_dir::DataDir;

/// 测试 DataDir 初始化
///
/// 验证：
/// - 能正确创建所有必需目录
/// - 目录结构完整
#[test]
fn test_data_dir_initialize() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    data_dir.initialize().expect("DataDir 初始化失败");
    
    assert!(fixture.work_dir.exists());
    assert!(data_dir.data_dir().exists());
    assert!(data_dir.logs_dir().exists());
}

/// 测试 DataDir 子目录访问
///
/// 验证：
/// - data_dir() 返回正确路径
/// - logs_dir() 返回正确路径
#[test]
fn test_data_dir_subdirectories() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    data_dir.initialize().expect("DataDir 初始化失败");
    
    let data_subdir = data_dir.data_dir();
    assert!(data_subdir.ends_with("data"));
    
    let logs_subdir = data_dir.logs_dir();
    assert!(logs_subdir.ends_with("logs"));
}

/// 测试 FsTestFixture 子目录创建
///
/// 验证：
/// - 能创建嵌套子目录
#[test]
fn test_fixture_create_subdir() {
    let fixture = FsTestFixture::new();
    
    let subdir = fixture.create_subdir("chapters");
    assert!(subdir.exists());
    assert!(subdir.ends_with("chapters"));
    
    let nested = fixture.create_subdir("chapters/volume1");
    assert!(nested.exists());
}

/// 测试 FsTestFixture 文件创建
///
/// 验证：
/// - 能创建带内容的文件
/// - 自动创建父目录
#[test]
fn test_fixture_create_file() {
    let fixture = FsTestFixture::new();
    
    let file_path = fixture.create_file("test.txt", "测试内容");
    assert!(file_path.exists());
    
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "测试内容");
}

/// 测试嵌套路径文件创建
///
/// 验证：
/// - 能自动创建多层父目录
#[test]
fn test_fixture_create_nested_file() {
    let fixture = FsTestFixture::new();
    
    let file_path = fixture.create_file("deep/nested/path/file.md", "嵌套文件内容");
    assert!(file_path.exists());
    
    let parent = file_path.parent().unwrap();
    assert!(parent.ends_with("deep/nested/path"));
}

/// 测试 DataDir 路径拼接
///
/// 验证：
/// - join_path 正确拼接路径
#[test]
fn test_data_dir_path_join() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    let config_path = data_dir.root().join("config.json");
    let expected = fixture.work_dir.join("config.json");
    assert_eq!(config_path, expected);
}

/// 测试 DataDir skills 目录
///
/// 验证：
/// - skills_dir() 返回正确路径
#[test]
fn test_data_dir_skills() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    data_dir.initialize().expect("DataDir 初始化失败");
    
    let skills_dir = data_dir.skills_dir();
    assert!(skills_dir.ends_with("skills"));
}

/// 测试 DataDir books 目录
///
/// 验证：
/// - books_dir() 返回正确路径
#[test]
fn test_data_dir_books() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    data_dir.initialize().expect("DataDir 初始化失败");
    
    let books_dir = data_dir.books_dir();
    assert!(books_dir.ends_with("books"));
}

/// 测试 DataDir agents 目录
///
/// 验证：
/// - agents_dir() 返回正确路径
#[test]
fn test_data_dir_agents() {
    let fixture = FsTestFixture::new();
    let data_dir = DataDir::new(fixture.work_dir.clone());
    
    data_dir.initialize().expect("DataDir 初始化失败");
    
    let agents_dir = data_dir.agents_dir();
    assert!(agents_dir.ends_with("agents"));
}