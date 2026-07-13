// 纯数据类型已迁移到 shared::wiki::models（避免 infrastructure → domain 反向依赖）。
// 此处保留 re-export 以兼容既有代码；新代码请直接从 crate::shared::wiki::models 引用。
pub use crate::shared::wiki::models::*;
