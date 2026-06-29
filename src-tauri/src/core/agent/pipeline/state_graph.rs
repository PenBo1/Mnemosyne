use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::shared::errors::AppError;

/// State graph node function type
pub type NodeFn = Box<dyn Fn(&GraphState) -> Result<GraphUpdate, AppError> + Send + Sync>;

/// State graph edge - conditional routing
pub struct Edge {
    pub condition: Box<dyn Fn(&GraphState) -> bool + Send + Sync>,
    pub target: String,
}

/// A LangGraph-style state graph for pipeline orchestration
pub struct StateGraph {
    nodes: HashMap<String, NodeFn>,
    edges: HashMap<String, Vec<Edge>>,
    entry_point: String,
}

/// State update returned by a node
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphUpdate {
    pub values: HashMap<String, serde_json::Value>,
    pub next_node: Option<String>,
}

/// Serializable graph state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphState {
    pub values: HashMap<String, serde_json::Value>,
    pub current_node: Option<String>,
    pub history: Vec<String>,
    pub checkpoint_id: Option<String>,
}

impl GraphState {
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.values.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.values.get(key).and_then(|v| v.as_i64())
    }

    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.values.insert(key.to_string(), value);
    }
}

impl StateGraph {
    pub fn new(entry_point: impl Into<String>) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            entry_point: entry_point.into(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, name: impl Into<String>, node_fn: NodeFn) {
        self.nodes.insert(name.into(), node_fn);
    }

    /// Add an unconditional edge
    pub fn add_edge(&mut self, source: impl Into<String>, target: impl Into<String>) {
        let source = source.into();
        let target = target.into();
        self.edges.entry(source).or_default().push(Edge {
            condition: Box::new(|_| true),
            target,
        });
    }

    /// Add a conditional edge
    pub fn add_conditional_edge(
        &mut self,
        source: impl Into<String>,
        condition: Box<dyn Fn(&GraphState) -> bool + Send + Sync>,
        target: impl Into<String>,
    ) {
        self.edges.entry(source.into()).or_default().push(Edge {
            condition,
            target: target.into(),
        });
    }

    /// Get the next node based on edges
    fn next_node(&self, current: &str, state: &GraphState) -> Option<String> {
        if let Some(edges) = self.edges.get(current) {
            for edge in edges {
                if (edge.condition)(state) {
                    return Some(edge.target.clone());
                }
            }
        }
        None
    }

    /// Serialize state to JSON for checkpointing
    pub fn checkpoint(&self, state: &GraphState) -> Result<String, AppError> {
        serde_json::to_string_pretty(state)
            .map_err(|e| AppError::internal(format!("Failed to serialize checkpoint: {}", e)))
    }

    /// Deserialize state from a checkpoint
    pub fn restore_checkpoint(&self, json: &str) -> Result<GraphState, AppError> {
        serde_json::from_str(json)
            .map_err(|e| AppError::internal(format!("Failed to restore checkpoint: {}", e)))
    }
}

/// Graph runner that executes nodes and manages state
pub struct GraphRunner {
    graph: StateGraph,
    max_steps: usize,
}

impl GraphRunner {
    pub fn new(graph: StateGraph, max_steps: usize) -> Self {
        Self { graph, max_steps }
    }

    /// Run the graph from initial state
    pub async fn run(&self, initial_state: GraphState) -> Result<GraphState, AppError> {
        let mut state = initial_state;
        state.current_node = Some(self.graph.entry_point.clone());

        for _step in 0..self.max_steps {
            let current = match &state.current_node {
                Some(node) => node.clone(),
                None => return Ok(state),
            };

            // Get node function
            let node_fn = self.graph.nodes.get(&current)
                .ok_or_else(|| AppError::internal(format!("Node not found: {}", current)))?;

            // Execute node
            tracing::info!(node = %current, step = state.history.len(), "Executing graph node");
            let update = (node_fn)(&state)?;

            // Apply update
            for (key, value) in update.values {
                state.values.insert(key, value);
            }
            state.history.push(current);

            // Determine next node
            state.current_node = update.next_node.or_else(|| self.graph.next_node(&state.history.last().unwrap(), &state));
        }

        tracing::warn!(steps = state.history.len(), "Graph execution exceeded max steps");
        Ok(state)
    }

    /// Run from a checkpoint
    pub async fn run_from_checkpoint(&self, checkpoint_json: &str) -> Result<GraphState, AppError> {
        let state = self.graph.restore_checkpoint(checkpoint_json)?;
        self.run(state).await
    }
}

/// Persistence backend for graph checkpoints
pub struct CheckpointStore {
    checkpoints: HashMap<String, String>, // id -> json
}

impl CheckpointStore {
    pub fn new() -> Self {
        Self { checkpoints: HashMap::new() }
    }

    pub fn save(&mut self, id: &str, state: &GraphState) -> Result<(), AppError> {
        let json = serde_json::to_string(state)
            .map_err(|e| AppError::internal(format!("Failed to serialize checkpoint: {}", e)))?;
        self.checkpoints.insert(id.to_string(), json);
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Option<GraphState>, AppError> {
        match self.checkpoints.get(id) {
            Some(json) => {
                let state: GraphState = serde_json::from_str(json)
                    .map_err(|e| AppError::internal(format!("Failed to deserialize checkpoint: {}", e)))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    pub fn list(&self) -> Vec<String> {
        self.checkpoints.keys().cloned().collect()
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.checkpoints.remove(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_state_serialization() {
        let mut state = GraphState::default();
        state.set("step", serde_json::json!("plan"));
        state.set("chapter", serde_json::json!(1));

        let json = serde_json::to_string(&state).unwrap();
        let restored: GraphState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.get_string("step"), Some("plan".into()));
        assert_eq!(restored.get_i64("chapter"), Some(1));
    }

    #[test]
    fn test_checkpoint_store() {
        let mut store = CheckpointStore::new();
        let state = GraphState::default();
        store.save("cp1", &state).unwrap();
        assert!(store.load("cp1").unwrap().is_some());
        assert!(store.delete("cp1"));
        assert!(store.load("cp1").unwrap().is_none());
    }
}
