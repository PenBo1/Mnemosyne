//! ═══════════════════════════════════════════════════════════════════════════
//! Phase2 - Global consolidation 阶段
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;

use tokio::sync::Mutex as TokioMutex;

use crate::shared::error::AppError;

use super::artifact::{
    self, rebuild_raw_memories_file, remove_workspace_diff, sync_rollout_summaries,
    write_workspace_diff, MemoryArtifact,
};

// ── 常量定义 ────────────────────────────────────────────────────────────────

/// 失败 job 重试延迟（秒）
pub const JOB_RETRY_DELAY_SECONDS: i64 = 3_600;

/// consolidation agent 心跳间隔（秒）
pub const JOB_HEARTBEAT_SECONDS: u64 = 90;

// ── 全局锁 ──────────────────────────────────────────────────────────────────

/// 全局 consolidation 锁
///
/// 进程内单实例锁，确保同时只有一个 consolidation 运行。
pub struct Phase2Lock {
    inner: TokioMutex<()>,
}

impl Phase2Lock {
    pub fn new() -> Self {
        Self {
            inner: TokioMutex::new(()),
        }
    }

    /// 尝试获取锁，成功返回 guard，失败返回 None（已有 consolidation 运行中）
    pub async fn try_acquire(&self) -> Option<Phase2LockGuard<'_>> {
        match self.inner.try_lock() {
            Ok(guard) => Some(Phase2LockGuard { _guard: guard }),
            Err(_) => None,
        }
    }
}

impl Default for Phase2Lock {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局锁的 guard，drop 时自动释放
pub struct Phase2LockGuard<'a> {
    _guard: tokio::sync::MutexGuard<'a, ()>,
}

// ── 配置与策略 ─────────────────────────────────────────────────────────────

/// consolidation agent 的锁定策略配置
///
/// 描述 consolidation sub-agent 的安全约束：
/// - 无审批（AskForApproval::Never）
/// - 无网络访问
/// - 仅本地 memory_root 写权限
/// - 禁用 collab / spawn / memory tool / apps / plugins
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// consolidation 的工作目录（memory_root）
    pub cwd: std::path::PathBuf,
    /// 可写目录列表（仅 memory_root）
    pub writable_roots: Vec<std::path::PathBuf>,
    /// 是否允许网络访问（始终 false）
    pub network_access: bool,
    /// 是否允许审批（始终 Never）
    pub approval_policy: ApprovalPolicy,
    /// 禁用的 feature 列表
    pub disabled_features: Vec<ConsolidationFeature>,
}

/// consolidation agent 的审批策略
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ApprovalPolicy {
    /// 从不请求审批（consolidation 始终使用此策略）
    Never,
}

/// consolidation agent 禁用的 feature
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConsolidationFeature {
    /// 禁止递归派生子 agent
    SpawnSubAgent,
    /// 禁止协作模式
    Collab,
    /// 禁止记忆工具（防止 consolidation 反馈到 phase-1）
    MemoryTool,
    /// 禁止 apps
    Apps,
    /// 禁止 plugins
    Plugins,
}

impl ConsolidationConfig {
    /// 构造默认的 consolidation 配置（锁定策略）
    ///
    /// - cwd = memory_root
    /// - writable_roots = [memory_root]
    /// - network_access = false
    /// - approval_policy = Never
    /// - disabled_features = [SpawnSubAgent, Collab, MemoryTool, Apps, Plugins]
    pub fn locked_down(memory_root: &Path) -> Self {
        Self {
            cwd: memory_root.to_path_buf(),
            writable_roots: vec![memory_root.to_path_buf()],
            network_access: false,
            approval_policy: ApprovalPolicy::Never,
            disabled_features: vec![
                ConsolidationFeature::SpawnSubAgent,
                ConsolidationFeature::Collab,
                ConsolidationFeature::MemoryTool,
                ConsolidationFeature::Apps,
                ConsolidationFeature::Plugins,
            ],
        }
    }
}

// ── Consolidation Agent 抽象 ────────────────────────────────────────────────

/// 抽象 consolidation sub-agent 执行器
///
/// 调用方实现以注入具体的 sub-agent spawn 机制。
#[async_trait::async_trait]
pub trait ConsolidationAgent: Send + Sync {
    /// 执行 consolidation，返回 agent 的输出文本
    async fn run(&self, config: &ConsolidationConfig, prompt: &str) -> Result<String, AppError>;
}

/// Phase 2 consolidation 的执行结果
#[derive(Debug, Clone)]
pub struct ConsolidationResult {
    /// 是否实际执行了 consolidation（无变更时跳过）
    pub consolidated: bool,
    /// 输入的 raw memory 数量
    pub input_count: usize,
    /// consolidation agent 的输出（若有）
    pub agent_output: Option<String>,
}

// ── Prompt 构建 ─────────────────────────────────────────────────────────────

/// 构建 consolidation prompt
///
/// prompt 指示 agent 读取 `phase2_workspace_diff.md` 并据此编辑 memory 文件。
pub fn build_consolidation_prompt(memory_root: &Path) -> String {
    let diff_path = artifact::workspace_diff_file(memory_root);
    let raw_path = artifact::raw_memories_file(memory_root);
    let summaries_dir = artifact::rollout_summaries_dir(memory_root);

    format!(
        r#"你是记忆整合 agent（consolidation agent）。你的任务是阅读 memory workspace 的变更，并整合 raw memories。

## 工作目录

你的工作目录是 memory root：`{root}`
你只能在此目录内读写文件。

## 输入

1. **Workspace diff**：`{diff}`
   - 这是本次整合的增量变更（git-style diff），请先阅读此文件了解发生了什么变化。
2. **Raw memories**：`{raw}`
   - 当前合并后的原始记忆列表。
3. **Rollout summaries**：`{summaries}/`
   - 每个 rollout 的摘要文件（按 thread_id 命名）。

## 任务

1. 阅读 `phase2_workspace_diff.md`，理解新增/修改/删除的记忆内容。
2. 阅读 `raw_memories.md`，理解当前记忆状态。
3. 整合记忆：
   - 合并重复或相似的记忆条目。
   - 移除过时或被后续记忆否定的条目。
   - 保持记忆的时序一致性。
   - 不要编造未在输入中出现的记忆。
4. 将整合后的记忆写回 `raw_memories.md`（覆盖旧内容）。

## 约束

- **无审批**：你不需要请求任何审批，直接执行。
- **无网络**：你不能访问网络，仅能操作本地文件。
- **仅本地写**：你只能在 memory root 目录内写文件。
- **禁用协作**：你不能派生子 agent，不能使用协作模式。
- **禁用记忆工具**：你不能触发记忆工具（防止反馈到 phase-1）。

## 输出

完成后，输出一段简短的整合摘要（变更了什么、为什么）。"#,
        root = memory_root.display(),
        diff = diff_path.display(),
        raw = raw_path.display(),
        summaries = summaries_dir.display(),
    )
}

// ── Consolidation 执行 ─────────────────────────────────────────────────────

/// 执行 Phase 2 consolidation 的完整流程
///
/// 步骤：
/// 1. 获取全局锁（失败则跳过，说明已有 consolidation 运行）
/// 2. 同步 artifact（raw_memories.md / rollout_summaries/）
/// 3. 写入 workspace diff
/// 4. 若 diff 为空，跳过 consolidation
/// 5. spawn consolidation agent
/// 6. 返回结果
///
/// `diff` 由调用方提供（git baseline diff 字符串）；`max_raw_memories` 限制处理的记忆数。
pub async fn run_consolidation(
    lock: &Phase2Lock,
    agent: &dyn ConsolidationAgent,
    memory_root: &Path,
    memories: &[MemoryArtifact],
    diff: &str,
    max_raw_memories: usize,
) -> Result<ConsolidationResult, AppError> {
    // 1. 获取全局锁
    let _guard = lock.try_acquire().await.ok_or_else(|| {
        AppError::conflict("Phase 2 consolidation already in progress")
    })?;

    // 2. 同步 artifact
    sync_rollout_summaries(memory_root, memories, max_raw_memories)
        .map_err(|e| AppError::internal(format!("sync rollout summaries failed: {}", e)))?;
    rebuild_raw_memories_file(memory_root, memories, max_raw_memories)
        .map_err(|e| AppError::internal(format!("rebuild raw memories failed: {}", e)))?;

    // 3. 写入 workspace diff
    remove_workspace_diff(memory_root)
        .map_err(|e| AppError::internal(format!("remove workspace diff failed: {}", e)))?;

    if diff.trim().is_empty() {
        // 4. 无变更，跳过
        return Ok(ConsolidationResult {
            consolidated: false,
            input_count: memories.len(),
            agent_output: None,
        });
    }

    write_workspace_diff(memory_root, diff)
        .map_err(|e| AppError::internal(format!("write workspace diff failed: {}", e)))?;

    // 5. spawn consolidation agent
    let config = ConsolidationConfig::locked_down(memory_root);
    let prompt = build_consolidation_prompt(memory_root);
    let output = agent.run(&config, &prompt).await?;

    // 6. 清理 diff 文件（对齐 codex reset_memory_workspace_baseline 的一部分）
    let _ = remove_workspace_diff(memory_root);

    Ok(ConsolidationResult {
        consolidated: true,
        input_count: memories.len(),
        agent_output: Some(output),
    })
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    use super::super::phase1::StageOneOutput;

    /// Mock consolidation agent：记录调用次数，返回固定输出
    struct MockAgent {
        calls: AtomicU64,
        output: String,
    }

    impl MockAgent {
        fn new(output: &str) -> Self {
            Self {
                calls: AtomicU64::new(0),
                output: output.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ConsolidationAgent for MockAgent {
        async fn run(&self, _config: &ConsolidationConfig, _prompt: &str) -> Result<String, AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.output.clone())
        }
    }

    fn artifact(thread_id: &str, raw: &str, summary: &str) -> MemoryArtifact {
        MemoryArtifact {
            thread_id: thread_id.to_string(),
            stage_one: StageOneOutput {
                raw_memory: raw.to_string(),
                rollout_summary: summary.to_string(),
                rollout_slug: Some(format!("slug-{}", thread_id)),
            },
        }
    }

    // 测试 1：ConsolidationConfig::locked_down 策略正确
    #[test]
    fn locked_down_config_is_restrictive() {
        let root = Path::new("/tmp/memories");
        let config = ConsolidationConfig::locked_down(root);

        assert_eq!(config.cwd, root);
        assert_eq!(config.writable_roots, vec![root]);
        assert!(!config.network_access, "network_access 必须为 false");
        assert_eq!(config.approval_policy, ApprovalPolicy::Never);
        assert!(config
            .disabled_features
            .contains(&ConsolidationFeature::Collab));
        assert!(config
            .disabled_features
            .contains(&ConsolidationFeature::SpawnSubAgent));
        assert!(config
            .disabled_features
            .contains(&ConsolidationFeature::MemoryTool));
    }

    // 测试 2：build_consolidation_prompt 包含关键信息
    #[test]
    fn consolidation_prompt_contains_key_info() {
        let root = Path::new("/tmp/memories");
        let prompt = build_consolidation_prompt(root);

        assert!(prompt.contains("记忆整合 agent"));
        assert!(prompt.contains("phase2_workspace_diff.md"));
        assert!(prompt.contains("raw_memories.md"));
        assert!(prompt.contains("rollout_summaries"));
        assert!(prompt.contains("无审批"));
        assert!(prompt.contains("无网络"));
        assert!(prompt.contains("仅本地写"));
        assert!(prompt.contains("禁用协作"));
    }

    // 测试 3：全局锁串行化 —— 同时只有一个 consolidation 运行
    #[tokio::test]
    async fn phase2_lock_serializes_consolidation() {
        let lock = Arc::new(Phase2Lock::new());
        let lock1 = lock.clone();
        let lock2 = lock.clone();

        // 第一个获取成功
        let guard1 = lock1.try_acquire().await;
        assert!(guard1.is_some(), "第一次 acquire 应成功");

        // 第二个获取失败（已有 consolidation 运行）
        let guard2 = lock2.try_acquire().await;
        assert!(guard2.is_none(), "第二次 acquire 应失败（锁被占用）");

        // 释放第一个后，第二个可获取
        drop(guard1);
        let guard3 = lock2.try_acquire().await;
        assert!(guard3.is_some(), "释放后应能重新 acquire");
    }

    // 测试 4：run_consolidation 完整流程（有 diff）
    #[tokio::test]
    async fn run_consolidation_with_diff_spawns_agent() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let lock = Phase2Lock::new();
        let agent = MockAgent::new("consolidation done");
        let memories = vec![
            artifact("t1", "memory one", "summary one"),
            artifact("t2", "memory two", "summary two"),
        ];

        let result = run_consolidation(&lock, &agent, &root, &memories, "diff content", 100)
            .await
            .unwrap();

        assert!(result.consolidated, "应执行了 consolidation");
        assert_eq!(result.input_count, 2);
        assert_eq!(result.agent_output.as_deref(), Some("consolidation done"));

        // artifact 文件应已生成
        assert!(artifact::raw_memories_file(&root).exists());
        assert!(artifact::rollout_summaries_dir(&root).join("t1.md").exists());
        assert!(artifact::rollout_summaries_dir(&root).join("t2.md").exists());

        // diff 文件应在结束后被清理
        assert!(
            !artifact::workspace_diff_file(&root).exists(),
            "diff 文件应在 consolidation 后被清理"
        );
    }

    // 测试 5：run_consolidation 空 diff 跳过 agent
    #[tokio::test]
    async fn run_consolidation_empty_diff_skips_agent() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let lock = Phase2Lock::new();
        let agent = MockAgent::new("should not be called");
        let memories = vec![artifact("t1", "memory one", "summary one")];

        let result = run_consolidation(&lock, &agent, &root, &memories, "   ", 100)
            .await
            .unwrap();

        assert!(!result.consolidated, "空 diff 不应执行 consolidation");
        assert!(result.agent_output.is_none());
    }

    // 测试 6：run_consolidation 锁被占用时返回 conflict 错误
    #[tokio::test]
    async fn run_consolidation_returns_conflict_when_locked() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let lock = Arc::new(Phase2Lock::new());

        // 先占用锁
        let _guard = lock.try_acquire().await.unwrap();

        let agent = MockAgent::new("should not be called");
        let result = run_consolidation(&lock, &agent, &root, &[], "diff", 100).await;

        assert!(result.is_err(), "锁被占用时应返回错误");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("already in progress"),
            "错误消息应说明 consolidation 进行中，got: {}",
            err.message
        );
    }

    // 测试 7：max_raw_memories 限制处理的记忆数
    #[tokio::test]
    async fn run_consolidation_respects_max_raw_memories() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let lock = Phase2Lock::new();
        let agent = MockAgent::new("done");
        let memories = vec![
            artifact("t1", "m1", "s1"),
            artifact("t2", "m2", "s2"),
            artifact("t3", "m3", "s3"),
        ];

        let result = run_consolidation(&lock, &agent, &root, &memories, "diff", 2)
            .await
            .unwrap();

        assert_eq!(result.input_count, 3, "input_count 应为全部输入数");
        // 但 raw_memories.md 只应包含前 2 个
        let content = std::fs::read_to_string(artifact::raw_memories_file(&root)).unwrap();
        assert!(content.contains("## Thread `t1`"));
        assert!(content.contains("## Thread `t2`"));
        assert!(!content.contains("## Thread `t3`"));
    }
}
