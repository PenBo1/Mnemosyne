// 双轨制状态管理 —— markdown 真相文件 + JSON 加速索引 + delta reducer。
//
// 模块组成：
// - types: RuntimeState 类型（HookRecord/RuntimeStateDelta/Snapshot 等）
// - manager: StateManager（control docs/book lock/chapter index）
// - store: runtime-state-store（load/save snapshot）
// - reducer: applyRuntimeStateDelta（应用 settler 输出的增量）
// - validator: validateRuntimeState（校验 snapshot 一致性）
// - bootstrap: bootstrapStructuredStateFromMarkdown（markdown → JSON 首次引导）

pub mod types;
pub mod manager;
pub mod store;
pub mod reducer;
pub mod validator;
pub mod bootstrap;

pub use types::*;
