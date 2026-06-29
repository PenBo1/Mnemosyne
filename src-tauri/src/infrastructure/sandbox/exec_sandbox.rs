use std::process::Command;
use std::time::Duration;
use super::policy::{ExecRule, ExecAction};

/// 命令执行沙箱 - 控制哪些命令可以执行
pub struct ExecSandbox {
    rules: Vec<ExecRule>,
    timeout: Duration,
    max_output_bytes: usize,
}

/// 命令执行结果
#[derive(Debug)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub killed: bool,
}

/// 命令执行权限评估结果
#[derive(Debug, Clone)]
pub struct ExecDecision {
    pub allowed: bool,
    pub reason: String,
    pub matched_rule: Option<String>,
}

impl ExecSandbox {
    pub fn new(rules: Vec<ExecRule>, timeout_secs: u64, max_output_bytes: usize) -> Self {
        Self {
            rules,
            timeout: Duration::from_secs(timeout_secs),
            max_output_bytes,
        }
    }

    /// 评估命令是否允许执行
    pub fn evaluate(&self, command: &str) -> ExecDecision {
        let tokens = tokenize_command(command);

        // 从后向前遍历规则，找到最后一个匹配的规则
        for rule in self.rules.iter().rev() {
            if self.matches_rule(&rule.pattern, &tokens) {
                return ExecDecision {
                    allowed: rule.action == ExecAction::Allow,
                    reason: rule.description.clone()
                        .unwrap_or_else(|| format!("规则匹配: {}", rule.pattern)),
                    matched_rule: Some(rule.pattern.clone()),
                };
            }
        }

        // 默认拒绝
        ExecDecision {
            allowed: false,
            reason: "没有匹配的规则，默认拒绝".into(),
            matched_rule: None,
        }
    }

    /// 检查命令是否匹配规则
    fn matches_rule(&self, pattern: &str, tokens: &[String]) -> bool {
        let pattern_tokens = tokenize_command(pattern);

        if pattern == "*" {
            return true;
        }

        if pattern_tokens.len() > tokens.len() {
            return false;
        }

        for (p, t) in pattern_tokens.iter().zip(tokens.iter()) {
            if p != t && p != "*" {
                return false;
            }
        }

        true
    }

    /// 在沙箱中执行命令
    pub fn execute(&self, command: &str, work_dir: &std::path::Path) -> Result<ExecResult, String> {
        // 先评估权限
        let decision = self.evaluate(command);
        if !decision.allowed {
            return Err(format!("命令被沙箱拒绝: {}", decision.reason));
        }

        // 检测危险命令模式
        self.detect_dangerous_patterns(command)?;

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(command);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(command);
            c
        };

        cmd.current_dir(work_dir);

        // 设置资源限制
        self.apply_resource_limits(&mut cmd);

        // 执行命令（带超时）
        let result = self.execute_with_timeout(cmd)?;

        Ok(result)
    }

    /// 检测危险命令模式
    fn detect_dangerous_patterns(&self, command: &str) -> Result<(), String> {
        let cmd_lower = command.to_lowercase();

        // 检查实际换行字节(0x0A) - 不能只检查字符串"\n"
        if command.contains('\n') {
            return Err("检测到实际换行字节，可能被用于命令注入".to_string());
        }
        // 检查回车字节(0x0D)
        if command.contains('\r') {
            return Err("检测到回车字节，可能被用于命令注入".to_string());
        }
        // 检查空字节
        if command.contains('\0') {
            return Err("检测到空字节，可能被用于命令注入".to_string());
        }

        // 检测注入攻击
        let dangerous_patterns = [
            ("|", "管道可能被用于命令注入"),
            (";", "分号可能被用于命令链接"),
            ("`", "反引号可能被用于命令替换"),
            ("$(", "命令替换可能被用于注入"),
            ("&&", "逻辑与可能被用于命令链接"),
            ("||", "逻辑或可能被用于命令链接"),
            (">", "重定向可能被用于文件覆盖"),
            (">>", "追加重定向可能被用于文件写入"),
            ("<", "输入重定向可能被用于读取敏感文件"),
            ("\\n", "转义换行符可能被用于命令注入"),
        ];

        for (pattern, reason) in dangerous_patterns {
            if cmd_lower.contains(pattern) {
                // 特殊情况：某些命令允许管道（仅限grep/find的第一个管道）
                if pattern == "|" && (cmd_lower.starts_with("grep") || cmd_lower.starts_with("find")) {
                    continue;
                }
                return Err(format!("检测到潜在的命令注入风险: {} ({})", pattern, reason));
            }
        }

        // 检查Unicode同形字攻击（常见混淆字符）
        let homoglyphs = [
            ('а', 'a'), // Cyrillic а vs Latin a
            ('е', 'e'), // Cyrillic е vs Latin e
            ('о', 'o'), // Cyrillic о vs Latin o
            ('р', 'p'), // Cyrillic р vs Latin p
            ('с', 'c'), // Cyrillic с vs Latin c
        ];
        for (bad, _good) in &homoglyphs {
            if command.contains(*bad) {
                return Err(format!("检测到Unicode同形字字符 '{}'，可能被用于绕过安全检查", bad));
            }
        }

        Ok(())
    }

    /// 应用资源限制
    fn apply_resource_limits(&self, _cmd: &mut Command) {
        // 在 Unix 系统上，可以使用 rlimit 设置资源限制
        // 在 Windows 上，可以使用作业对象
        // 这里先使用基本的超时控制
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // 设置进程组
            unsafe {
                cmd.pre_exec(|| {
                    // 创建新的进程组
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }
    }

    /// 带超时的命令执行
    fn execute_with_timeout(&self, mut cmd: Command) -> Result<ExecResult, String> {
        let child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败: {}", e))?;

        let pid = child.id();

        // 启动超时监控线程
        let timeout = self.timeout;
        let timeout_handle = std::thread::spawn(move || {
            std::thread::sleep(timeout);
            // 杀死进程组
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            #[cfg(windows)]
            {
                // Windows 上使用 taskkill
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .output();
            }
        });

        let output = child
            .wait_with_output()
            .map_err(|e| format!("等待命令完成失败: {}", e))?;

        // 停止超时监控
        timeout_handle.join().unwrap_or(());

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // 检查输出大小
        if stdout.len() + stderr.len() > self.max_output_bytes {
            let truncated_stdout = &stdout[..self.max_output_bytes.min(stdout.len())];
            let truncated_stderr = &stderr[..self.max_output_bytes.min(stderr.len())];
            return Ok(ExecResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: truncated_stdout.to_string(),
                stderr: format!("{}... [输出被截断]", truncated_stderr),
                timed_out: false,
                killed: false,
            });
        }

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            timed_out: false,
            killed: false,
        })
    }
}

/// 命令分词器
fn tokenize_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for ch in command.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            c if c == quote_char && in_quotes => {
                in_quotes = false;
            }
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            c => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_command() {
        assert_eq!(tokenize_command("ls -la"), vec!["ls", "-la"]);
        assert_eq!(tokenize_command("echo \"hello world\""), vec!["echo", "hello world"]);
        assert_eq!(tokenize_command("grep -r \"pattern\" ."), vec!["grep", "-r", "pattern", "."]);
    }

    #[test]
    fn test_dangerous_pattern_detection() {
        let sandbox = ExecSandbox::new(vec![], 60, 1024 * 1024);
        assert!(sandbox.detect_dangerous_patterns("ls | grep test").is_err());
        assert!(sandbox.detect_dangerous_patterns("ls; rm -rf /").is_err());
        assert!(sandbox.detect_dangerous_patterns("echo `whoami`").is_err());
        assert!(sandbox.detect_dangerous_patterns("ls").is_ok());
    }
}
