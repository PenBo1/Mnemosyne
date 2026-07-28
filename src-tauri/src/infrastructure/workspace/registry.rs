//! ═══════════════════════════════════════════════════════════════════════════
//! 工作区注册表 - 授权路径管理
//! ═══════════════════════════════════════════════════════════════════════════

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
        self.roots.lock().unwrap_or_else(|e| e.into_inner()).insert(canonical.clone());
        Ok(canonical)
    }

    pub fn is_authorized(&self, target: &Path) -> bool {
        // 规范化 target 再比对,否则符号链接 / 大小写差异 / .. 会导致 starts_with 误判。
        // 路径可能尚不存在(写入前校验),按 L20 同款策略:父目录存在则规范化父目录后拼接文件名。
        let canonical = match std::fs::canonicalize(target) {
            Ok(p) => p,
            Err(_) => {
                // 路径不存在:尝试规范化父目录后拼接文件名
                match target.parent() {
                    Some(parent) if parent.exists() => match std::fs::canonicalize(parent) {
                        Ok(canonical_parent) => match target.file_name() {
                            Some(name) => canonical_parent.join(name),
                            None => return false,
                        },
                        Err(_) => return false,
                    },
                    _ => return false,
                }
            }
        };
        let roots = self.roots.lock().unwrap_or_else(|e| e.into_inner());
        roots.iter().any(|root| canonical.starts_with(root))
    }

    pub fn clear(&self) {
        self.roots.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn authorized_roots(&self) -> Vec<PathBuf> {
        self.roots.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}