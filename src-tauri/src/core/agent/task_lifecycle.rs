use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub summary: String,
    pub status: TaskStatus,
    pub assigned_agent: Option<String>,
    pub parent_id: Option<String>,
    pub subtasks: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub blocked_reason: Option<String>,
    pub result: Option<String>,
}

pub struct TaskManager {
    tasks: HashMap<String, AgentTask>,
    next_id: u32,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn create(&mut self, summary: String, parent_id: Option<String>) -> String {
        let id = format!("T{}", self.next_id);
        self.next_id += 1;
        let now = Utc::now().to_rfc3339();
        let task = AgentTask {
            id: id.clone(),
            summary,
            status: TaskStatus::Open,
            assigned_agent: None,
            parent_id: parent_id.clone(),
            subtasks: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            blocked_reason: None,
            result: None,
        };
        if let Some(ref pid) = parent_id {
            if let Some(parent) = self.tasks.get_mut(pid) {
                parent.subtasks.push(id.clone());
            }
        }
        self.tasks.insert(id.clone(), task);
        id
    }

    pub fn start(&mut self, task_id: &str, agent: &str) -> Result<(), String> {
        let task = self.tasks.get_mut(task_id).ok_or_else(|| format!("Task {task_id} not found"))?;
        if task.status != TaskStatus::Open {
            return Err(format!("Task {task_id} is {:?}, not Open", task.status));
        }
        task.status = TaskStatus::InProgress;
        task.assigned_agent = Some(agent.to_string());
        task.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn block(&mut self, task_id: &str, reason: &str) -> Result<(), String> {
        let task = self.tasks.get_mut(task_id).ok_or_else(|| format!("Task {task_id} not found"))?;
        if task.status != TaskStatus::InProgress && task.status != TaskStatus::Open {
            return Err(format!("Task {task_id} cannot be blocked from {:?}", task.status));
        }
        task.status = TaskStatus::Blocked;
        task.blocked_reason = Some(reason.to_string());
        task.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn complete(&mut self, task_id: &str, result: Option<String>) -> Result<(), String> {
        let task = self.tasks.get_mut(task_id).ok_or_else(|| format!("Task {task_id} not found"))?;
        if task.status != TaskStatus::InProgress {
            return Err(format!("Task {task_id} is {:?}, not InProgress", task.status));
        }
        task.status = TaskStatus::Done;
        task.result = result;
        task.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn abandon(&mut self, task_id: &str, reason: &str) -> Result<(), String> {
        let task = self.tasks.get_mut(task_id).ok_or_else(|| format!("Task {task_id} not found"))?;
        if task.status == TaskStatus::Done || task.status == TaskStatus::Abandoned {
            return Err(format!("Task {task_id} is already {:?}, cannot abandon", task.status));
        }
        task.status = TaskStatus::Abandoned;
        task.blocked_reason = Some(reason.to_string());
        task.updated_at = Utc::now().to_rfc3339();
        Ok(())
    }

    pub fn get(&self, task_id: &str) -> Option<&AgentTask> {
        self.tasks.get(task_id)
    }

    pub fn open_tasks(&self) -> Vec<&AgentTask> {
        self.tasks.values().filter(|t| t.status == TaskStatus::Open).collect()
    }

    pub fn in_progress_tasks(&self) -> Vec<&AgentTask> {
        self.tasks.values().filter(|t| t.status == TaskStatus::InProgress).collect()
    }
}
