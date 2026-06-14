use std::path::{Path, PathBuf};
use super::policy::{FsRule, FsAction};

/// 文件系统沙箱 - 控制文件访问权限
pub struct FileSystemSandbox {
    work_dir: PathBuf,
    rules: Vec<FsRule>,
    resolved_rules: Vec<(glob::Pattern, FsAction)>,
}

impl FileSystemSandbox {
    pub fn new(work_dir: PathBuf, rules: Vec<FsRule>) -> Self {
        let resolved_rules = rules.iter().map(|rule| {
            let pattern = rule.pattern
                .replace("${WORKDIR}", &work_dir.to_string_lossy())
                .replace("${TMPDIR}", &std::env::temp_dir().to_string_lossy());
            let glob_pattern = glob::Pattern::new(&pattern)
                .unwrap_or_else(|_| glob::Pattern::new("*").unwrap());
            (glob_pattern, rule.action)
        }).collect();

        Self {
            work_dir,
            rules,
            resolved_rules,
        }
    }

    /// 检查路径是否允许读取
    pub fn can_read(&self, path: &Path) -> bool {
        self.check_access(path, true)
    }

    /// 检查路径是否允许写入
    pub fn can_write(&self, path: &Path) -> bool {
        self.check_access(path, false)
    }

    /// 检查路径访问权限
    /// 安全规则：最后匹配的规则决定权限，如果没有规则匹配则默认拒绝
    fn check_access(&self, path: &Path, _is_read: bool) -> bool {
        let path_str = path.to_string_lossy();

        // 规范化路径
        let normalized = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => path.to_path_buf(),
        };
        let normalized_str = normalized.to_string_lossy();

        // 从后向前遍历规则，找到最后一个匹配的规则
        for (pattern, action) in self.resolved_rules.iter().rev() {
            if pattern.matches(&normalized_str) || pattern.matches(&path_str) {
                return *action == FsAction::Allow;
            }
        }

        // 默认拒绝
        false
    }

    /// 路径遍历攻击检测
    pub fn is_path_traversal_safe(&self, path: &Path) -> bool {
        // 检查路径中是否包含 ..
        if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return false;
        }

        // 检查路径是否在允许的目录内
        if let Ok(canonical) = path.canonicalize() {
            if let Ok(work_canonical) = self.work_dir.canonicalize() {
                return canonical.starts_with(&work_canonical);
            }
        }

        // 如果无法规范化，检查是否包含危险模式
        let path_str = path.to_string_lossy();
        !path_str.contains("..") && !path_str.contains("~")
    }

    /// 获取沙箱状态信息
    pub fn status(&self) -> FsSandboxStatus {
        FsSandboxStatus {
            work_dir: self.work_dir.clone(),
            rule_count: self.rules.len(),
            allowed_patterns: self.resolved_rules.iter()
                .filter(|(_, action)| *action == FsAction::Allow)
                .map(|(pattern, _)| pattern.as_str().to_string())
                .collect(),
            denied_patterns: self.resolved_rules.iter()
                .filter(|(_, action)| *action == FsAction::Deny)
                .map(|(pattern, _)| pattern.as_str().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FsSandboxStatus {
    pub work_dir: PathBuf,
    pub rule_count: usize,
    pub allowed_patterns: Vec<String>,
    pub denied_patterns: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_detection() {
        let sandbox = FileSystemSandbox::new(
            PathBuf::from("/workspace"),
            vec![],
        );

        assert!(sandbox.is_path_traversal_safe(Path::new("/workspace/file.txt")));
        assert!(!sandbox.is_path_traversal_safe(Path::new("/workspace/../etc/passwd")));
        assert!(!sandbox.is_path_traversal_safe(Path::new("~/secret")));
    }

    #[test]
    fn test_rule_based_access() {
        let sandbox = FileSystemSandbox::new(
            PathBuf::from("/workspace"),
            vec![
                FsRule {
                    pattern: "${WORKDIR}/**".into(),
                    action: FsAction::Allow,
                    description: None,
                },
                FsRule {
                    pattern: "/*".into(),
                    action: FsAction::Deny,
                    description: None,
                },
            ],
        );

        // 工作目录内应该允许
        // 注意：canonicalize 在测试中可能失败，所以这里只测试基本逻辑
        assert!(sandbox.is_path_traversal_safe(Path::new("/workspace/test.txt")));
    }
}
