//! ═══════════════════════════════════════════════════════════════════════════
//! capability - 权限能力定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{FsOperation, FsScope, NetworkScope, ShellScope};

// ── 权限能力枚举 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    Filesystem(FsScope, FsOperation),
    Shell(ShellScope),
    Network(NetworkScope),
}

impl Capability {
    pub fn filesystem_read(scope: FsScope) -> Self {
        Self::Filesystem(scope, FsOperation::Read)
    }

    pub fn filesystem_write(scope: FsScope) -> Self {
        Self::Filesystem(scope, FsOperation::Write)
    }

    pub fn filesystem_delete(scope: FsScope) -> Self {
        Self::Filesystem(scope, FsOperation::Delete)
    }

    pub fn filesystem_list(scope: FsScope) -> Self {
        Self::Filesystem(scope, FsOperation::List)
    }

    pub fn filesystem_create_dir(scope: FsScope) -> Self {
        Self::Filesystem(scope, FsOperation::CreateDir)
    }

    pub fn is_filesystem(&self) -> bool {
        matches!(self, Self::Filesystem(_, _))
    }

    pub fn is_shell(&self) -> bool {
        matches!(self, Self::Shell(_))
    }

    pub fn is_network(&self) -> bool {
        matches!(self, Self::Network(_))
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Filesystem(scope, op) => write!(f, "fs:{}:{}", scope, op),
            Self::Shell(scope) => write!(f, "shell:{}", scope),
            Self::Network(scope) => write!(f, "network:{}", scope),
        }
    }
}