//! ═══════════════════════════════════════════════════════════════════════════
//! TokenCounter - Token 计数器
//! ═══════════════════════════════════════════════════════════════════════════

use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use super::types::SubAgentRole;

#[derive(Debug, Clone, Copy)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }
}

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub role: SubAgentRole,
    pub usage: TokenUsage,
    pub timestamp: DateTime<Utc>,
    pub task: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RoleStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_calls: u64,
    pub avg_duration_ms: f64,
}

impl Default for RoleStats {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleStats {
    pub fn new() -> Self {
        Self {
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_calls: 0,
            avg_duration_ms: 0.0,
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }
}

pub struct TokenCounter {
    // 用 VecDeque 让头部淘汰为 O(1)；Vec::remove(0) 是 O(n) 会随记录数线性退化。
    records: Arc<RwLock<VecDeque<UsageRecord>>>,
    max_records: usize,
}

impl TokenCounter {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(VecDeque::new())),
            max_records,
        }
    }

    pub async fn record(
        &self,
        role: SubAgentRole,
        task: String,
        input_tokens: u32,
        output_tokens: u32,
        duration_ms: u64,
    ) {
        let start = std::time::Instant::now();
        tracing::info!(
            role = ?role,
            input_tokens,
            output_tokens,
            task_len = task.len(),
            "token_record: enter"
        );
        
        let record = UsageRecord {
            role,
            usage: TokenUsage::new(input_tokens, output_tokens),
            timestamp: Utc::now(),
            task,
            duration_ms,
        };

        let mut records = self.records.write().await;

        if records.len() >= self.max_records {
            records.pop_front();
        }

        records.push_back(record);
        
        tracing::info!(
            role = ?role,
            total_tokens = input_tokens + output_tokens,
            records = records.len(),
            duration_ms = start.elapsed().as_millis(),
            "token_record: exit"
        );
    }

    pub async fn total_usage(&self) -> TokenUsage {
        let records = self.records.read().await;
        let input: u32 = records.iter().map(|r| r.usage.input_tokens).sum();
        let output: u32 = records.iter().map(|r| r.usage.output_tokens).sum();
        TokenUsage::new(input, output)
    }

    pub async fn usage_by_role(&self, role: SubAgentRole) -> TokenUsage {
        let records = self.records.read().await;
        let filtered: Vec<_> = records.iter().filter(|r| r.role == role).collect();

        let input: u32 = filtered.iter().map(|r| r.usage.input_tokens).sum();
        let output: u32 = filtered.iter().map(|r| r.usage.output_tokens).sum();

        TokenUsage::new(input, output)
    }

    pub async fn stats_by_role(&self) -> HashMap<SubAgentRole, RoleStats> {
        let records = self.records.read().await;
        let mut stats_map: HashMap<SubAgentRole, RoleStats> = HashMap::new();

        for role in [SubAgentRole::Researcher, SubAgentRole::Outliner, SubAgentRole::Critic] {
            stats_map.insert(role, RoleStats::new());
        }

        let mut durations: HashMap<SubAgentRole, (u64, u64)> = HashMap::new();

        for record in records.iter() {
            // 用 entry().or_default() 兜底未知 role（如未来新增 Default/Custom 等），
            // 避免 stats_map 只预填三种 role 时其它 role 触发 panic。
            let stats = stats_map.entry(record.role).or_default();
            stats.total_input_tokens += record.usage.input_tokens as u64;
            stats.total_output_tokens += record.usage.output_tokens as u64;
            stats.total_calls += 1;

            let entry = durations.entry(record.role).or_insert((0, 0));
            entry.0 += record.duration_ms;
            entry.1 += 1;
        }

        for (role, (total_duration, count)) in durations.iter() {
            if let Some(stats) = stats_map.get_mut(role) {
                if *count > 0 {
                    stats.avg_duration_ms = *total_duration as f64 / *count as f64;
                }
            }
        }

        stats_map
    }

    pub async fn recent_records(&self, limit: usize) -> Vec<UsageRecord> {
        let records = self.records.read().await;
        records.iter().rev().take(limit).cloned().collect()
    }

    pub async fn clear(&self) {
        let mut records = self.records.write().await;
        records.clear();
    }

    pub async fn len(&self) -> usize {
        self.records.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new(1000)
    }
}

pub struct ExecutionTimer {
    start: Instant,
}

impl ExecutionTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_counter_record() {
        let counter = TokenCounter::new(100);

        counter.record(SubAgentRole::Researcher, "task1".to_string(), 100, 200, 150).await;

        assert_eq!(counter.len().await, 1);

        let usage = counter.total_usage().await;
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
    }

    #[tokio::test]
    async fn test_usage_by_role() {
        let counter = TokenCounter::new(100);

        counter.record(SubAgentRole::Researcher, "r1".to_string(), 100, 200, 100).await;
        counter.record(SubAgentRole::Critic, "c1".to_string(), 50, 75, 80).await;

        let researcher_usage = counter.usage_by_role(SubAgentRole::Researcher).await;
        assert_eq!(researcher_usage.total_tokens, 300);

        let critic_usage = counter.usage_by_role(SubAgentRole::Critic).await;
        assert_eq!(critic_usage.total_tokens, 125);
    }
}