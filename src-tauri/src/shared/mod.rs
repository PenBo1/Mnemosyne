// ============================================================================
// shared —— 跨层共享类型（解决 infra 反向依赖）
// ============================================================================
//
// 设计理由（为什么要有 shared/）：
// 分层架构要求 infra 只依赖 shared，不依赖任何业务域或 agent。但 infra/db
// 的 store 实现需要操作业务领域的数据结构（如 WikiCategory、ChapterVersion、
// MemoryEntry）。如果把这些类型放业务域，infra 就要反向依赖业务域，破坏
// 依赖图单向性。解决方案：把"被 infra 和业务域同时需要"的纯数据类型下沉
// 到 shared/，让 infra 和业务域都依赖 shared，互不依赖对方。
//
// 职责边界：
// 只放纯数据类型（struct/enum）与无副作用的纯函数。不放业务逻辑、不放 I/O、
// 不放 trait 定义（trait 属于各自的域）。本模块是"数据契约层"，不是"公共
// 工具箱"——只有"跨层共享"的类型才放这里，单域专用的类型留在各自域内。
//
// 依赖规则：
// - 仅依赖 errors（可选）、std
// - 严禁依赖任何业务域、agent、infra、Tauri
// - 严禁包含业务逻辑、I/O、trait 定义
//
// 被依赖方：
// - infra/（db store 操作这些类型）
// - 所有业务域（story/session/version/wiki/novel/radar/user_profile）
// - agent/（agent 上下文构建时引用这些类型）
// - app/commands/（IPC 边界的数据传递）
//
// 内容：
// - wiki.rs:    WikiCategory, WikiSourceType（wiki 域与 infra/wiki_store 共享）
// - version.rs: ChapterVersion, CreateVersionRequest, RevisionMode（version 域与 infra/version_store 共享）
// - memory.rs:  MemoryEntry, MemoryType（agent 与 infra/memory 共享）
// - text.rs:    count_words 纯函数（多域共享的字数统计）
//
// 下沉判定标准（什么类型应下沉到 shared/）：
// 1. 被 infra/ 与业务域同时需要 → 必须下沉
// 2. 被两个及以上业务域同时需要 → 应当下沉
// 3. 仅被单一业务域需要 → 留在域内，不下沉
// 4. 包含行为（方法/trait）→ 拆分：数据下沉，行为留域
pub mod errors;
pub mod memory;
pub mod text;
pub mod version;
pub mod wiki;
