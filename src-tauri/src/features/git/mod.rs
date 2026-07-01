pub mod detector;
pub mod installer;
pub mod models;
pub mod service;

pub use detector::{detect_git, git_executable, parse_version};
pub use installer::install_git;
pub use models::*;
pub use service::GitService;
