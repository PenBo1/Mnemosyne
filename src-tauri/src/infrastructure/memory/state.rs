
use std::sync::Arc;
use super::store::MemoryStore;
use std::path::PathBuf;

#[derive(Clone)]
pub struct MemoryState {
    pub store: Arc<MemoryStore>,
}

impl MemoryState {
    pub fn new(data_dir: PathBuf) -> Self {
        let store = MemoryStore::new(data_dir);
        Self { store }
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            store: MemoryStore::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        }
    }
}