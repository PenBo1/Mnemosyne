use super::registry::WorkspaceRegistry;

#[derive(Clone)]
pub struct WorkspaceState {
    pub registry: std::sync::Arc<WorkspaceRegistry>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            registry: std::sync::Arc::new(WorkspaceRegistry::new()),
        }
    }
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self::new()
    }
}