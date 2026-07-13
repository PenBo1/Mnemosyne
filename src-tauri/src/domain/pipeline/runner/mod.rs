// Pipeline Runner —— pipeline 编排层。
//
// 子模块：
// - chapter_review_cycle: Audit↔Revise 评分循环（带最佳快照回退）
// - chapter_state_recovery: 状态校验失败恢复（重试结算 + 降级问题构建 + review note 解析）
// - chapter_truth_validation: 真相文件持久化校验（含状态恢复与降级标记）

pub mod chapter_review_cycle;
pub mod chapter_state_recovery;
pub mod chapter_truth_validation;
pub mod pipeline_runner;
pub mod short_fiction_runner;
pub mod script_storyboard_runner;

pub use pipeline_runner::{
    ChapterPipelineResult, ComposeChapterResult, PipelineConfig, PipelineRunner,
    PlanChapterResult, ReviseResult,
};
