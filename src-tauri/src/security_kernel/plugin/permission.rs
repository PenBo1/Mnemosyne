use serde::{Deserialize, Serialize};
use std::fmt;

use crate::security_kernel::permission::{
    FsScope, FsOperation, NetworkEndpoint,
    GitOperation, CargoOperation,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginPermission {
    Filesystem(FsPermission),
    Shell(ShellPermission),
    Network(NetworkPermission),
    Clipboard(ClipboardPermission),
    Notification(NotificationPermission),
}

impl PluginPermission {
    pub fn filesystem(scope: FsScope, operations: Vec<FsOperation>) -> Self {
        Self::Filesystem(FsPermission::new(scope, operations))
    }

    pub fn shell_git(operations: Vec<GitOperation>) -> Self {
        Self::Shell(ShellPermission::Git(operations))
    }

    pub fn shell_cargo(operations: Vec<CargoOperation>) -> Self {
        Self::Shell(ShellPermission::Cargo(operations))
    }

    pub fn shell_python(scripts: Vec<String>) -> Self {
        Self::Shell(ShellPermission::Python(scripts))
    }

    pub fn shell_node(scripts: Vec<String>) -> Self {
        Self::Shell(ShellPermission::Node(scripts))
    }

    pub fn network(hosts: Vec<String>, allow_localhost: bool) -> Self {
        Self::Network(NetworkPermission::from_hosts(hosts, allow_localhost))
    }

    pub fn clipboard(read: bool, write: bool) -> Self {
        Self::Clipboard(ClipboardPermission::new(read, write))
    }

    pub fn notification() -> Self {
        Self::Notification(NotificationPermission::new())
    }

    pub fn is_filesystem(&self) -> bool {
        matches!(self, Self::Filesystem(_))
    }

    pub fn is_shell(&self) -> bool {
        matches!(self, Self::Shell(_))
    }

    pub fn is_network(&self) -> bool {
        matches!(self, Self::Network(_))
    }

    pub fn is_clipboard(&self) -> bool {
        matches!(self, Self::Clipboard(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    pub fn is_write_operation(&self) -> bool {
        match self {
            Self::Filesystem(fs) => fs.has_write_operations(),
            Self::Shell(shell) => shell.has_write_operations(),
            Self::Network(_) => false,
            Self::Clipboard(cb) => cb.write,
            Self::Notification(_) => true,
        }
    }

    pub fn is_critical(&self) -> bool {
        match self {
            Self::Filesystem(fs) => fs.scope == FsScope::AppData && fs.has_write_operations(),
            Self::Shell(shell) => shell.has_write_operations(),
            Self::Network(net) => net.allow_localhost,
            Self::Clipboard(_) => false,
            Self::Notification(_) => false,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Filesystem(fs) => format!("Filesystem: {} ({})", fs.scope, fs.operations_summary()),
            Self::Shell(shell) => shell.display_name(),
            Self::Network(net) => format!("Network: {} hosts", net.endpoints.len()),
            Self::Clipboard(cb) => format!("Clipboard: {}{}", 
                if cb.read { "read" } else { "" },
                if cb.write { "+write" } else { "" }),
            Self::Notification(_) => "Notification".to_string(),
        }
    }
}

impl fmt::Display for PluginPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsPermission {
    pub scope: FsScope,
    pub operations: Vec<FsOperation>,
}

impl std::hash::Hash for FsPermission {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.scope.hash(state);
        let mut ops = self.operations.clone();
        ops.sort();
        ops.hash(state);
    }
}

impl FsPermission {
    pub fn new(scope: FsScope, operations: Vec<FsOperation>) -> Self {
        Self {
            scope,
            operations,
        }
    }

    pub fn readonly(scope: FsScope) -> Self {
        Self::new(scope, vec![FsOperation::Read, FsOperation::List])
    }

    pub fn full_access(scope: FsScope) -> Self {
        Self::new(scope, vec![
            FsOperation::Read, FsOperation::Write, 
            FsOperation::Delete, FsOperation::List, FsOperation::CreateDir,
        ])
    }

    pub fn has_operation(&self, op: FsOperation) -> bool {
        self.operations.contains(&op)
    }

    pub fn has_write_operations(&self) -> bool {
        self.operations.iter().any(|op| op.is_write_operation())
    }

    pub fn operations_summary(&self) -> String {
        let ops: Vec<String> = self.operations.iter().map(|o| o.to_string()).collect();
        ops.join(",")
    }
}

impl fmt::Display for FsPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fs:{}:[{}]", self.scope, self.operations_summary())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellPermission {
    Git(Vec<GitOperation>),
    Cargo(Vec<CargoOperation>),
    Python(Vec<String>),
    Node(Vec<String>),
}

impl ShellPermission {
    pub fn git_readonly() -> Self {
        Self::Git(vec![
            GitOperation::Status, GitOperation::Diff, 
            GitOperation::Log, GitOperation::Branch, GitOperation::Remote,
        ])
    }

    pub fn git_full() -> Self {
        Self::Git(vec![
            GitOperation::Status, GitOperation::Commit, GitOperation::Push,
            GitOperation::Pull, GitOperation::Fetch, GitOperation::Diff,
            GitOperation::Log, GitOperation::Reset, GitOperation::Branch,
            GitOperation::Checkout, GitOperation::Merge, GitOperation::Rebase,
            GitOperation::Stash, GitOperation::Tag, GitOperation::Remote, GitOperation::Clone,
        ])
    }

    pub fn cargo_safe() -> Self {
        Self::Cargo(vec![
            CargoOperation::Build, CargoOperation::Check, CargoOperation::Test,
            CargoOperation::Clippy, CargoOperation::Fmt, CargoOperation::Doc,
        ])
    }

    pub fn has_write_operations(&self) -> bool {
        match self {
            Self::Git(ops) => ops.iter().any(|o| o.is_write_operation()),
            Self::Cargo(ops) => ops.iter().any(|o| o.is_write_operation()),
            Self::Python(_) | Self::Node(_) => true,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Git(ops) => {
                let ops_str: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
                format!("Git: [{}]", ops_str.join(","))
            }
            Self::Cargo(ops) => {
                let ops_str: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
                format!("Cargo: [{}]", ops_str.join(","))
            }
            Self::Python(scripts) => format!("Python: {} scripts", scripts.len()),
            Self::Node(scripts) => format!("Node: {} scripts", scripts.len()),
        }
    }

    pub fn is_git(&self) -> bool {
        matches!(self, Self::Git(_))
    }

    pub fn is_cargo(&self) -> bool {
        matches!(self, Self::Cargo(_))
    }

    pub fn is_python(&self) -> bool {
        matches!(self, Self::Python(_))
    }

    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }
}

impl fmt::Display for ShellPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPermission {
    pub endpoints: Vec<NetworkEndpoint>,
    pub allow_localhost: bool,
}

impl std::hash::Hash for NetworkPermission {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.allow_localhost.hash(state);
        let mut endpoints = self.endpoints.clone();
        endpoints.sort_by(|a, b| a.host.cmp(&b.host));
        endpoints.hash(state);
    }
}

impl NetworkPermission {
    pub fn new(endpoints: Vec<NetworkEndpoint>, allow_localhost: bool) -> Self {
        Self {
            endpoints,
            allow_localhost,
        }
    }

    pub fn from_hosts(hosts: Vec<String>, allow_localhost: bool) -> Self {
        let endpoints = hosts.into_iter()
            .map(NetworkEndpoint::https)
            .collect();
        Self { endpoints, allow_localhost }
    }

    pub fn provider_only(host: String) -> Self {
        Self::new(vec![NetworkEndpoint::https(host)], false)
    }

    pub fn localhost_only(port: u16) -> Self {
        Self::new(vec![NetworkEndpoint::localhost(port)], true)
    }

    pub fn has_endpoint(&self, endpoint: &NetworkEndpoint) -> bool {
        if endpoint.is_localhost() {
            return self.allow_localhost;
        }
        self.endpoints.contains(endpoint)
    }

    pub fn can_access_host(&self, host: &str) -> bool {
        if host == "127.0.0.1" || host == "localhost" || host == "::1" {
            return self.allow_localhost;
        }
        self.endpoints.iter().any(|e| e.host == host)
    }
}

impl fmt::Display for NetworkPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hosts: Vec<String> = self.endpoints.iter().map(|e| e.host.clone()).collect();
        write!(f, "network:[{}]localhost={}", hosts.join(","), self.allow_localhost)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipboardPermission {
    pub read: bool,
    pub write: bool,
}

impl ClipboardPermission {
    pub fn new(read: bool, write: bool) -> Self {
        Self { read, write }
    }

    pub fn readonly() -> Self {
        Self::new(true, false)
    }

    pub fn writeonly() -> Self {
        Self::new(false, true)
    }

    pub fn full() -> Self {
        Self::new(true, true)
    }

    pub fn can_read(&self) -> bool {
        self.read
    }

    pub fn can_write(&self) -> bool {
        self.write
    }
}

impl fmt::Display for ClipboardPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let caps: Vec<&str> = [
            if self.read { Some("read") } else { None },
            if self.write { Some("write") } else { None },
        ].iter().filter_map(|x| *x).collect();
        write!(f, "clipboard:[{}]", caps.join(","))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationPermission {
    pub enabled: bool,
}

impl NotificationPermission {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

impl Default for NotificationPermission {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NotificationPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "notification:{}", if self.enabled { "enabled" } else { "disabled" })
    }
}