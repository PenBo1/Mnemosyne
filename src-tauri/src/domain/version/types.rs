// 纯数据类型已迁移到 shared::version::types（避免 infrastructure → domain 反向依赖）。
// 此处保留 re-export 以兼容既有代码；新代码请直接从 crate::shared::version::types 引用。
pub use crate::shared::version::types::*;
