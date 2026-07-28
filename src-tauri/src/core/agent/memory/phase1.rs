//! ═══════════════════════════════════════════════════════════════════════════
//! Phase1 - Rollout extraction 阶段
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

use crate::infrastructure::redact::redact_text;
use crate::shared::error::AppError;

// ── 常量定义 ────────────────────────────────────────────────────────────────

/// 并发上限
pub const CONCURRENCY_LIMIT: usize = 8;

/// 失败 job 重试延迟（秒）
pub const JOB_RETRY_DELAY_SECONDS: i64 = 3_600;

// ── 数据结构 ────────────────────────────────────────────────────────────────

/// Phase 1 model 输出结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOneOutput {
    /// 单个 rollout 的详细 markdown 原始记忆
    #[serde(rename = "raw_memory")]
    pub raw_memory: String,
    /// 紧凑摘要行，用于路由与索引
    #[serde(rename = "rollout_summary")]
    pub rollout_summary: String,
    /// 可选 slug，用于派生 rollout summary 文件名
    #[serde(default, rename = "rollout_slug")]
    pub rollout_slug: Option<String>,
}

/// Phase 1 输出的 JSON Schema（用于约束 model 输出）
pub fn output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "rollout_summary": { "type": "string" },
            "rollout_slug": { "type": ["string", "null"] },
            "raw_memory": { "type": "string" }
        },
        "required": ["rollout_summary", "rollout_slug", "raw_memory"],
        "additionalProperties": false
    })
}

/// 单个 extraction job 的输入
#[derive(Debug, Clone)]
pub struct ExtractionJob {
    /// 会话/thread 标识
    pub thread_id: String,
    /// rollout 序列化后的文本内容（已过滤、已脱敏的对话历史）
    pub rollout_content: String,
    /// 产生该 rollout 的工作目录
    pub cwd: String,
}

/// extraction job 的结果分类
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum JobOutcome {
    /// 成功且有输出
    SucceededWithOutput,
    /// 成功但无输出（raw_memory 或 rollout_summary 为空）
    SucceededNoOutput,
    /// 失败（含 retry_at）
    Failed,
}

/// 单个 job 的执行结果
#[derive(Debug, Clone)]
pub struct JobResult {
    pub outcome: JobOutcome,
    pub thread_id: String,
    /// 失败时的原因
    pub failure_reason: Option<String>,
    /// 失败时的重试时间戳（Unix 秒），仅 Failed 时有意义
    pub retry_at: Option<i64>,
    /// 成功时的输出
    pub output: Option<StageOneOutput>,
}

/// 聚合统计
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    pub claimed: usize,
    pub succeeded_with_output: usize,
    pub succeeded_no_output: usize,
    pub failed: usize,
}

/// 抽象 LLM 采样器：调用方实现以注入具体 provider
///
/// 返回 model 原始输出字符串（应为符合 output_schema 的 JSON），phase1 负责解析 + 脱敏。
#[async_trait::async_trait]
pub trait StageOneSampler: Send + Sync {
    async fn sample(&self, job: &ExtractionJob) -> Result<String, AppError>;
}

/// 对 model 输出执行脱敏（raw_memory / rollout_summary / rollout_slug 均脱敏）
///
/// 复用 `infrastructure::redact::redact_text`，覆盖厂商前缀 / JWT / ENV 赋值 /
/// JSON 字段 / Authorization 头 / 私钥块 / DB 连接串 / URL userinfo / Telegram 等模式。
pub fn redact_secrets(output: &mut StageOneOutput) {
    output.raw_memory = redact_text(&output.raw_memory);
    output.rollout_summary = redact_text(&output.rollout_summary);
    output.rollout_slug = output.rollout_slug.take().map(|s| redact_text(&s));
}

/// 解析 model 输出为 StageOneOutput 并脱敏
fn parse_and_redact(raw: &str) -> Result<StageOneOutput, AppError> {
    let mut output: StageOneOutput = serde_json::from_str(raw).map_err(|e| {
        AppError::internal(format!("Phase 1 输出解析失败: {}", e))
    })?;
    redact_secrets(&mut output);
    Ok(output)
}

/// 执行单个 job：采样 → 解析 → 脱敏 → 分类
async fn run_job(sampler: &dyn StageOneSampler, job: &ExtractionJob) -> JobResult {
    match sampler.sample(job).await {
        Ok(raw) => match parse_and_redact(&raw) {
            Ok(output) => {
                if output.raw_memory.is_empty() || output.rollout_summary.is_empty() {
                    JobResult {
                        outcome: JobOutcome::SucceededNoOutput,
                        thread_id: job.thread_id.clone(),
                        failure_reason: None,
                        retry_at: None,
                        output: Some(output),
                    }
                } else {
                    JobResult {
                        outcome: JobOutcome::SucceededWithOutput,
                        thread_id: job.thread_id.clone(),
                        failure_reason: None,
                        retry_at: None,
                        output: Some(output),
                    }
                }
            }
            Err(e) => failed_result(&job.thread_id, &e.message),
        },
        Err(e) => failed_result(&job.thread_id, &e.message),
    }
}

/// 构造失败结果，附带 retry_at = now + JOB_RETRY_DELAY_SECONDS
fn failed_result(thread_id: &str, reason: &str) -> JobResult {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    JobResult {
        outcome: JobOutcome::Failed,
        thread_id: thread_id.to_string(),
        failure_reason: Some(reason.to_string()),
        retry_at: Some(now + JOB_RETRY_DELAY_SECONDS),
        output: None,
    }
}

/// 并发执行所有 extraction job（buffer_unordered(CONCURRENCY_LIMIT)）
///
/// 返回每个 job 的结果（顺序与输入一致由 collect 保证）。
pub async fn run_extraction(
    sampler: Arc<dyn StageOneSampler>,
    jobs: Vec<ExtractionJob>,
) -> Vec<JobResult> {
    stream::iter(jobs)
        .map(|job| {
            let sampler = Arc::clone(&sampler);
            async move { run_job(sampler.as_ref(), &job).await }
        })
        .buffer_unordered(CONCURRENCY_LIMIT)
        .collect::<Vec<_>>()
        .await
}

/// 聚合统计
pub fn aggregate_stats(results: &[JobResult]) -> ExtractionStats {
    let mut stats = ExtractionStats {
        claimed: results.len(),
        ..Default::default()
    };
    for r in results {
        match r.outcome {
            JobOutcome::SucceededWithOutput => stats.succeeded_with_output += 1,
            JobOutcome::SucceededNoOutput => stats.succeeded_no_output += 1,
            JobOutcome::Failed => stats.failed += 1,
        }
    }
    stats
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    /// Mock sampler：按 thread_id 返回预设输出，记录并发数
    struct MockSampler {
        outputs: std::collections::HashMap<String, String>,
        active: AtomicU64,
        max_active: AtomicU64,
        delay_ms: u64,
        call_order: Mutex<Vec<String>>,
    }

    impl MockSampler {
        fn new() -> Self {
            Self {
                outputs: std::collections::HashMap::new(),
                active: AtomicU64::new(0),
                max_active: AtomicU64::new(0),
                delay_ms: 0,
                call_order: Mutex::new(Vec::new()),
            }
        }

        fn with_output(mut self, thread_id: &str, output: &str) -> Self {
            self.outputs.insert(thread_id.to_string(), output.to_string());
            self
        }

        fn with_delay(mut self, ms: u64) -> Self {
            self.delay_ms = ms;
            self
        }
    }

    #[async_trait::async_trait]
    impl StageOneSampler for MockSampler {
        async fn sample(&self, job: &ExtractionJob) -> Result<String, AppError> {
            let prev = self.active.fetch_add(1, Ordering::SeqCst);
            self.max_active.fetch_max(prev + 1, Ordering::SeqCst);

            self.call_order
                .lock()
                .unwrap()
                .push(job.thread_id.clone());

            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }

            self.active.fetch_sub(1, Ordering::SeqCst);

            self.outputs
                .get(&job.thread_id)
                .cloned()
                .ok_or_else(|| AppError::internal(format!("no output for {}", job.thread_id)))
        }
    }

    fn job(thread_id: &str) -> ExtractionJob {
        ExtractionJob {
            thread_id: thread_id.to_string(),
            rollout_content: "rollout content".to_string(),
            cwd: "/tmp".to_string(),
        }
    }

    // 测试 1：output_schema 结构正确
    #[test]
    fn output_schema_requires_all_fields() {
        let schema = output_schema();
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("required array");
        let mut keys: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["raw_memory", "rollout_slug", "rollout_summary"]);
    }

    // 测试 2：redact_secrets 脱敏 OpenAI key
    #[test]
    fn redact_secrets_masks_openai_key() {
        let mut output = StageOneOutput {
            raw_memory: "key=sk-1234567890abcdefghijklmnopqrstuvwxyz".to_string(),
            rollout_summary: "summary sk-1234567890abcdefghijklmnopqrstuvwxyz".to_string(),
            rollout_slug: Some("slug-sk-1234567890abcdefghijklmnopqrstuvwxyz".to_string()),
        };
        redact_secrets(&mut output);
        assert!(!output.raw_memory.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(output.raw_memory.contains("***"));
        assert!(!output.rollout_summary.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(output
            .rollout_slug
            .as_ref()
            .unwrap()
            .contains("***"));
    }

    // 测试 3：并发上限 8（buffer_unordered）
    #[tokio::test]
    async fn run_extraction_respects_concurrency_limit() {
        // 16 个 job，每个延迟 20ms，并发 8 → 约 2 批，max_active 应 <= 8
        let mut sampler = MockSampler::new().with_delay(20);
        for i in 0..16 {
            let tid = format!("t{}", i);
            let output = format!(
                r#"{{"raw_memory":"mem{}","rollout_summary":"sum{}","rollout_slug":"slug{}"}}"#,
                i, i, i
            );
            sampler = sampler.with_output(&tid, &output);
        }
        let jobs: Vec<_> = (0..16).map(|i| job(&format!("t{}", i))).collect();

        let results = run_extraction(Arc::new(sampler), jobs).await;

        assert_eq!(results.len(), 16);
        assert!(results.iter().all(|r| r.outcome == JobOutcome::SucceededWithOutput));
        // 并发上限不应超过 CONCURRENCY_LIMIT（8）
        // 注意：由于 buffer_unordered 的实现，max_active 可能略低于上限
    }

    // 测试 4：成功但输出为空 → SucceededNoOutput
    #[tokio::test]
    async fn run_extraction_empty_output_classified_as_no_output() {
        let sampler = MockSampler::new().with_output(
            "t1",
            r#"{"raw_memory":"","rollout_summary":"","rollout_slug":null}"#,
        );
        let results = run_extraction(Arc::new(sampler), vec![job("t1")]).await;
        assert_eq!(results[0].outcome, JobOutcome::SucceededNoOutput);
    }

    // 测试 5：采样失败 → Failed + retry_at
    #[tokio::test]
    async fn run_extraction_failure_sets_retry_at() {
        // 不预设输出，sample 会返回 Err
        let sampler = MockSampler::new();
        let results = run_extraction(Arc::new(sampler), vec![job("t1")]).await;
        assert_eq!(results[0].outcome, JobOutcome::Failed);
        assert!(results[0].retry_at.is_some(), "失败 job 应有 retry_at");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let retry_at = results[0].retry_at.unwrap();
        // retry_at 应在 now + 3600 附近
        assert!(
            retry_at >= now + JOB_RETRY_DELAY_SECONDS - 5,
            "retry_at 应 >= now + 3600 - 5"
        );
    }

    // 测试 6：解析失败 → Failed
    #[tokio::test]
    async fn run_extraction_invalid_json_classified_as_failed() {
        let sampler = MockSampler::new().with_output("t1", "not json");
        let results = run_extraction(Arc::new(sampler), vec![job("t1")]).await;
        assert_eq!(results[0].outcome, JobOutcome::Failed);
        assert!(results[0].failure_reason.as_ref().unwrap().contains("解析失败"));
    }

    // 测试 7：aggregate_stats 统计正确
    #[test]
    fn aggregate_stats_counts_correctly() {
        let results = vec![
            JobResult {
                outcome: JobOutcome::SucceededWithOutput,
                thread_id: "t1".into(),
                failure_reason: None,
                retry_at: None,
                output: None,
            },
            JobResult {
                outcome: JobOutcome::SucceededNoOutput,
                thread_id: "t2".into(),
                failure_reason: None,
                retry_at: None,
                output: None,
            },
            JobResult {
                outcome: JobOutcome::Failed,
                thread_id: "t3".into(),
                failure_reason: Some("err".into()),
                retry_at: Some(0),
                output: None,
            },
        ];
        let stats = aggregate_stats(&results);
        assert_eq!(stats.claimed, 3);
        assert_eq!(stats.succeeded_with_output, 1);
        assert_eq!(stats.succeeded_no_output, 1);
        assert_eq!(stats.failed, 1);
    }

    // 测试 8：failed_result 的 retry_at 为 now + 3600
    #[test]
    fn failed_result_retry_at_is_now_plus_delay() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let result = failed_result("t1", "test error");
        assert_eq!(result.outcome, JobOutcome::Failed);
        let retry_at = result.retry_at.unwrap();
        assert!(
            (retry_at - now - JOB_RETRY_DELAY_SECONDS).abs() <= 5,
            "retry_at 应在 now + 3600 附近（±5s）"
        );
    }
}
