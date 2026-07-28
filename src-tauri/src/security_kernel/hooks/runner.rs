//! ═══════════════════════════════════════════════════════════════════════════
//! runner - Hook 执行器模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use url::Url;

use crate::shared::error::AppError;

use super::types::{HookHandler, HookPayload, HookResult, HookSpec};

// ── Hook 执行器 trait ────────────────────────────────────────────────────────

/// Hook 执行器 trait —— 定义统一的 hook 执行接口。
#[async_trait]
pub trait HookRunner: Send + Sync {
    /// 执行 hook，返回执行结果。
    async fn run(&self, spec: &HookSpec, payload: &HookPayload) -> HookResult;
}

// ── Command Runner ────────────────────────────────────────────────────────────────

/// Command Runner —— 执行 shell 命令。
pub struct CommandRunner;

#[async_trait]
impl HookRunner for CommandRunner {
    async fn run(&self, spec: &HookSpec, payload: &HookPayload) -> HookResult {
        let (cmd, timeout_ms) = match &spec.handler {
            HookHandler::Command { cmd } => (cmd.clone(), spec.timeout),
            _ => {
                tracing::error!("CommandRunner received non-Command handler");
                return HookResult::FailedContinue;
            }
        };

        let env = build_env_vars(payload);
        let result = execute_command(&cmd, env, timeout_ms).await;

        match result {
            Ok(output) => {
                tracing::info!(
                    hook_name = %spec.name,
                    event = ?spec.event,
                    output = %output,
                    "Hook command executed successfully"
                );
                HookResult::Success
            }
            Err(e) => {
                tracing::warn!(
                    hook_name = %spec.name,
                    event = ?spec.event,
                    error = %e,
                    "Hook command failed"
                );
                HookResult::FailedContinue
            }
        }
    }
}

// ── HTTP Runner ────────────────────────────────────────────────────────────────

/// HTTP Runner —— 发送 HTTP 请求（含 SSRF 防护）。
pub struct HttpRunner;

#[async_trait]
impl HookRunner for HttpRunner {
    async fn run(&self, spec: &HookSpec, payload: &HookPayload) -> HookResult {
        let (url, method, headers, timeout_ms) = match &spec.handler {
            HookHandler::Http {
                url,
                method,
                headers,
                timeout_ms,
            } => (url.clone(), method.clone(), headers.clone(), *timeout_ms),
            _ => {
                tracing::error!("HttpRunner received non-Http handler");
                return HookResult::FailedContinue;
            }
        };

        if let Err(e) = validate_url_for_ssrf(&url) {
            tracing::warn!(
                hook_name = %spec.name,
                url = %url,
                error = %e,
                "SSRF protection blocked HTTP hook"
            );
            return HookResult::FailedAbort;
        }

        let body = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let result = execute_http_request(&url, &method, &headers, &body, timeout_ms).await;

        match result {
            Ok(response) => {
                tracing::info!(
                    hook_name = %spec.name,
                    event = ?spec.event,
                    url = %url,
                    status = response.status().as_u16(),
                    "Hook HTTP request completed"
                );
                HookResult::Success
            }
            Err(e) => {
                tracing::warn!(
                    hook_name = %spec.name,
                    event = ?spec.event,
                    url = %url,
                    error = %e,
                    "Hook HTTP request failed"
                );
                HookResult::FailedContinue
            }
        }
    }
}

// ── SSRF 防护 ────────────────────────────────────────────────────────────────

/// SSRF 防护 —— 验证 URL 是否安全。
pub fn validate_url_for_ssrf(url_str: &str) -> Result<(), AppError> {
    let url = Url::parse(url_str)
        .map_err(|e| AppError::invalid_input(format!("Invalid URL '{}': {}", url_str, e)))?;

    if url.scheme() != "https" {
        return Err(AppError::invalid_input(format!(
            "SSRF protection: only HTTPS allowed, got '{}'",
            url.scheme()
        )));
    }

    match url.host() {
        Some(url::Host::Ipv6(ip)) => {
            if is_private_ip(IpAddr::V6(ip)) {
                return Err(AppError::invalid_input(format!(
                    "SSRF protection: private IPv6 address '{}' blocked",
                    ip
                )));
            }
        }
        Some(url::Host::Ipv4(ip)) => {
            if is_private_ip(IpAddr::V4(ip)) {
                return Err(AppError::invalid_input(format!(
                    "SSRF protection: private IPv4 address '{}' blocked",
                    ip
                )));
            }
        }
        Some(url::Host::Domain(host)) => {
            if host == "localhost" || host == "localhost.localdomain" {
                return Err(AppError::invalid_input(
                    "SSRF protection: localhost hostname blocked",
                ));
            }
            if host.ends_with(".local")
                || host.ends_with(".internal")
                || host.ends_with(".localhost")
            {
                return Err(AppError::invalid_input(format!(
                    "SSRF protection: internal hostname '{}' blocked",
                    host
                )));
            }
        }
        None => {
            return Err(AppError::invalid_input("URL missing host"));
        }
    }

    Ok(())
}

/// 检查 IP 是否为私有地址。
fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => is_private_ipv4(ipv4),
        IpAddr::V6(ipv6) => is_private_ipv6(ipv6),
    }
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();

    if octets[0] == 10 {
        return true;
    }

    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return true;
    }

    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }

    if octets[0] == 127 {
        return true;
    }

    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }

    if octets[0] == 0 {
        return true;
    }

    if octets[0] >= 224 {
        return true;
    }

    false
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() {
        return true;
    }

    let segments = ip.segments();
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }

    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }

    false
}

// ── 执行辅助函数 ────────────────────────────────────────────────────────────────

/// 构建 hook payload 的环境变量映射。
fn build_env_vars(payload: &HookPayload) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("HOOK_EVENT".to_string(), payload.event.as_str().to_string());
    if let Some(tool) = &payload.tool_name {
        env.insert("HOOK_TOOL_NAME".to_string(), tool.clone());
    }
    if let Some(session) = &payload.session_id {
        env.insert("HOOK_SESSION_ID".to_string(), session.clone());
    }
    if let Some(ws) = &payload.workspace_id {
        env.insert("HOOK_WORKSPACE_ID".to_string(), ws.clone());
    }
    if let Some(role) = &payload.agent_role {
        env.insert("HOOK_AGENT_ROLE".to_string(), role.clone());
    }
    env.insert(
        "HOOK_TIMESTAMP".to_string(),
        payload.timestamp.to_rfc3339(),
    );
    if let Ok(json) = serde_json::to_string(payload) {
        env.insert("HOOK_PAYLOAD_JSON".to_string(), json);
    }
    env
}

/// 执行 shell 命令（带超时）。
async fn execute_command(
    cmd: &str,
    _env: HashMap<String, String>,
    timeout_ms: u64,
) -> Result<String, AppError> {
    let cmd = cmd.to_string();
    let timeout_dur = Duration::from_millis(timeout_ms);

    let output = timeout(timeout_dur, async {
        let mut child = TokioCommand::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::internal(format!("Failed to spawn command: {}", e)))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => stdout_lines.push(l),
                        Ok(None) => break,
                        Err(e) => {
                            tracing::warn!("Error reading stdout: {}", e);
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => stderr_lines.push(l),
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!("Error reading stderr: {}", e);
                        }
                    }
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| AppError::internal(format!("Failed to wait for command: {}", e)))?;

        if !status.success() {
            let stderr_output = stderr_lines.join("\n");
            return Err(AppError::internal(format!(
                "Command failed with status {}: {}",
                status, stderr_output
            )));
        }

        Ok(stdout_lines.join("\n"))
    })
    .await
    .map_err(|_| AppError::internal(format!("Command timed out after {}ms", timeout_ms)))??;

    Ok(output)
}

/// 执行 HTTP 请求（带超时）。
async fn execute_http_request(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: &str,
    timeout_ms: u64,
) -> Result<reqwest::Response, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| AppError::internal(format!("Failed to build HTTP client: {}", e)))?;

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        _ => client.post(url),
    };

    for (key, value) in headers {
        request = request.header(key, value);
    }

    if method != "GET" {
        request = request.header("Content-Type", "application/json").body(body.to_string());
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::internal(format!("HTTP request failed: {}", e)))?;

    Ok(response)
}

// ── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssrf_protection_localhost() {
        assert!(validate_url_for_ssrf("http://localhost:8080/hook").is_err());
        assert!(validate_url_for_ssrf("https://localhost/hook").is_err());
    }

    #[test]
    fn test_ssrf_protection_private_ipv4() {
        assert!(validate_url_for_ssrf("https://10.0.0.1/hook").is_err());
        assert!(validate_url_for_ssrf("https://172.16.0.1/hook").is_err());
        assert!(validate_url_for_ssrf("https://192.168.1.1/hook").is_err());
        assert!(validate_url_for_ssrf("https://127.0.0.1/hook").is_err());
    }

    #[test]
    fn test_ssrf_protection_private_ipv6() {
        assert!(validate_url_for_ssrf("https://[::1]/hook").is_err());
        assert!(validate_url_for_ssrf("https://[fc00::1]/hook").is_err());
        assert!(validate_url_for_ssrf("https://[fe80::1]/hook").is_err());
    }

    #[test]
    fn test_ssrf_protection_http_blocked() {
        assert!(validate_url_for_ssrf("http://example.com/hook").is_err());
    }

    #[test]
    fn test_ssrf_protection_valid_https() {
        assert!(validate_url_for_ssrf("https://api.example.com/hook").is_ok());
    }
}