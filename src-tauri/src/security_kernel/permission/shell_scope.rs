//! ═══════════════════════════════════════════════════════════════════════════
//! shell_scope - Shell 范围定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

// ── Shell 范围 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellScope {
    Git { operations: HashSet<GitOperation> },
    Python { scripts: HashSet<String> },
    Node { scripts: HashSet<String> },
    Cargo { operations: HashSet<CargoOperation> },
}

impl Hash for ShellScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Git { operations } => {
                "git".hash(state);
                let mut ops: Vec<_> = operations.iter().collect();
                ops.sort();
                ops.hash(state);
            }
            Self::Python { scripts } => {
                "python".hash(state);
                let mut s: Vec<_> = scripts.iter().collect();
                s.sort();
                s.hash(state);
            }
            Self::Node { scripts } => {
                "node".hash(state);
                let mut s: Vec<_> = scripts.iter().collect();
                s.sort();
                s.hash(state);
            }
            Self::Cargo { operations } => {
                "cargo".hash(state);
                let mut ops: Vec<_> = operations.iter().collect();
                ops.sort();
                ops.hash(state);
            }
        }
    }
}

impl ShellScope {
    pub fn git(operations: Vec<GitOperation>) -> Self {
        Self::Git { operations: operations.into_iter().collect() }
    }

    pub fn git_all() -> Self {
        Self::Git { operations: GitOperation::all() }
    }

    pub fn git_readonly() -> Self {
        Self::Git { operations: GitOperation::readonly() }
    }

    pub fn python(scripts: Vec<String>) -> Self {
        Self::Python { scripts: scripts.into_iter().collect() }
    }

    pub fn node(scripts: Vec<String>) -> Self {
        Self::Node { scripts: scripts.into_iter().collect() }
    }

    pub fn cargo(operations: Vec<CargoOperation>) -> Self {
        Self::Cargo { operations: operations.into_iter().collect() }
    }

    pub fn cargo_all() -> Self {
        Self::Cargo { operations: CargoOperation::all() }
    }

    pub fn is_git(&self) -> bool {
        matches!(self, Self::Git { .. })
    }

    pub fn is_python(&self) -> bool {
        matches!(self, Self::Python { .. })
    }

    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node { .. })
    }

    pub fn is_cargo(&self) -> bool {
        matches!(self, Self::Cargo { .. })
    }
}

impl fmt::Display for ShellScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Git { operations } => {
                let ops: Vec<String> = operations.iter().map(|o| o.to_string()).collect();
                write!(f, "git:[{}]", ops.join(","))
            }
            Self::Python { scripts } => {
                write!(f, "python:[{}]", scripts.iter().cloned().collect::<Vec<_>>().join(","))
            }
            Self::Node { scripts } => {
                write!(f, "node:[{}]", scripts.iter().cloned().collect::<Vec<_>>().join(","))
            }
            Self::Cargo { operations } => {
                let ops: Vec<String> = operations.iter().map(|o| o.to_string()).collect();
                write!(f, "cargo:[{}]", ops.join(","))
            }
        }
    }
}

// ── Git 操作 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum GitOperation {
    Status,
    Commit,
    Push,
    Pull,
    Fetch,
    Diff,
    Log,
    Reset,
    Branch,
    Checkout,
    Merge,
    Rebase,
    Stash,
    Tag,
    Remote,
    Clone,
}

impl GitOperation {
    pub fn all() -> HashSet<Self> {
        use GitOperation::*;
        [Status, Commit, Push, Pull, Fetch, Diff, Log, Reset, Branch, Checkout, Merge, Rebase, Stash, Tag, Remote, Clone]
            .into_iter()
            .collect()
    }

    pub fn readonly() -> HashSet<Self> {
        use GitOperation::*;
        [Status, Diff, Log, Branch, Remote].into_iter().collect()
    }

    pub fn is_write_operation(&self) -> bool {
        use GitOperation::*;
        matches!(self, Commit | Push | Reset | Merge | Rebase | Stash | Tag | Clone)
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        use GitOperation::*;
        match s {
            "status" => Some(Status),
            "commit" => Some(Commit),
            "push" => Some(Push),
            "pull" => Some(Pull),
            "fetch" => Some(Fetch),
            "diff" => Some(Diff),
            "log" => Some(Log),
            "reset" => Some(Reset),
            "branch" => Some(Branch),
            "checkout" => Some(Checkout),
            "merge" => Some(Merge),
            "rebase" => Some(Rebase),
            "stash" => Some(Stash),
            "tag" => Some(Tag),
            "remote" => Some(Remote),
            "clone" => Some(Clone),
            _ => None,
        }
    }
}

impl fmt::Display for GitOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status => write!(f, "status"),
            Self::Commit => write!(f, "commit"),
            Self::Push => write!(f, "push"),
            Self::Pull => write!(f, "pull"),
            Self::Fetch => write!(f, "fetch"),
            Self::Diff => write!(f, "diff"),
            Self::Log => write!(f, "log"),
            Self::Reset => write!(f, "reset"),
            Self::Branch => write!(f, "branch"),
            Self::Checkout => write!(f, "checkout"),
            Self::Merge => write!(f, "merge"),
            Self::Rebase => write!(f, "rebase"),
            Self::Stash => write!(f, "stash"),
            Self::Tag => write!(f, "tag"),
            Self::Remote => write!(f, "remote"),
            Self::Clone => write!(f, "clone"),
        }
    }
}

// ── Cargo 操作 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum CargoOperation {
    Build,
    Check,
    Test,
    Run,
    Clippy,
    Fmt,
    Doc,
    Clean,
    Publish,
    Install,
    Update,
    Add,
    Remove,
}

impl CargoOperation {
    pub fn all() -> HashSet<Self> {
        use CargoOperation::*;
        [Build, Check, Test, Run, Clippy, Fmt, Doc, Clean, Publish, Install, Update, Add, Remove]
            .into_iter()
            .collect()
    }

    pub fn safe() -> HashSet<Self> {
        use CargoOperation::*;
        [Build, Check, Test, Clippy, Fmt, Doc].into_iter().collect()
    }

    pub fn is_write_operation(&self) -> bool {
        use CargoOperation::*;
        matches!(self, Clean | Publish | Install | Update | Add | Remove)
    }

    pub fn from_str_name(s: &str) -> Option<Self> {
        use CargoOperation::*;
        match s {
            "build" => Some(Build),
            "check" => Some(Check),
            "test" => Some(Test),
            "run" => Some(Run),
            "clippy" => Some(Clippy),
            "fmt" => Some(Fmt),
            "doc" => Some(Doc),
            "clean" => Some(Clean),
            "publish" => Some(Publish),
            "install" => Some(Install),
            "update" => Some(Update),
            "add" => Some(Add),
            "remove" => Some(Remove),
            _ => None,
        }
    }
}

impl fmt::Display for CargoOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build => write!(f, "build"),
            Self::Check => write!(f, "check"),
            Self::Test => write!(f, "test"),
            Self::Run => write!(f, "run"),
            Self::Clippy => write!(f, "clippy"),
            Self::Fmt => write!(f, "fmt"),
            Self::Doc => write!(f, "doc"),
            Self::Clean => write!(f, "clean"),
            Self::Publish => write!(f, "publish"),
            Self::Install => write!(f, "install"),
            Self::Update => write!(f, "update"),
            Self::Add => write!(f, "add"),
            Self::Remove => write!(f, "remove"),
        }
    }
}