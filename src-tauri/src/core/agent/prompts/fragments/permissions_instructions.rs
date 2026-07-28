//! ═══════════════════════════════════════════════════════════════════════════
//! PermissionsInstructionsFragment - 权限指令 fragment
//! ═══════════════════════════════════════════════════════════════════════════

use std::any::Any;
use std::path::{Path, PathBuf};

use crate::security_kernel::approval::policy::AskForApproval;
use crate::security_kernel::sandbox::PermissionProfile;
use crate::shared::error::AppError;

use super::super::fragment::{ContextualUserFragment, FragmentRole};

/// 沙箱模式（用于 prompt 渲染）。
///
/// 与 `security_kernel::sandbox::PermissionProfile` 1:1 映射，但保持独立类型：
/// - `PermissionProfile` 是执行层概念（控制沙箱是否激活）
/// - `SandboxMode` 是渲染层概念（控制 prompt 中如何描述沙箱）
///
/// 通过 `From<PermissionProfile>` 转换，避免在 prompt 层泄漏执行层细节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    Managed,
    Disabled,
    External,
}

impl SandboxMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SandboxMode::Managed => "managed",
            SandboxMode::Disabled => "disabled",
            SandboxMode::External => "external",
        }
    }

    /// 渲染 sandbox_mode 模板（解释当前沙箱行为给模型）。
    fn render_template(self) -> &'static str {
        match self {
            SandboxMode::Managed => {
                "Sandbox mode: managed. File writes and command execution are \
                 enforced by a platform sandbox (landlock/seatbelt/restricted token). \
                 Writes outside writable_roots will be blocked. Network access is \
                 restricted to declared endpoints."
            }
            SandboxMode::Disabled => {
                "Sandbox mode: disabled. No filesystem or network isolation is \
                 active. All operations rely on approval policy for safety."
            }
            SandboxMode::External => {
                "Sandbox mode: external. Operations are delegated to an external \
                 sandbox provider. Writable_roots and denied_reads are enforced \
                 by the external sandbox."
            }
        }
    }
}

/// 从 PermissionProfile 转换为 SandboxMode（用于 fragment 渲染）。
impl From<PermissionProfile> for SandboxMode {
    fn from(profile: PermissionProfile) -> Self {
        match profile {
            PermissionProfile::Managed => SandboxMode::Managed,
            PermissionProfile::Disabled => SandboxMode::Disabled,
            PermissionProfile::External => SandboxMode::External,
        }
    }
}

/// 权限指令 fragment。
///
/// 由 Security Kernel 的当前会话状态构建，注入到 developer role 消息中，
/// 让模型了解当前可执行的操作边界。
pub struct PermissionsInstructionsFragment {
    sandbox_mode: SandboxMode,
    ask_for_approval: AskForApproval,
    cwd: PathBuf,
    writable_roots: Vec<PathBuf>,
    denied_reads: Vec<PathBuf>,
    approved_command_prefixes: Vec<Vec<String>>,
}

impl PermissionsInstructionsFragment {
    /// 用最小必要参数构造（无 writable_roots / denied_reads / approved_command_prefixes）。
    pub fn new(
        sandbox_mode: SandboxMode,
        ask_for_approval: AskForApproval,
        cwd: PathBuf,
    ) -> Self {
        Self {
            sandbox_mode,
            ask_for_approval,
            cwd,
            writable_roots: Vec::new(),
            denied_reads: Vec::new(),
            approved_command_prefixes: Vec::new(),
        }
    }

    /// 从 PermissionProfile 构造（便捷方法，自动转换为 SandboxMode）。
    pub fn from_permission_profile(
        profile: PermissionProfile,
        ask_for_approval: AskForApproval,
        cwd: PathBuf,
    ) -> Self {
        Self::new(SandboxMode::from(profile), ask_for_approval, cwd)
    }

    /// 设置允许写入的根路径。
    pub fn with_writable_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.writable_roots = roots;
        self
    }

    /// 设置禁止读取的路径。
    pub fn with_denied_reads(mut self, reads: Vec<PathBuf>) -> Self {
        self.denied_reads = reads;
        self
    }

    /// 设置免审批的命令前缀（如 `[["git", "status"], ["ls"]]`）。
    pub fn with_approved_command_prefixes(mut self, prefixes: Vec<Vec<String>>) -> Self {
        self.approved_command_prefixes = prefixes;
        self
    }

    /// 当前工作目录。
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// 渲染路径列表为 bullet list。
    fn render_paths(header: &str, paths: &[PathBuf]) -> String {
        if paths.is_empty() {
            return format!("{}: (none)", header);
        }
        let mut out = format!("{}:", header);
        for p in paths {
            out.push_str("\n  - ");
            out.push_str(&p.display().to_string());
        }
        out
    }

    /// 渲染命令前缀列表。
    fn render_prefixes(prefixes: &[Vec<String>]) -> String {
        if prefixes.is_empty() {
            return "Approved command prefixes: (none)".to_string();
        }
        let mut out = "Approved command prefixes:".to_string();
        for prefix in prefixes {
            out.push_str("\n  - ");
            out.push_str(&prefix.join(" "));
        }
        out
    }
}

impl ContextualUserFragment for PermissionsInstructionsFragment {
    fn name(&self) -> &str {
        "permissions_instructions"
    }

    fn render_full(&self) -> Result<String, AppError> {
        let mut sections: Vec<String> = Vec::with_capacity(6);

        sections.push(format!(
            "Working directory: {}",
            self.cwd.display()
        ));
        sections.push(self.sandbox_mode.render_template().to_string());
        sections.push(self.ask_for_approval.render_template().to_string());
        sections.push(Self::render_paths("Writable roots", &self.writable_roots));
        sections.push(Self::render_paths("Denied reads", &self.denied_reads));
        sections.push(Self::render_prefixes(&self.approved_command_prefixes));

        Ok(format!(
            "<permissions instructions>\n{}\n</permissions instructions>",
            sections.join("\n\n")
        ))
    }

    fn bounded_size(&self) -> Option<usize> {
        Some(2048)
    }

    fn role(&self) -> FragmentRole {
        FragmentRole::Developer
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fragment() -> PermissionsInstructionsFragment {
        PermissionsInstructionsFragment::new(
            SandboxMode::Managed,
            AskForApproval::OnRequest,
            PathBuf::from("/workspace/project"),
        )
        .with_writable_roots(vec![PathBuf::from("/workspace/project/src")])
        .with_denied_reads(vec![PathBuf::from("/etc/secrets")])
        .with_approved_command_prefixes(vec![
            vec!["git".to_string(), "status".to_string()],
            vec!["ls".to_string()],
        ])
    }

    #[test]
    fn render_full_includes_all_sections() {
        let frag = sample_fragment();
        let rendered = frag.render_full().unwrap();

        assert!(rendered.contains("<permissions instructions>"));
        assert!(rendered.contains("</permissions instructions>"));
        assert!(rendered.contains("Working directory: /workspace/project"));
        assert!(rendered.contains("Sandbox mode: managed"));
        assert!(rendered.contains("Approval policy: on_request"));
        assert!(rendered.contains("Writable roots:"));
        assert!(rendered.contains("/workspace/project/src"));
        assert!(rendered.contains("Denied reads:"));
        assert!(rendered.contains("/etc/secrets"));
        assert!(rendered.contains("Approved command prefixes:"));
        assert!(rendered.contains("git status"));
        assert!(rendered.contains("ls"));
    }

    #[test]
    fn fragment_metadata() {
        let frag = sample_fragment();
        assert_eq!(frag.name(), "permissions_instructions");
        assert_eq!(frag.bounded_size(), Some(2048));
        assert_eq!(frag.role(), FragmentRole::Developer);
    }

    #[test]
    fn empty_lists_render_as_none() {
        let frag = PermissionsInstructionsFragment::new(
            SandboxMode::Disabled,
            AskForApproval::Never,
            PathBuf::from("/"),
        );

        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("Writable roots: (none)"));
        assert!(rendered.contains("Denied reads: (none)"));
        assert!(rendered.contains("Approved command prefixes: (none)"));
    }

    #[test]
    fn sandbox_mode_templates_differ() {
        let managed = SandboxMode::Managed.render_template();
        let disabled = SandboxMode::Disabled.render_template();
        let external = SandboxMode::External.render_template();

        assert!(managed.contains("managed"));
        assert!(disabled.contains("disabled"));
        assert!(external.contains("external"));
        assert_ne!(managed, disabled);
        assert_ne!(managed, external);
        assert_ne!(disabled, external);
    }

    #[test]
    fn approval_policy_templates_differ() {
        let never = AskForApproval::Never.render_template();
        let on_failure = AskForApproval::OnFailure.render_template();
        let on_request = AskForApproval::OnRequest.render_template();
        let unless_trusted = AskForApproval::UnlessTrusted.render_template();
        let granular = AskForApproval::Granular.render_template();

        assert!(never.contains("never"));
        assert!(on_failure.contains("on_failure"));
        assert!(on_request.contains("on_request"));
        assert!(unless_trusted.contains("unless_trusted"));
        assert!(granular.contains("granular"));
    }

    #[test]
    fn render_diff_returns_none() {
        let a = sample_fragment();
        let b = sample_fragment();
        // permissions_instructions 不支持 diff（每会话固定，变更时重建）
        assert!(a.render_diff(&b).is_none());
    }

    #[test]
    fn from_permission_profile_converts_correctly() {
        assert_eq!(
            SandboxMode::from(PermissionProfile::Managed),
            SandboxMode::Managed
        );
        assert_eq!(
            SandboxMode::from(PermissionProfile::Disabled),
            SandboxMode::Disabled
        );
        assert_eq!(
            SandboxMode::from(PermissionProfile::External),
            SandboxMode::External
        );
    }

    #[test]
    fn from_permission_profile_constructor_works() {
        let frag = PermissionsInstructionsFragment::from_permission_profile(
            PermissionProfile::Managed,
            AskForApproval::OnFailure,
            PathBuf::from("/workspace"),
        );
        let rendered = frag.render_full().unwrap();
        assert!(rendered.contains("Sandbox mode: managed"));
        assert!(rendered.contains("Approval policy: on_failure"));
    }

    #[test]
    fn fragment_injects_into_registry() {
        // 集成测试：fragment 注册到 FragmentRegistry 并渲染到 system prompt
        use super::super::super::fragment::FragmentRegistry;

        let mut registry = FragmentRegistry::new();
        registry
            .register(Box::new(
                PermissionsInstructionsFragment::new(
                    SandboxMode::Managed,
                    AskForApproval::OnRequest,
                    PathBuf::from("/workspace/project"),
                )
                .with_writable_roots(vec![PathBuf::from("/workspace/project/src")]),
            ))
            .unwrap();

        let rendered = registry.render_all().unwrap();
        assert!(rendered.contains("<permissions instructions>"));
        assert!(rendered.contains("</permissions instructions>"));
        assert!(rendered.contains("Working directory: /workspace/project"));
        assert!(rendered.contains("Sandbox mode: managed"));
        assert!(rendered.contains("Approval policy: on_request"));
        assert!(rendered.contains("/workspace/project/src"));
    }
}
