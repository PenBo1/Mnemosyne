use super::policy::{NetRule, NetAction};

/// 网络沙箱 - 控制网络访问权限
pub struct NetworkSandbox {
    rules: Vec<NetRule>,
}

/// 网络访问权限评估结果
#[derive(Debug, Clone)]
pub struct NetDecision {
    pub allowed: bool,
    pub reason: String,
    pub matched_rule: Option<String>,
}

impl NetworkSandbox {
    pub fn new(rules: Vec<NetRule>) -> Self {
        Self { rules }
    }

    /// 评估是否允许访问指定主机
    pub fn evaluate(&self, host: &str) -> NetDecision {
        let host_lower = host.to_lowercase();

        // 从后向前遍历规则，找到最后一个匹配的规则
        for rule in self.rules.iter().rev() {
            if self.matches_host(&rule.host, &host_lower) {
                return NetDecision {
                    allowed: rule.action == NetAction::Allow,
                    reason: rule.description.clone()
                        .unwrap_or_else(|| format!("主机匹配: {}", rule.host)),
                    matched_rule: Some(rule.host.clone()),
                };
            }
        }

        // 默认拒绝
        NetDecision {
            allowed: false,
            reason: "没有匹配的规则，默认拒绝".into(),
            matched_rule: None,
        }
    }

    /// 检查主机是否匹配规则
    fn matches_host(&self, pattern: &str, host: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // 精确匹配
        if pattern == host {
            return true;
        }

        // 通配符匹配
        if pattern.starts_with("*.") {
            let suffix = &pattern[1..];
            return host.ends_with(suffix);
        }

        // 子域匹配
        if host.ends_with(&format!(".{}", pattern)) || host == pattern {
            return true;
        }

        false
    }

    /// 检查是否允许 URL 访问
    pub fn can_access_url(&self, url: &str) -> NetDecision {
        // 提取主机名
        let host = self.extract_host(url);
        match host {
            Some(h) => self.evaluate(&h),
            None => NetDecision {
                allowed: false,
                reason: "无法解析 URL 主机名".into(),
                matched_rule: None,
            },
        }
    }

    /// 从 URL 提取主机名
    fn extract_host(&self, url: &str) -> Option<String> {
        let url = url.trim();

        // 移除协议前缀
        let without_protocol = if let Some(pos) = url.find("://") {
            &url[pos + 3..]
        } else {
            url
        };

        // 移除路径和查询参数
        let host = without_protocol
            .split('/')
            .next()?
            .split('?')
            .next()?
            .split(':')
            .next()?;

        if host.is_empty() {
            None
        } else {
            Some(host.to_string())
        }
    }

    /// 获取沙箱状态信息
    pub fn status(&self) -> NetSandboxStatus {
        NetSandboxStatus {
            rule_count: self.rules.len(),
            allowed_hosts: self.rules.iter()
                .filter(|r| r.action == NetAction::Allow)
                .map(|r| r.host.clone())
                .collect(),
            denied_hosts: self.rules.iter()
                .filter(|r| r.action == NetAction::Deny)
                .map(|r| r.host.clone())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetSandboxStatus {
    pub rule_count: usize,
    pub allowed_hosts: Vec<String>,
    pub denied_hosts: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_matching() {
        let sandbox = NetworkSandbox::new(vec![
            NetRule {
                host: "*".into(),
                action: NetAction::Deny,
                description: None,
            },
            NetRule {
                host: "localhost".into(),
                action: NetAction::Allow,
                description: None,
            },
            NetRule {
                host: "*.github.com".into(),
                action: NetAction::Allow,
                description: None,
            },
        ]);

        assert!(sandbox.evaluate("localhost").allowed);
        assert!(sandbox.evaluate("api.github.com").allowed);
        assert!(!sandbox.evaluate("evil.com").allowed);
    }

    #[test]
    fn test_url_parsing() {
        let sandbox = NetworkSandbox::new(vec![]);
        assert_eq!(sandbox.extract_host("https://api.github.com/repos"), Some("api.github.com".into()));
        assert_eq!(sandbox.extract_host("http://localhost:8080/api"), Some("localhost".into()));
        assert_eq!(sandbox.extract_host("ftp://files.example.com"), Some("files.example.com".into()));
    }
}
