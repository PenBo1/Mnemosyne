use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::errors::WorkspaceError;

pub struct WorkspaceRegistry {
    roots: Mutex<HashSet<PathBuf>>,
}

impl WorkspaceRegistry {
    pub fn new() -> Self {
        Self {
            roots: Mutex::new(HashSet::new()),
        }
    }

    pub fn authorize<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, WorkspaceError> {
        let canonical = std::fs::canonicalize(path.as_ref())
            .map_err(WorkspaceError::Io)?;
        self.roots.lock().unwrap().insert(canonical.clone());
        Ok(canonical)
    }

    pub fn is_authorized(&self, target: &Path) -> bool {
        let roots = self.roots.lock().unwrap();
        roots.iter().any(|root| target.starts_with(root))
    }

    pub fn clear(&self) {
        self.roots.lock().unwrap().clear();
    }

    pub fn authorized_roots(&self) -> Vec<PathBuf> {
        self.roots.lock().unwrap().iter().cloned().collect()
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}