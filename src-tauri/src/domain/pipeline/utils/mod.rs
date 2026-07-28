//! ═══════════════════════════════════════════════════════════════════════════
//! Pipeline Utils - 工具模块
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 纯函数 / 规则计算，无 LLM 调用。
//!
//! 模块组成：
//! - hook_promotion: rerun_promotion_pass（从 chapter_summaries 推导 advancedCount 并晋升 hook）
//! - hook_governance: evaluate_hook_admission（重复 family 检测，准入决策）
//! - story_markdown: markdown 真相文件 -> 结构化解析器
//! - text_parse: 文本解析共享工具（count_zh_chars / strip_code_fence / extract_section 等）
//! - fs_safety: 文件系统安全段 / slugify（路径与文件名安全化）

// ── 模块声明 ────────────────────────────────────────────────────────────────

pub mod hook_promotion;
pub mod hook_governance;
pub mod story_markdown;
pub mod text_parse;
pub mod fs_safety;
