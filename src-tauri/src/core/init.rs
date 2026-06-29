// 应用初始化编排 —— 承接 data_dir 的业务初始化上提
//
// 职责边界：在 DataDir 完成目录创建后，执行业务级初始化（提取内置资源、生成默认身份文件等）
// 依赖规则：可依赖 infra（data_dir）+ 业务域（novel::source、agent::identity）
//
// 设计理由：原 infra/data_dir.rs 反向调用 domain::novel::source 和 domain::agents::identity，
// 违反"infra 不依赖业务"原则。本模块将业务初始化上提到 app 层编排，infra 只负责目录结构。

use crate::shared::errors::AppError;
use crate::infrastructure::file_storage::data_dir::DataDir;

/// 执行业务级初始化（在 data_dir.initialize() 之后调用）
pub fn initialize_app_business_state(_data_dir: &DataDir) -> Result<(), AppError> {
    // TODO: 从原 data_dir.rs 迁移业务初始化逻辑
    // 1. novel::source::extract_builtin_sources_to_dir(&data_dir.novel_sources_dir())
    // 2. agent::identity::ensure_default_files(&data_dir.agents_dir())
    Ok(())
}
