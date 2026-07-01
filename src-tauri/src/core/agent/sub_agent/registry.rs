//! 子 Agent 线程注册表。
//!
//! 学习 codex 的 `ThreadRegistry`：用 `RwLock<HashMap>` 管理所有子 Agent 元信息。
//! 注册表是查询子 Agent 状态的唯一来源，由 `SubAgentControl` 持有。

use std::collections::HashMap;
use tokio::sync::RwLock;

use super::types::{SubAgentInfo, SubAgentSpawnRequest, SubAgentStatus};

/// 子 Agent spawn 树最大深度（防递归爆炸）。
///
/// 主 Agent 直接 spawn 的子 Agent depth=1，depth=4 会被拒绝。
/// 当前子 Agent 不携带 spawn_subagent 工具，此限制作为安全网存在。
pub const MAX_DEPTH: u32 = 3;

/// 并发活跃子 Agent 数量上限（Thalia 比 codex 默认 6 更保守）。
pub const MAX_CONCURRENT: usize = 4;

/// 线程注册表 — 管理所有子 Agent 的元信息。
///
/// 线程安全：内部用 `RwLock<HashMap>` 保护，多读单写。
/// 所有方法都是 `async` 以配合 `RwLock`。
pub struct ThreadRegistry {
    entries: RwLock<HashMap<String, SubAgentInfo>>,
}

impl ThreadRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// 注册一个新子 Agent，返回生成的 task_id。
    ///
    /// `depth` 由调用方（SubAgentControl）计算后传入。
    pub async fn register(
        &self,
        req: SubAgentSpawnRequest,
        depth: u32,
    ) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        let info = SubAgentInfo {
            task_id: task_id.clone(),
            role: req.role,
            task: req.task,
            status: SubAgentStatus::Pending,
            parent_thread_id: req.parent_thread_id,
            depth,
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        self.entries.write().await.insert(task_id.clone(), info);
        task_id
    }

    pub async fn get(&self, task_id: &str) -> Option<SubAgentInfo> {
        self.entries.read().await.get(task_id).cloned()
    }

    pub async fn update_status(&self, task_id: &str, status: SubAgentStatus) {
        if let Some(info) = self.entries.write().await.get_mut(task_id) {
            info.status = status;
        }
    }

    pub async fn list_by_parent(&self, parent_thread_id: &str) -> Vec<SubAgentInfo> {
        self.entries
            .read()
            .await
            .values()
            .filter(|info| info.parent_thread_id == parent_thread_id)
            .cloned()
            .collect()
    }

    pub async fn remove(&self, task_id: &str) {
        self.entries.write().await.remove(task_id);
    }

    /// 统计当前活跃（Pending/Running）的子 Agent 数量。
    pub async fn count_active(&self) -> usize {
        self.entries
            .read()
            .await
            .values()
            .filter(|info| info.status.is_active())
            .count()
    }

    /// 查询指定 parent_thread_id 对应的深度。
    ///
    /// 如果 parent 在注册表中（即 parent 是子 Agent），返回其 depth。
    /// 如果不在（即 parent 是主 Agent），返回 None（depth 由调用方默认为 0）。
    pub async fn parent_depth(&self, parent_thread_id: &str) -> Option<u32> {
        self.entries
            .read()
            .await
            .get(parent_thread_id)
            .map(|info| info.depth)
    }
}

impl Default for ThreadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(parent: &str) -> SubAgentSpawnRequest {
        SubAgentSpawnRequest {
            role: super::super::types::SubAgentRole::Researcher,
            task: "test task".to_string(),
            parent_thread_id: parent.to_string(),
            context: None,
        }
    }

    #[tokio::test]
    async fn register_and_get() {
        let registry = ThreadRegistry::new();
        let task_id = registry.register(make_request("session-1"), 1).await;

        let info = registry.get(&task_id).await;
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.task, "test task");
        assert_eq!(info.status, SubAgentStatus::Pending);
        assert_eq!(info.depth, 1);
    }

    #[tokio::test]
    async fn update_status() {
        let registry = ThreadRegistry::new();
        let task_id = registry.register(make_request("session-1"), 1).await;

        registry.update_status(&task_id, SubAgentStatus::Running).await;
        assert_eq!(
            registry.get(&task_id).await.unwrap().status,
            SubAgentStatus::Running
        );

        registry.update_status(&task_id, SubAgentStatus::Completed).await;
        assert_eq!(
            registry.get(&task_id).await.unwrap().status,
            SubAgentStatus::Completed
        );
    }

    #[tokio::test]
    async fn list_by_parent() {
        let registry = ThreadRegistry::new();
        registry.register(make_request("session-1"), 1).await;
        registry.register(make_request("session-1"), 1).await;
        registry.register(make_request("session-2"), 1).await;

        let children = registry.list_by_parent("session-1").await;
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|c| c.parent_thread_id == "session-1"));
    }

    #[tokio::test]
    async fn count_active() {
        let registry = ThreadRegistry::new();
        let t1 = registry.register(make_request("s1"), 1).await;
        let t2 = registry.register(make_request("s1"), 1).await;
        registry.register(make_request("s1"), 1).await;

        assert_eq!(registry.count_active().await, 3);

        registry.update_status(&t1, SubAgentStatus::Completed).await;
        assert_eq!(registry.count_active().await, 2);

        registry.update_status(&t2, SubAgentStatus::Cancelled).await;
        assert_eq!(registry.count_active().await, 1);
    }

    #[tokio::test]
    async fn parent_depth() {
        let registry = ThreadRegistry::new();
        let parent_id = registry.register(make_request("main"), 1).await;

        assert_eq!(registry.parent_depth(&parent_id).await, Some(1));
        assert_eq!(registry.parent_depth("main").await, None);
    }

    #[tokio::test]
    async fn remove() {
        let registry = ThreadRegistry::new();
        let task_id = registry.register(make_request("s1"), 1).await;

        assert!(registry.get(&task_id).await.is_some());
        registry.remove(&task_id).await;
        assert!(registry.get(&task_id).await.is_none());
    }
}
