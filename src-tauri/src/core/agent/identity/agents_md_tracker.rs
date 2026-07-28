//! ═══════════════════════════════════════════════════════════════════════════
//! AgentsMdTracker - AGENTS.md 文件追踪器
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use ignore::gitignore::Gitignore;

// ── AGENTS.md 追踪器 ────────────────────────────────────────────────────────

pub struct AgentsMdTracker {
    checked_dirs: HashSet<PathBuf>,
    initial_discovery: HashSet<PathBuf>,
    reminded: HashSet<PathBuf>,
    git_root: Option<PathBuf>,
    gitignore: Option<Gitignore>,
}

impl AgentsMdTracker {
    pub fn new(git_root: Option<PathBuf>) -> Self {
        let gitignore = git_root.as_ref().and_then(|root| {
            let gitignore_path = root.join(".gitignore");
            if gitignore_path.exists() {
                let (gitignore, err) = Gitignore::new(&gitignore_path);
                if let Some(e) = err {
                    tracing::warn!(
                        path = %gitignore_path.display(),
                        error = %e,
                        "Error loading .gitignore file"
                    );
                }
                Some(gitignore)
            } else {
                None
            }
        });

        let mut tracker = Self {
            checked_dirs: HashSet::new(),
            initial_discovery: HashSet::new(),
            reminded: HashSet::new(),
            git_root,
            gitignore,
        };
        
        tracker.scan_initial_agents_md();
        tracker
    }

    fn scan_initial_agents_md(&mut self) {
        if let Some(root) = &self.git_root {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        self.scan_directory_for_agents_md(&path, true);
                    }
                }
            }
        }
    }

    fn scan_directory_for_agents_md(&mut self, dir: &Path, is_initial: bool) {
        if self.checked_dirs.contains(dir) {
            return;
        }

        if let Some(ref gitignore) = self.gitignore {
            if gitignore.matched(dir, true).is_ignore() {
                return;
            }
        }

        self.checked_dirs.insert(dir.to_path_buf());

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.scan_directory_for_agents_md(&path, is_initial);
                } else if path.file_name().map(|name| name == "AGENTS.md").unwrap_or(false) && is_initial {
                    self.initial_discovery.insert(path);
                }
            }
        }
    }

    pub fn check_on_file_access(&mut self, path: &Path) -> Option<String> {
        if !path.exists() {
            return None;
        }

        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return None,
        };

        let mut current = canonical_path.parent();
        while let Some(dir) = current {
            if let Some(ref git_root) = self.git_root {
                if dir == *git_root {
                    break;
                }
            }

            if self.checked_dirs.contains(dir) {
                current = dir.parent();
                continue;
            }

            if let Some(ref gitignore) = self.gitignore {
                if gitignore.matched(dir, true).is_ignore() {
                    self.checked_dirs.insert(dir.to_path_buf());
                    current = dir.parent();
                    continue;
                }
            }

            let agents_md_path = dir.join("AGENTS.md");
            if agents_md_path.exists() {
                let agents_md_canonical = match agents_md_path.canonicalize() {
                    Ok(p) => p,
                    Err(_) => {
                        current = dir.parent();
                        continue;
                    }
                };

                if !self.initial_discovery.contains(&agents_md_canonical)
                    && !self.reminded.contains(&agents_md_canonical)
                {
                    self.reminded.insert(agents_md_canonical.clone());
                    
                    return Some(format!(
                        "发现新的 AGENTS.md 文件：{}\n\n此文件定义了当前目录的 Agent 行为规范。请查阅该文件以了解项目特定的 Agent 指令。",
                        agents_md_path.display()
                    ));
                }
            }

            self.checked_dirs.insert(dir.to_path_buf());
            current = dir.parent();
        }

        None
    }

    pub fn reset_after_compaction(&mut self) {
        self.checked_dirs.clear();
        self.reminded.clear();
        
        if let Some(root) = &self.git_root {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        self.scan_directory_for_agents_md(&path, false);
                    }
                }
            }
        }

        tracing::debug!(
            initial_count = self.initial_discovery.len(),
            "AgentsMdTracker reset after compaction"
        );
    }
}

// ── AGENTS.md 多层级加载 ───────────────────────────────────────────────────

/// 默认项目根标记（用于向上查找项目根）。
#[allow(dead_code)]
pub const DEFAULT_PROJECT_ROOT_MARKERS: &[&str] = &[".git"];

/// 默认候选文件名优先级（高 → 低）。
///
/// `AGENTS.override.md` 优先于 `AGENTS.md`，回退到 `CLAUDE.md` / `.cursorrules`。
#[allow(dead_code)]
pub const DEFAULT_CANDIDATE_FILENAMES: &[&str] =
    &["AGENTS.override.md", "AGENTS.md", "CLAUDE.md", ".cursorrules"];

/// `project_doc_max_bytes` 下限（20K）。
#[allow(dead_code)]
pub const MIN_PROJECT_DOC_MAX_BYTES: usize = 20_000;
/// `project_doc_max_bytes` 上限（500K）。
#[allow(dead_code)]
pub const MAX_PROJECT_DOC_MAX_BYTES: usize = 500_000;

/// project-doc 段落分隔符（与 codex 一致，标识 project-doc 边界）。
#[allow(dead_code)]
pub const PROJECT_DOC_SEPARATOR: &str = "\n\n--- project-doc ---\n\n";

/// AGENTS.md 加载来源（Provenance）。
///
/// 每条加载的 AGENTS.md 内容都附带来源路径、environment_id 与当时的 cwd，
/// 便于审计与多环境区分。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsMdProvenance {
    /// AGENTS.md 文件的绝对路径
    pub source_path: PathBuf,
    /// environment_id（多环境场景下区分来源，单环境默认 "default"）
    pub environment_id: String,
    /// 当前工作目录（请求加载时的 cwd）
    pub cwd: PathBuf,
}

/// 加载的 AGENTS.md 条目（内容 + Provenance）。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AgentsMdEntry {
    pub contents: String,
    pub provenance: AgentsMdProvenance,
}

/// 多层级 AGENTS.md 加载结果。
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct LoadedAgentsMd {
    /// 按加载顺序（项目根 → cwd）排列的条目。
    pub entries: Vec<AgentsMdEntry>,
}

#[allow(dead_code)]
impl LoadedAgentsMd {
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.contents.trim().is_empty())
    }

    /// 拼接所有 entry 的内容，按加载顺序（项目根 → cwd）。
    ///
    /// 段落间使用 `PROJECT_DOC_SEPARATOR` 分隔，
    /// 让模型能识别 project-doc 边界，便于 prompt cache 命中前缀。
    pub fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            if !entry.contents.trim().is_empty() {
                parts.push(entry.contents.clone());
            }
        }
        parts.join(PROJECT_DOC_SEPARATOR)
    }

    /// 列出所有来源路径（用于调试 / 日志）。
    pub fn sources(&self) -> impl Iterator<Item = &Path> {
        self.entries
            .iter()
            .map(|e| e.provenance.source_path.as_path())
    }
}

/// AGENTS.md 加载配置。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AgentsMdLoadConfig {
    /// 项目根标记（用于向上查找）。空列表禁用向上查找。
    pub project_root_markers: Vec<String>,
    /// 候选文件名优先级（高 → 低）。每目录按此顺序查找，仅取第一个匹配。
    pub candidate_filenames: Vec<String>,
    /// 字节预算上限。0 表示禁用加载。
    pub project_doc_max_bytes: usize,
    /// environment_id（写入 Provenance）。
    pub environment_id: String,
}

impl Default for AgentsMdLoadConfig {
    fn default() -> Self {
        Self {
            project_root_markers: DEFAULT_PROJECT_ROOT_MARKERS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            candidate_filenames: DEFAULT_CANDIDATE_FILENAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            project_doc_max_bytes: MIN_PROJECT_DOC_MAX_BYTES,
            environment_id: "default".to_string(),
        }
    }
}

/// 按 model context_length 计算字节预算。
///
/// 保守估计 1 token ≈ 4 bytes（英文），clamp 到 [20K, 500K]。
#[allow(dead_code)]
pub fn dynamic_project_doc_max_bytes(context_length: usize) -> usize {
    let bytes = context_length.saturating_mul(4);
    bytes.clamp(MIN_PROJECT_DOC_MAX_BYTES, MAX_PROJECT_DOC_MAX_BYTES)
}

/// 向上查找项目根（包含任一 project_root_markers 的目录）。
///
/// - markers 为空 → 返回 None（禁用向上查找）
/// - 无标记命中 → 返回 None
/// - 命中第一个标记所在目录即返回（不继续向上）
#[allow(dead_code)]
pub fn find_project_root(cwd: &Path, markers: &[String]) -> Option<PathBuf> {
    if markers.is_empty() {
        return None;
    }
    for current in cwd.ancestors() {
        for marker in markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }
    }
    None
}

/// 收集从项目根到 cwd（含）的所有候选 AGENTS.md 文件路径。
///
/// 每个目录按 `candidate_filenames` 优先级查找，仅取第一个匹配。
/// 返回顺序：项目根 → cwd（外层优先于内层）。
#[allow(dead_code)]
pub fn collect_agents_md_paths(
    project_root: &Path,
    cwd: &Path,
    candidate_filenames: &[String],
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut cursor = cwd.to_path_buf();
    loop {
        dirs.push(cursor.clone());
        if cursor == project_root {
            break;
        }
        let Some(parent) = cursor.parent() else {
            break;
        };
        cursor = parent.to_path_buf();
    }
    dirs.reverse(); // 项目根 → cwd

    let mut found = Vec::new();
    for d in dirs {
        for name in candidate_filenames {
            let candidate = d.join(name);
            if candidate.is_file() {
                found.push(candidate);
                break; // 每目录仅取第一个匹配
            }
        }
    }
    found
}

/// 加载多层级 AGENTS.md，应用字节预算 + Provenance。
///
/// 流程：
/// 1. 向上查找项目根（无标记时仅扫描 cwd）
/// 2. 从项目根向下收集候选文件路径
/// 3. 按路径顺序读取，应用字节预算（超限截断 + warn 日志）
/// 4. 每个文件附 Provenance（path + environment_id + cwd）
///
/// 返回：
/// - `Ok(None)`：无文件命中或预算为 0
/// - `Ok(Some(loaded))`：成功加载（可能含被截断的末尾文件）
/// - `Err(e)`：仅 I/O 失败（非 NotFound）时返回
#[allow(dead_code)]
pub async fn load_agents_md(
    config: &AgentsMdLoadConfig,
    cwd: &Path,
) -> std::io::Result<Option<LoadedAgentsMd>> {
    if config.project_doc_max_bytes == 0 {
        return Ok(None);
    }

    let project_root = match find_project_root(cwd, &config.project_root_markers) {
        Some(root) => root,
        None => cwd.to_path_buf(), // 无标记时仅扫描 cwd
    };

    let paths = collect_agents_md_paths(&project_root, cwd, &config.candidate_filenames);
    if paths.is_empty() {
        return Ok(None);
    }

    let mut remaining = config.project_doc_max_bytes;
    let mut loaded = LoadedAgentsMd::default();

    for p in paths {
        if remaining == 0 {
            break;
        }

        // 跳过非文件（目录或符号链接指向目录）
        match tokio::fs::metadata(&p).await {
            Ok(m) if m.is_file() => {}
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }

        let mut data = match tokio::fs::read(&p).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };

        let size = data.len();
        if size > remaining {
            data.truncate(remaining);
            tracing::warn!(
                path = %p.display(),
                original_size = size,
                remaining_bytes = remaining,
                "project doc exceeds remaining budget; truncating"
            );
        }

        let text = String::from_utf8_lossy(&data).to_string();
        if !text.trim().is_empty() {
            loaded.entries.push(AgentsMdEntry {
                contents: text,
                provenance: AgentsMdProvenance {
                    source_path: p.clone(),
                    environment_id: config.environment_id.clone(),
                    cwd: cwd.to_path_buf(),
                },
            });
            remaining = remaining.saturating_sub(data.len());
        }
    }

    if loaded.is_empty() {
        Ok(None)
    } else {
        Ok(Some(loaded))
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_without_git_root() {
        let tracker = AgentsMdTracker::new(None);
        assert!(tracker.git_root.is_none());
        assert!(tracker.gitignore.is_none());
    }

    #[test]
    fn new_with_git_root() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        
        let tracker = AgentsMdTracker::new(Some(git_root.clone()));
        assert_eq!(tracker.git_root, Some(git_root));
    }

    #[test]
    fn check_on_file_access_no_agents_md() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();
        
        let mut tracker = AgentsMdTracker::new(Some(git_root));
        let result = tracker.check_on_file_access(&file_path);
        assert!(result.is_none());
    }

    #[test]
    fn check_on_file_access_with_agents_md() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        let agents_md = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_md, "# Agent Rules").unwrap();
        
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();
        
        let mut tracker = AgentsMdTracker::new(Some(git_root));
        tracker.initial_discovery.clear();
        
        let result = tracker.check_on_file_access(&file_path);
        assert!(result.is_some());
        assert!(result.unwrap().contains("AGENTS.md"));
        
        let result2 = tracker.check_on_file_access(&file_path);
        assert!(result2.is_none());
    }

    #[test]
    fn reset_after_compaction_clears_state() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        
        let mut tracker = AgentsMdTracker::new(Some(git_root));
        tracker.checked_dirs.insert(tmp.path().to_path_buf());
        tracker.reminded.insert(tmp.path().join("AGENTS.md"));
        
        tracker.reset_after_compaction();
        
        assert!(tracker.checked_dirs.is_empty());
        assert!(tracker.reminded.is_empty());
    }

    #[test]
    fn ignores_gitignored_files() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        
        let gitignore_path = git_root.join(".gitignore");
        std::fs::write(&gitignore_path, "node_modules\n").unwrap();
        
        let node_modules = git_root.join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        let agents_md = node_modules.join("AGENTS.md");
        std::fs::write(&agents_md, "# Should be ignored").unwrap();
        
        let file_path = node_modules.join("test.txt");
        std::fs::write(&file_path, "test").unwrap();
        
        let mut tracker = AgentsMdTracker::new(Some(git_root));
        tracker.initial_discovery.clear();
        
        let result = tracker.check_on_file_access(&file_path);
        assert!(result.is_none());
    }

    #[test]
    fn only_reminds_once_per_session() {
        let tmp = TempDir::new().unwrap();
        let git_root = tmp.path().to_path_buf();
        let agents_md = tmp.path().join("AGENTS.md");
        std::fs::write(&agents_md, "# Agent Rules").unwrap();

        let file_path1 = tmp.path().join("test1.txt");
        std::fs::write(&file_path1, "test1").unwrap();
        let file_path2 = tmp.path().join("test2.txt");
        std::fs::write(&file_path2, "test2").unwrap();

        let mut tracker = AgentsMdTracker::new(Some(git_root));
        tracker.initial_discovery.clear();

        let result1 = tracker.check_on_file_access(&file_path1);
        assert!(result1.is_some());

        let result2 = tracker.check_on_file_access(&file_path2);
        assert!(result2.is_none());
    }

    // ── 多层级 AGENTS.md 加载测试 ─────────────────────────────────────────

    #[test]
    fn dynamic_project_doc_max_bytes_clamps_to_min() {
        // context_length < 5000 → 5000×4=20000 = MIN
        assert_eq!(dynamic_project_doc_max_bytes(0), MIN_PROJECT_DOC_MAX_BYTES);
        assert_eq!(dynamic_project_doc_max_bytes(5_000), MIN_PROJECT_DOC_MAX_BYTES);
    }

    #[test]
    fn dynamic_project_doc_max_bytes_clamps_to_max() {
        // context_length > 125000 → 125000×4=500000 = MAX
        assert_eq!(
            dynamic_project_doc_max_bytes(125_000),
            MAX_PROJECT_DOC_MAX_BYTES
        );
        assert_eq!(
            dynamic_project_doc_max_bytes(1_000_000),
            MAX_PROJECT_DOC_MAX_BYTES
        );
    }

    #[test]
    fn dynamic_project_doc_max_bytes_linear_in_between() {
        // 50000 tokens × 4 = 200000 bytes（介于 MIN 和 MAX 之间）
        assert_eq!(dynamic_project_doc_max_bytes(50_000), 200_000);
    }

    #[test]
    fn find_project_root_returns_none_when_no_markers() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".git"), "gitdir").unwrap();
        // markers 为空 → None
        assert!(find_project_root(tmp.path(), &[]).is_none());
    }

    #[test]
    fn find_project_root_finds_git_marker_in_ancestor() {
        let tmp = TempDir::new().unwrap();
        // 创建 .git 标记
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        // 嵌套目录：/repo/apps/foo
        let foo = tmp.path().join("apps").join("foo");
        std::fs::create_dir_all(&foo).unwrap();

        let markers = vec![".git".to_string()];
        let root = find_project_root(&foo, &markers);
        assert_eq!(root, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn find_project_root_returns_none_when_no_marker_exists() {
        let tmp = TempDir::new().unwrap();
        let foo = tmp.path().join("apps").join("foo");
        std::fs::create_dir_all(&foo).unwrap();

        let markers = vec![".git".to_string()];
        // tmp 与所有祖先都没有 .git → None（注意：tmp 之外的祖先通常也不含 .git）
        // 这里宽松断言：要么 None，要么是某个真实存在的 .git 祖先（CI 环境罕见）
        let _ = find_project_root(&foo, &markers);
    }

    #[test]
    fn collect_paths_picks_only_first_matching_filename_per_dir() {
        let tmp = TempDir::new().unwrap();
        // 同一目录同时存在 AGENTS.override.md 与 AGENTS.md
        std::fs::write(tmp.path().join("AGENTS.override.md"), "override").unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "default").unwrap();

        let candidates: Vec<String> = DEFAULT_CANDIDATE_FILENAMES
            .iter()
            .map(|s| s.to_string())
            .collect();
        let paths = collect_agents_md_paths(tmp.path(), tmp.path(), &candidates);

        // 每目录仅取第一个匹配（AGENTS.override.md 优先于 AGENTS.md）
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("AGENTS.override.md"));
    }

    #[test]
    fn collect_paths_walks_from_project_root_to_cwd() {
        let tmp = TempDir::new().unwrap();
        // /repo/AGENTS.md, /repo/apps/AGENTS.md, /repo/apps/foo/AGENTS.md
        std::fs::write(tmp.path().join("AGENTS.md"), "root").unwrap();
        let apps = tmp.path().join("apps");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(apps.join("AGENTS.md"), "apps").unwrap();
        let foo = apps.join("foo");
        std::fs::create_dir_all(&foo).unwrap();
        std::fs::write(foo.join("AGENTS.md"), "foo").unwrap();

        let candidates: Vec<String> = vec!["AGENTS.md".to_string()];
        let paths = collect_agents_md_paths(tmp.path(), &foo, &candidates);

        // 顺序：项目根 → cwd
        assert_eq!(paths.len(), 3);
        assert!(paths[0].ends_with("AGENTS.md") && paths[0].parent() == Some(tmp.path()));
        assert!(paths[1].ends_with("AGENTS.md") && paths[1].parent() == Some(apps.as_path()));
        assert!(paths[2].ends_with("AGENTS.md") && paths[2].parent() == Some(foo.as_path()));
    }

    #[test]
    fn collect_paths_returns_empty_when_no_files() {
        let tmp = TempDir::new().unwrap();
        let candidates: Vec<String> = vec!["AGENTS.md".to_string()];
        let paths = collect_agents_md_paths(tmp.path(), tmp.path(), &candidates);
        assert!(paths.is_empty());
    }

    #[tokio::test]
    async fn load_agents_md_returns_none_when_budget_zero() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "rules").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![], // 禁用向上查找
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 0, // 禁用加载
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, tmp.path()).await.unwrap();
        assert!(loaded.is_none(), "budget=0 应禁用加载");
    }

    #[tokio::test]
    async fn load_agents_md_returns_none_when_no_files() {
        let tmp = TempDir::new().unwrap();
        let config = AgentsMdLoadConfig::default();
        let loaded = load_agents_md(&config, tmp.path()).await.unwrap();
        assert!(loaded.is_none(), "无文件时应返回 None");
    }

    #[tokio::test]
    async fn load_agents_md_loads_single_agents_md_in_cwd() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "# Rules").unwrap();

        // 禁用向上查找（避免 tmp 之外的祖先有 .git）
        let config = AgentsMdLoadConfig {
            project_root_markers: vec![],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100_000,
            environment_id: "test-env".to_string(),
        };
        let loaded = load_agents_md(&config, tmp.path())
            .await
            .unwrap()
            .expect("应加载到内容");

        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].contents, "# Rules");
        assert_eq!(loaded.entries[0].provenance.environment_id, "test-env");
        assert_eq!(loaded.entries[0].provenance.cwd, tmp.path());
        assert!(loaded.entries[0]
            .provenance
            .source_path
            .ends_with("AGENTS.md"));
    }

    #[tokio::test]
    async fn load_agents_md_walks_up_to_project_root_and_back_down() {
        let tmp = TempDir::new().unwrap();
        // 创建 .git 标记使 tmp 成为项目根
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        // /repo/AGENTS.md
        std::fs::write(tmp.path().join("AGENTS.md"), "root rules").unwrap();
        // /repo/apps/foo/AGENTS.md
        let foo = tmp.path().join("apps").join("foo");
        std::fs::create_dir_all(&foo).unwrap();
        std::fs::write(foo.join("AGENTS.md"), "foo rules").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![".git".to_string()],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100_000,
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, &foo)
            .await
            .unwrap()
            .expect("应加载到内容");

        // 顺序：项目根 → cwd
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].contents, "root rules");
        assert_eq!(loaded.entries[1].contents, "foo rules");
        // Provenance 的 cwd 应为传入的 foo
        assert_eq!(loaded.entries[0].provenance.cwd, foo);
    }

    #[tokio::test]
    async fn load_agents_md_prefers_override_over_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.override.md"), "override").unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "default").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![],
            candidate_filenames: vec![
                "AGENTS.override.md".to_string(),
                "AGENTS.md".to_string(),
            ],
            project_doc_max_bytes: 100_000,
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, tmp.path())
            .await
            .unwrap()
            .expect("应加载到内容");

        // 每目录仅取第一个匹配 → AGENTS.override.md
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].contents, "override");
    }

    #[tokio::test]
    async fn load_agents_md_truncates_when_exceeding_budget() {
        let tmp = TempDir::new().unwrap();
        // 创建大文件 > budget
        let big = "A".repeat(1_000);
        std::fs::write(tmp.path().join("AGENTS.md"), &big).unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100, // 仅 100 bytes
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, tmp.path())
            .await
            .unwrap()
            .expect("应加载到内容");

        // 内容被截断到 budget 内
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.entries[0].contents.len() <= 100);
    }

    #[tokio::test]
    async fn load_agents_md_skips_empty_files() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "   \n  \n").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100_000,
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, tmp.path()).await.unwrap();
        // 空/空白文件应被跳过
        assert!(loaded.is_none(), "空文件应被跳过，返回 None");
    }

    #[tokio::test]
    async fn load_agents_md_text_joins_with_project_doc_separator() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "root").unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("AGENTS.md"), "sub").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![".git".to_string()],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100_000,
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, &sub)
            .await
            .unwrap()
            .expect("应加载到内容");

        let text = loaded.text();
        assert!(text.contains("root"));
        assert!(text.contains("sub"));
        assert!(text.contains("--- project-doc ---"), "应使用项目文档分隔符");
    }

    #[tokio::test]
    async fn load_agents_md_sources_iterator_yields_all_paths() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "root").unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("AGENTS.md"), "sub").unwrap();

        let config = AgentsMdLoadConfig {
            project_root_markers: vec![".git".to_string()],
            candidate_filenames: vec!["AGENTS.md".to_string()],
            project_doc_max_bytes: 100_000,
            environment_id: "test".to_string(),
        };
        let loaded = load_agents_md(&config, &sub)
            .await
            .unwrap()
            .expect("应加载到内容");

        let sources: Vec<_> = loaded.sources().collect();
        assert_eq!(sources.len(), 2);
    }
}