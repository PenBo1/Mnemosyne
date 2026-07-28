//! ═══════════════════════════════════════════════════════════════════════════
//! filesystem_scope - 文件系统范围定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fmt;

// ── 文件系统范围 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FsScope {
    Workspace,
    WorkspaceReadonly,
    AppData,
    Cache,
    Temp,
    Resources,
    Templates,
    Plugins,
    Logs,
}

impl fmt::Display for FsScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace => write!(f, "workspace"),
            Self::WorkspaceReadonly => write!(f, "workspace_readonly"),
            Self::AppData => write!(f, "app_data"),
            Self::Cache => write!(f, "cache"),
            Self::Temp => write!(f, "temp"),
            Self::Resources => write!(f, "resources"),
            Self::Templates => write!(f, "templates"),
            Self::Plugins => write!(f, "plugins"),
            Self::Logs => write!(f, "logs"),
        }
    }
}

impl FsScope {
    pub fn is_writable(&self) -> bool {
        match self {
            Self::Workspace | Self::AppData | Self::Cache | Self::Temp | Self::Plugins | Self::Logs => true,
            Self::WorkspaceReadonly | Self::Resources | Self::Templates => false,
        }
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "workspace" => Some(Self::Workspace),
            "workspace_readonly" => Some(Self::WorkspaceReadonly),
            "app_data" => Some(Self::AppData),
            "cache" => Some(Self::Cache),
            "temp" => Some(Self::Temp),
            "resources" => Some(Self::Resources),
            "templates" => Some(Self::Templates),
            "plugins" => Some(Self::Plugins),
            "logs" => Some(Self::Logs),
            _ => None,
        }
    }
}

// ── 文件系统操作 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FsOperation {
    Read,
    Write,
    Delete,
    List,
    CreateDir,
}

impl fmt::Display for FsOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Delete => write!(f, "delete"),
            Self::List => write!(f, "list"),
            Self::CreateDir => write!(f, "create_dir"),
        }
    }
}

impl FsOperation {
    pub fn is_write_operation(&self) -> bool {
        matches!(self, Self::Write | Self::Delete | Self::CreateDir)
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Delete),
            "list" => Some(Self::List),
            "create_dir" => Some(Self::CreateDir),
            _ => None,
        }
    }
}