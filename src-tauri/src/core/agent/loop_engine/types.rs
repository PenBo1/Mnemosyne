// Loop-Engineering 核心类型 —— 循环模式、预算、运行记录。
//
// 核心抽象:
// - LoopPattern:声明式循环配置(id/cadence/budget/skills/state_file)
// - LoopBudget:运行时预算阈值(80% 降级 / 100% 退出 / <5k early-exit)
// - LoopRun:单次运行记录(start/end/tokens/outcome/attempts)
// - LoopOutcome:运行结果分类(running/report-only/fix-proposed/escalated/no-op/failed)

use serde::{Deserialize, Serialize};

/// 循环模式 ID(对应 registry.yaml 中的 id 字段)
///
/// 内置 4 个 pattern,覆盖 Mnemosyne 现有 pipeline 的核心循环场景:
/// - ChapterWriteLoop:章节写作循环(Plan→Compose→Write)
/// - AuditReviseLoop:审计-修订循环(Audit→Revise→Re-audit,带 attempt cap)
/// - ObservationLoop:事实观察循环(observer 提取事实 → memory)
/// - ConsolidationLoop:章节归档循环(consolidator 压缩历史章节)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopPatternId {
    ChapterWriteLoop,
    AuditReviseLoop,
    ObservationLoop,
    ConsolidationLoop,
}

impl LoopPatternId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChapterWriteLoop => "chapter-write-loop",
            Self::AuditReviseLoop => "audit-revise-loop",
            Self::ObservationLoop => "observation-loop",
            Self::ConsolidationLoop => "consolidation-loop",
        }
    }

    pub fn state_file(&self) -> &'static str {
        match self {
            Self::ChapterWriteLoop => "chapter-write-state.md",
            Self::AuditReviseLoop => "audit-revise-state.md",
            Self::ObservationLoop => "observation-state.md",
            Self::ConsolidationLoop => "consolidation-state.md",
        }
    }
}

impl std::fmt::Display for LoopPatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 循环模式配置(对应 registry.yaml 中的单条 pattern)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopPattern {
    pub id: LoopPatternId,
    /// 一句话目标(对应 goal 字段)
    pub goal: &'static str,
    /// 最大尝试次数(对应 human_gates 中的 max-attempts)
    pub max_attempts: u32,
    /// 单次运行 token 上限(soft,超过降级到 report-only)
    pub tokens_per_run_cap: u64,
    /// 每日 token 上限(对应 cost.suggested_daily_cap)
    pub tokens_daily_cap: u64,
    /// 空闲早退阈值(<5k tokens 立即退出,不 spawn sub-agent)
    pub early_exit_tokens: u64,
    /// 是否强制 early-exit(high-cadence pattern 必须为 true)
    pub early_exit_required: bool,
}

impl LoopPattern {
    /// 获取指定 pattern 的默认配置
    pub fn get(id: &LoopPatternId) -> &'static Self {
        match id {
            LoopPatternId::ChapterWriteLoop => &BUILTIN_PATTERNS[0],
            LoopPatternId::AuditReviseLoop => &BUILTIN_PATTERNS[1],
            LoopPatternId::ObservationLoop => &BUILTIN_PATTERNS[2],
            LoopPatternId::ConsolidationLoop => &BUILTIN_PATTERNS[3],
        }
    }
}

/// 内置 4 个 pattern 的默认配置
///
/// 配置原则(对齐 loop-engineering/operating-loops.md):
/// - high-cost pattern(audit-revise)强制 early_exit_required = true
/// - 低风险 pattern(observation)放宽 daily_cap
/// - 所有 pattern 的 max_attempts = 3(对齐 failure-modes.md S2 缓解)
pub static BUILTIN_PATTERNS: [LoopPattern; 4] = [
    LoopPattern {
        id: LoopPatternId::ChapterWriteLoop,
        goal: "生成下一章正文:Plan → Compose → Write,带字数归一化",
        max_attempts: 1,
        tokens_per_run_cap: 200_000,
        tokens_daily_cap: 1_000_000,
        early_exit_tokens: 5_000,
        early_exit_required: false,
    },
    LoopPattern {
        id: LoopPatternId::AuditReviseLoop,
        goal: "审计-修订循环:Audit → Revise → Re-audit,带 attempt cap 和最佳快照回退",
        max_attempts: 3,
        tokens_per_run_cap: 300_000,
        tokens_daily_cap: 1_500_000,
        early_exit_tokens: 5_000,
        early_exit_required: true,
    },
    LoopPattern {
        id: LoopPatternId::ObservationLoop,
        goal: "事实观察:从章节提取角色/情节/设定事实,写入 memory",
        max_attempts: 1,
        tokens_per_run_cap: 60_000,
        tokens_daily_cap: 300_000,
        early_exit_tokens: 3_000,
        early_exit_required: false,
    },
    LoopPattern {
        id: LoopPatternId::ConsolidationLoop,
        goal: "章节归档:压缩历史章节摘要,释放 context 窗口",
        max_attempts: 1,
        tokens_per_run_cap: 80_000,
        tokens_daily_cap: 400_000,
        early_exit_tokens: 3_000,
        early_exit_required: false,
    },
];

/// 运行结果分类(对应 loop-run-log.md 中的 outcome 字段)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopOutcome {
    /// 运行中(未结束)
    Running,
    /// 仅报告(未执行修复,如 budget 80% 降级或无 actionable 项)
    ReportOnly,
    /// 已提出修复(执行了 revise 但未确认通过)
    FixProposed,
    /// 升级到人类(attempt 超限或 verifier ESCALATE_HUMAN)
    Escalated,
    /// 无操作(空 watchlist,<5k tokens 早退)
    NoOp,
    /// 失败(异常或 timeout)
    Failed,
}

impl LoopOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::ReportOnly => "report-only",
            Self::FixProposed => "fix-proposed",
            Self::Escalated => "escalated",
            Self::NoOp => "no-op",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "running" => Self::Running,
            "report-only" => Self::ReportOnly,
            "fix-proposed" => Self::FixProposed,
            "escalated" => Self::Escalated,
            "no-op" => Self::NoOp,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

impl Default for LoopOutcome {
    fn default() -> Self {
        Self::Running
    }
}

/// 单次循环运行记录(对应 loop-run-log.md 中的 JSON 条目)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopRun {
    /// ISO8601 时间戳作为唯一标识
    pub run_id: String,
    pub pattern_id: LoopPatternId,
    /// 关联书籍 ID(可为空)
    pub book_id: Option<String>,
    /// 关联章节号(可为空)
    pub chapter_number: Option<u32>,
    /// 开始时间 ISO8601
    pub started_at: String,
    /// 结束时间 ISO8601(运行中为 None)
    pub ended_at: Option<String>,
    /// 运行时长(秒)
    pub duration_s: Option<u64>,
    /// 运行结果
    pub outcome: LoopOutcome,
    /// 发现的可操作项数
    pub items_found: u32,
    /// 已执行的操作数
    pub actions_taken: u32,
    /// 升级到人类的事项数
    pub escalations: u32,
    /// token 估算(总)
    pub tokens_estimate: u64,
    /// prompt tokens
    pub prompt_tokens: u64,
    /// completion tokens
    pub completion_tokens: u64,
    /// total tokens
    pub total_tokens: u64,
    /// revise 循环尝试次数
    pub attempts: u32,
    /// 自由文本备注
    pub notes: Option<String>,
}

impl LoopRun {
    /// 创建新的运行记录(状态为 Running)
    pub fn new(pattern_id: LoopPatternId, book_id: Option<String>, chapter_number: Option<u32>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            run_id: now.clone(),
            pattern_id,
            book_id,
            chapter_number,
            started_at: now,
            ended_at: None,
            duration_s: None,
            outcome: LoopOutcome::Running,
            items_found: 0,
            actions_taken: 0,
            escalations: 0,
            tokens_estimate: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            attempts: 0,
            notes: None,
        }
    }

    /// 标记运行结束
    pub fn finish(&mut self, outcome: LoopOutcome) {
        let now = chrono::Utc::now().to_rfc3339();
        self.ended_at = Some(now);
        let started = chrono::DateTime::parse_from_rfc3339(&self.started_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        self.duration_s = Some(
            chrono::Utc::now()
                .signed_duration_since(started)
                .num_seconds()
                .max(0) as u64,
        );
        self.outcome = outcome;
    }

    /// 累加 token 用量
    pub fn add_usage(&mut self, prompt: u64, completion: u64) {
        self.prompt_tokens += prompt;
        self.completion_tokens += completion;
        self.total_tokens = self.prompt_tokens + self.completion_tokens;
        self.tokens_estimate = self.total_tokens;
    }
}

/// 预算检查结果(对应 loop-budget skill 的三档阈值)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetCheckResult {
    /// 允许执行(< 80% daily cap 且 ≥ early_exit_tokens)
    Allow,
    /// 降级到 report-only(≥ 80% daily cap 或 ≥ tokens_per_run_cap)
    DegradeToReportOnly { reason: String },
    /// 立即退出(≥ 100% daily cap 或 kill_switch 触发)
    Exit { reason: String },
    /// 早退(空 watchlist,< early_exit_tokens)
    EarlyExit { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_id_str_roundtrip() {
        assert_eq!(LoopPatternId::ChapterWriteLoop.as_str(), "chapter-write-loop");
        assert_eq!(LoopPatternId::AuditReviseLoop.as_str(), "audit-revise-loop");
        assert_eq!(LoopPatternId::ObservationLoop.as_str(), "observation-loop");
        assert_eq!(LoopPatternId::ConsolidationLoop.as_str(), "consolidation-loop");
    }

    #[test]
    fn pattern_get_returns_correct_config() {
        let p = LoopPattern::get(&LoopPatternId::AuditReviseLoop);
        assert_eq!(p.max_attempts, 3);
        assert!(p.early_exit_required);
        assert_eq!(p.tokens_daily_cap, 1_500_000);
    }

    #[test]
    fn builtin_patterns_cover_all_ids() {
        assert_eq!(BUILTIN_PATTERNS.len(), 4);
        assert!(BUILTIN_PATTERNS.iter().all(|p| p.max_attempts >= 1));
        assert!(BUILTIN_PATTERNS.iter().all(|p| p.tokens_daily_cap >= 100_000));
    }

    #[test]
    fn outcome_str_roundtrip() {
        for &o in &[
            LoopOutcome::Running,
            LoopOutcome::ReportOnly,
            LoopOutcome::FixProposed,
            LoopOutcome::Escalated,
            LoopOutcome::NoOp,
            LoopOutcome::Failed,
        ] {
            assert_eq!(LoopOutcome::from_str(o.as_str()), Some(o));
        }
        assert_eq!(LoopOutcome::from_str("unknown"), None);
    }

    #[test]
    fn loop_run_new_starts_running() {
        let run = LoopRun::new(LoopPatternId::AuditReviseLoop, Some("book1".into()), Some(5));
        assert_eq!(run.outcome, LoopOutcome::Running);
        assert_eq!(run.pattern_id, LoopPatternId::AuditReviseLoop);
        assert_eq!(run.book_id.as_deref(), Some("book1"));
        assert_eq!(run.chapter_number, Some(5));
        assert!(run.ended_at.is_none());
        assert_eq!(run.total_tokens, 0);
    }

    #[test]
    fn loop_run_finish_sets_outcome_and_duration() {
        let mut run = LoopRun::new(LoopPatternId::AuditReviseLoop, None, None);
        run.finish(LoopOutcome::Escalated);
        assert_eq!(run.outcome, LoopOutcome::Escalated);
        assert!(run.ended_at.is_some());
        assert!(run.duration_s.is_some());
    }

    #[test]
    fn loop_run_add_usage_accumulates() {
        let mut run = LoopRun::new(LoopPatternId::AuditReviseLoop, None, None);
        run.add_usage(100, 50);
        assert_eq!(run.prompt_tokens, 100);
        assert_eq!(run.completion_tokens, 50);
        assert_eq!(run.total_tokens, 150);
        assert_eq!(run.tokens_estimate, 150);

        run.add_usage(200, 100);
        assert_eq!(run.prompt_tokens, 300);
        assert_eq!(run.completion_tokens, 150);
        assert_eq!(run.total_tokens, 450);
    }
}
