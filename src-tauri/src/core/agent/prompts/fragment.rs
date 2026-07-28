//! ═══════════════════════════════════════════════════════════════════════════
//! Fragment - ContextualUserFragment trait 注入模型内容的统一抽象
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;

use crate::core::agent::identity::estimate_tokens;
use crate::shared::error::AppError;

// ── 常量定义 ────────────────────────────────────────────────────────────────

/// 启动期 lint 触发阈值（tokens）。
///
/// 单个 fragment 渲染后估算 token 数超过此阈值时，`FragmentRegistry::lint`
/// 会产出 `FragmentLintWarning`。10K tokens 约占 200K context 的 5%，
/// 单 fragment 占比超过此值通常意味着内容应当拆分或外部化为工具调用。
pub const FRAGMENT_LINT_TOKEN_THRESHOLD: usize = 10_000;

// ── FragmentRole 枚举 ─────────────────────────────────────────────────────

/// fragment 在消息序列中的角色。
///
/// 对应 OpenAI / Anthropic 消息协议的 role 字段：
/// - System：系统提示词（SOUL / 环境约束 / 安全规则）
/// - Developer：开发者指令（permissions / 工具使用约束）
/// - User：用户级上下文（memory / user_profile / 自定义指令）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentRole {
    System,
    Developer,
    User,
}

impl FragmentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            FragmentRole::System => "system",
            FragmentRole::Developer => "developer",
            FragmentRole::User => "user",
        }
    }
}

// ── ContextualUserFragment Trait ───────────────────────────────────────────

/// 注入模型的内容片段。
///
/// 所有要进入 system prompt / developer message / user message 的内容
/// 必须实现此 trait，由 `FragmentRegistry` 统一注册与渲染。
pub trait ContextualUserFragment: Send + Sync {
    /// fragment 名称（用于日志、去重与 lint 报告）。
    ///
    /// 同一 registry 内不应有重名 fragment（重名由 `register` 拒绝）。
    fn name(&self) -> &str;

    /// 完整渲染：返回该 fragment 的完整文本。
    ///
    /// 首次注入或 `render_diff` 返回 None 时使用。
    fn render_full(&self) -> Result<String, AppError>;

    /// 差异渲染：与 baseline 比较后返回 diff 文本。
    ///
    /// 返回 `Some(diff)` 时用 diff 替代全量，保护 prompt cache 命中前缀；
    /// 返回 `None` 时回退到 `render_full`。
    ///
    /// 默认实现返回 `None`（大多数 fragment 不支持 diff，每次全量渲染）。
    /// 支持 diff 的 fragment（如 WorldState）需重写此方法并通过 `as_any`
    /// 把 `baseline` 下转回自身类型进行比较。
    fn render_diff(&self, _baseline: &dyn ContextualUserFragment) -> Option<String> {
        None
    }

    /// 硬上限（tokens）。超过则启动期 lint 警告。
    ///
    /// 返回 `None` 表示不设硬上限（仍受 registry 总 token 预算约束）。
    /// 返回 `Some(n)` 表示该 fragment 设计上不应超过 n tokens，
    /// `FragmentRegistry::lint` 会在渲染后估算实际 token 数并对比。
    fn bounded_size(&self) -> Option<usize>;

    /// 消息角色（system / developer / user）。
    fn role(&self) -> FragmentRole;

    /// 提供 `Any` 支持，用于 `render_diff` 中下转 baseline。
    ///
    /// 默认实现要求 `Self: 'static`，所有 fragment 实现都满足此约束
    /// （都是具名 struct，无生命周期参数）。
    fn as_any(&self) -> &dyn Any;
}

// ── FragmentLint ───────────────────────────────────────────────────────────

/// Fragment lint 警告。
///
/// 由 `FragmentRegistry::lint` 产出，描述单个 fragment 的违规情况。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentLintWarning {
    /// 触发警告的 fragment 名称。
    pub fragment_name: String,
    /// 警告类型。
    pub kind: FragmentLintKind,
    /// 人类可读的描述（含具体数值）。
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentLintKind {
    /// 渲染后估算 token 数超过 `bounded_size` 或 `FRAGMENT_LINT_TOKEN_THRESHOLD`。
    ExceedsBoundedSize,
}

// ── FragmentRegistry ───────────────────────────────────────────────────────

/// Fragment 注册表。
///
/// 启动期注册所有 fragment，提供：
/// - `render_all`：按注册顺序拼接所有 fragment 的 `render_full` 输出
/// - `lint`：检查每个 fragment 是否超过 bounded_size 或全局阈值
///
/// 注册时拒绝重名 fragment（避免去重歧义）。
pub struct FragmentRegistry {
    fragments: Vec<Box<dyn ContextualUserFragment>>,
}

impl FragmentRegistry {
    pub fn new() -> Self {
        Self {
            fragments: Vec::new(),
        }
    }

    /// 注册一个 fragment。
    ///
    /// 重名 fragment 返回 `Err`，避免去重与 lint 报告歧义。
    pub fn register(&mut self, fragment: Box<dyn ContextualUserFragment>) -> Result<(), AppError> {
        let name = fragment.name();
        if self.fragments.iter().any(|f| f.name() == name) {
            return Err(AppError::conflict(format!(
                "Fragment '{}' is already registered; duplicate fragment names are not allowed",
                name
            )));
        }
        self.fragments.push(fragment);
        Ok(())
    }

    /// 当前注册的 fragment 数量。
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// 按注册顺序渲染所有 fragment 的 `render_full` 输出。
    ///
    /// 各 fragment 之间用 `\n\n---\n\n` 分隔（与 `tiered::render_system_prompt`
    /// 的 tier 分隔符一致，保持 prompt cache 友好）。
    ///
    /// 任一 fragment 渲染失败则向上传播错误（不静默跳过）。
    pub fn render_all(&self) -> Result<String, AppError> {
        let mut parts: Vec<String> = Vec::with_capacity(self.fragments.len());
        for f in &self.fragments {
            let rendered = f.render_full()?;
            if !rendered.trim().is_empty() {
                parts.push(rendered);
            }
        }
        Ok(parts.join("\n\n---\n\n"))
    }

    /// 检查所有 fragment 的 bounded_size 与全局阈值。
    ///
    /// 对每个 fragment：
    /// - 调用 `render_full` 估算实际 token 数
    /// - 若 `bounded_size` 返回 `Some(n)` 且实际 > n → 产出 ExceedsBoundedSize 警告
    /// - 若实际 token 数 > `FRAGMENT_LINT_TOKEN_THRESHOLD` 且无 bounded_size 约束 → 产出警告
    ///
    /// 渲染失败的 fragment 产出一个描述性警告（不阻塞其他 fragment 的 lint）。
    pub fn lint(&self) -> Vec<FragmentLintWarning> {
        let mut warnings = Vec::new();
        for f in &self.fragments {
            let name = f.name().to_string();
            let rendered = match f.render_full() {
                Ok(s) => s,
                Err(e) => {
                    warnings.push(FragmentLintWarning {
                        fragment_name: name,
                        kind: FragmentLintKind::ExceedsBoundedSize,
                        message: format!("render_full failed during lint: {}", e),
                    });
                    continue;
                }
            };
            let actual_tokens = estimate_tokens(&rendered);

            if let Some(bound) = f.bounded_size() {
                if actual_tokens > bound {
                    warnings.push(FragmentLintWarning {
                        fragment_name: name.clone(),
                        kind: FragmentLintKind::ExceedsBoundedSize,
                        message: format!(
                            "fragment '{}' rendered {} tokens, exceeds bounded_size {}",
                            name, actual_tokens, bound
                        ),
                    });
                }
            } else if actual_tokens > FRAGMENT_LINT_TOKEN_THRESHOLD {
                warnings.push(FragmentLintWarning {
                    fragment_name: name.clone(),
                    kind: FragmentLintKind::ExceedsBoundedSize,
                    message: format!(
                        "fragment '{}' rendered {} tokens, exceeds global threshold {}",
                        name, actual_tokens, FRAGMENT_LINT_TOKEN_THRESHOLD
                    ),
                });
            }
        }
        warnings
    }
}

impl Default for FragmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 fragment：固定内容 + 可配置 bounded_size。
    struct StubFragment {
        name: &'static str,
        body: String,
        bound: Option<usize>,
        role: FragmentRole,
    }

    impl StubFragment {
        fn new(name: &'static str, body: &str) -> Self {
            Self {
                name,
                body: body.to_string(),
                bound: None,
                role: FragmentRole::System,
            }
        }

        fn with_bound(mut self, n: usize) -> Self {
            self.bound = Some(n);
            self
        }

        #[allow(dead_code)]
        fn with_role(mut self, r: FragmentRole) -> Self {
            self.role = r;
            self
        }
    }

    impl ContextualUserFragment for StubFragment {
        fn name(&self) -> &str {
            self.name
        }
        fn render_full(&self) -> Result<String, AppError> {
            Ok(self.body.clone())
        }
        fn bounded_size(&self) -> Option<usize> {
            self.bound
        }
        fn role(&self) -> FragmentRole {
            self.role
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn fragment_role_as_str() {
        assert_eq!(FragmentRole::System.as_str(), "system");
        assert_eq!(FragmentRole::Developer.as_str(), "developer");
        assert_eq!(FragmentRole::User.as_str(), "user");
    }

    #[test]
    fn render_diff_default_returns_none() {
        let f1 = StubFragment::new("a", "body");
        let f2 = StubFragment::new("b", "body");
        // 默认 render_diff 返回 None
        assert!(f1.render_diff(&f2).is_none());
    }

    #[test]
    fn registry_register_and_render_all() {
        let mut reg = FragmentRegistry::new();
        assert!(reg.is_empty());

        reg.register(Box::new(StubFragment::new("soul", "SOUL_CONTENT")))
            .unwrap();
        reg.register(Box::new(StubFragment::new("memory", "MEMORY_CONTENT")))
            .unwrap();
        assert_eq!(reg.len(), 2);

        let rendered = reg.render_all().unwrap();
        assert!(rendered.contains("SOUL_CONTENT"));
        assert!(rendered.contains("MEMORY_CONTENT"));
        assert_eq!(rendered.matches("\n\n---\n\n").count(), 1);
    }

    #[test]
    fn registry_rejects_duplicate_names() {
        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(StubFragment::new("soul", "v1")))
            .unwrap();
        let err = reg.register(Box::new(StubFragment::new("soul", "v2")));
        assert!(err.is_err(), "duplicate name should be rejected");
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_render_all_skips_empty_fragments() {
        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(StubFragment::new("a", "AAA")))
            .unwrap();
        reg.register(Box::new(StubFragment::new("b", "   ")))
            .unwrap();
        reg.register(Box::new(StubFragment::new("c", "CCC")))
            .unwrap();

        let rendered = reg.render_all().unwrap();
        // 空 fragment 被跳过，不应出现连续分隔符
        assert!(!rendered.contains("---\n\n\n\n---"));
        assert!(rendered.contains("AAA"));
        assert!(rendered.contains("CCC"));
    }

    #[test]
    fn registry_render_all_empty_returns_empty_string() {
        let reg = FragmentRegistry::new();
        assert_eq!(reg.render_all().unwrap(), "");
    }

    #[test]
    fn lint_no_warnings_when_under_bounds() {
        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(
            StubFragment::new("small", "short content").with_bound(100),
        ))
        .unwrap();
        let warnings = reg.lint();
        assert!(warnings.is_empty(), "got {:?}", warnings);
    }

    #[test]
    fn lint_warns_when_exceeds_bounded_size() {
        let big = "word ".repeat(10_000); // ≈13K tokens
        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(
            StubFragment::new("big", &big).with_bound(5_000),
        ))
        .unwrap();
        let warnings = reg.lint();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].fragment_name, "big");
        assert_eq!(warnings[0].kind, FragmentLintKind::ExceedsBoundedSize);
        assert!(warnings[0].message.contains("exceeds bounded_size 5000"));
    }

    #[test]
    fn lint_warns_when_exceeds_global_threshold_without_bound() {
        // 无 bounded_size，但渲染后 token > FRAGMENT_LINT_TOKEN_THRESHOLD
        let big = "word ".repeat(20_000); // ≈26K tokens > 10K
        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(StubFragment::new("unbounded", &big)))
            .unwrap();
        let warnings = reg.lint();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("exceeds global threshold"));
    }

    #[test]
    fn lint_records_render_failure_without_panicking() {
        struct FailingFragment;
        impl ContextualUserFragment for FailingFragment {
            fn name(&self) -> &str {
                "failing"
            }
            fn render_full(&self) -> Result<String, AppError> {
                Err(AppError::internal("render failed"))
            }
            fn bounded_size(&self) -> Option<usize> {
                None
            }
            fn role(&self) -> FragmentRole {
                FragmentRole::System
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        let mut reg = FragmentRegistry::new();
        reg.register(Box::new(FailingFragment)).unwrap();
        let warnings = reg.lint();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("render_full failed"));
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = FragmentRegistry::default();
        assert!(reg.is_empty());
    }
}
