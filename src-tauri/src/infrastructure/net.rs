
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

use tauri::State;

use crate::shared::error::{AppError, IpcResponse};
use crate::security_kernel::{
    SecurityKernelState, OperationContext,
    WorkspaceId, UserId, SessionId,
};
use crate::security_kernel::permission::{Operation, NetworkScope, NetworkEndpoint};

// ── IP classification ──────────────────────────────────────

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum IpKind {
    Public,
    Private,
    Loopback,
    BlockedMetadata,
}

fn ip_kind(ip: IpAddr) -> IpKind {
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            if v.is_link_local() {
                return IpKind::BlockedMetadata;
            }
            if v.is_loopback() || v.is_unspecified() || v.is_broadcast() || v.is_multicast() {
                return IpKind::Loopback;
            }
            if o[0] == 10
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168)
                || (o[0] == 100 && (64..=127).contains(&o[1]))
                || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
            {
                return IpKind::Private;
            }
            IpKind::Public
        }
        IpAddr::V6(v) => {
            if v.is_loopback() || v.is_unspecified() || v.is_multicast() {
                return IpKind::Loopback;
            }
            let segs = v.segments();
            if segs[0] == 0xfd00 && segs[1] == 0xec2 {
                return IpKind::BlockedMetadata;
            }
            if segs[0] & 0xffc0 == 0xfe80 {
                return IpKind::BlockedMetadata;
            }
            if segs[0] & 0xfe00 == 0xfc00 {
                return IpKind::Private;
            }
            IpKind::Public
        }
    }
}

async fn resolve_and_classify(host: &str) -> Result<(IpKind, Vec<IpAddr>), AppError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok((ip_kind(ip), vec![ip]));
    }
    let host_owned = host.to_string();
    let lookup = tokio::task::spawn_blocking(move || {
        (host_owned.as_str(), 0u16)
            .to_socket_addrs()
            .map(|it| it.map(|a| a.ip()).collect::<Vec<_>>())
    })
    .await
    .map_err(|e| AppError::internal(format!("DNS task join failed: {}", e)))?
    .map_err(|e| AppError::dns_failed(format!("{}: {}", host, e)))?;
    if lookup.is_empty() {
        return Err(AppError::dns_failed(format!("{}: no addresses", host)));
    }
    let mut worst = IpKind::Public;
    for ip in &lookup {
        let k = ip_kind(*ip);
        worst = match (worst, k) {
            (_, IpKind::BlockedMetadata) => IpKind::BlockedMetadata,
            (IpKind::BlockedMetadata, _) => IpKind::BlockedMetadata,
            (IpKind::Public, x) => x,
            (x, IpKind::Public) => x,
            (a, _) => a,
        };
    }
    Ok((worst, lookup))
}

fn is_blocked_host_name(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    matches!(
        host.as_str(),
        "metadata.google.internal" | "metadata" | "metadata.azure.com"
    )
}

fn validate_url(url: &str, _allow_private: bool) -> Result<reqwest::Url, AppError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| {
        AppError::invalid_input(format!("invalid url: {}", e))
    })?;
    match parsed.scheme() {
        "http" | "https" => {}
        s => return Err(AppError::invalid_input(format!("scheme not allowed: {}", s))),
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return Err(AppError::invalid_input("userinfo in url is not allowed"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::invalid_input("missing host"))?;
    if is_blocked_host_name(host) {
        return Err(AppError::sandbox_violation(format!("host not allowed: {}", host)));
    }
    Ok(parsed)
}

async fn classify_and_collect_safe_ips(
    host: &str,
    allow_private: bool,
) -> Result<Vec<IpAddr>, AppError> {
    let (worst, ips) = resolve_and_classify(host).await?;
    match worst {
        IpKind::BlockedMetadata => {
            return Err(AppError::sandbox_violation(format!("host not allowed: {}", host)))
        }
        IpKind::Loopback | IpKind::Private if !allow_private => {
            return Err(AppError::sandbox_violation(format!(
                "host {} resolves to a private/loopback address; this endpoint requires explicit opt-in",
                host
            )));
        }
        _ => {}
    }
    let safe: Vec<IpAddr> = ips
        .into_iter()
        .filter(|ip| match ip_kind(*ip) {
            IpKind::BlockedMetadata => false,
            IpKind::Loopback | IpKind::Private => allow_private,
            IpKind::Public => true,
        })
        .collect();
    if safe.is_empty() {
        return Err(AppError::sandbox_violation(format!("host {}: no safe IPs", host)));
    }
    Ok(safe)
}

// ── IPC commands ───────────────────────────────────────────

fn create_operation_context() -> OperationContext {
    OperationContext {
        workspace: WorkspaceId(uuid::Uuid::nil()),
        user: UserId(uuid::Uuid::nil()),
        session: SessionId(uuid::Uuid::nil()),
        approval_token: None,
    }
}

/// Probe local model service (GET <base>/models, return HTTP status).
#[tauri::command]
pub async fn lm_ping(
    base_url: String,
    kernel_state: State<'_, SecurityKernelState>,
) -> Result<IpcResponse<u16>, AppError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("empty base url"));
    }
    let probe = format!("{}/models", trimmed);
    let parsed = validate_url(&probe, true)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::invalid_input("missing host"))?
        .to_string();
    let safe_ips = classify_and_collect_safe_ips(&host, true).await?;

    let ctx = create_operation_context();
    let endpoint = NetworkEndpoint::https(host.clone());
    let op = Operation::Network {
        scope: NetworkScope::provider(endpoint),
        endpoint: probe.clone(),
        method: "GET".to_string(),
    };

    let kernel = kernel_state.kernel();
    
    let status = kernel.execute("lm_ping", &op, &ctx, || {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut builder = reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .redirect(reqwest::redirect::Policy::none());
                let addrs: Vec<SocketAddr> = safe_ips.iter().map(|ip| SocketAddr::new(*ip, 0)).collect();
                builder = builder.resolve_to_addrs(&host, &addrs);
                let client = builder
                    .build()
                    .map_err(|e| AppError::internal(format!("failed to build ping client: {}", e)))?;
                let status = client
                    .get(parsed)
                    .send()
                    .await
                    .map(|r| r.status().as_u16())
                    .map_err(|e| AppError::internal(format!("ping failed: {}", e)))?;
                Ok(status)
            })
        })
    })?;

    Ok(IpcResponse::ok(status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn metadata_ips_classified_as_blocked() {
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))),
            IpKind::BlockedMetadata
        );
        assert_eq!(
            ip_kind("fd00:ec2::254".parse().unwrap()),
            IpKind::BlockedMetadata
        );
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))),
            IpKind::BlockedMetadata
        );
        assert_eq!(
            ip_kind("fe80::1".parse().unwrap()),
            IpKind::BlockedMetadata
        );
    }

    #[test]
    fn private_ips_classified_correctly() {
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            IpKind::Private
        );
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))),
            IpKind::Private
        );
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            IpKind::Private
        );
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))),
            IpKind::Private
        );
    }

    #[test]
    fn loopback_classified_as_loopback() {
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            IpKind::Loopback
        );
        assert_eq!(ip_kind("::1".parse().unwrap()), IpKind::Loopback);
    }

    #[test]
    fn public_ips_classified_as_public() {
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
            IpKind::Public
        );
        assert_eq!(
            ip_kind(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))),
            IpKind::Public
        );
    }

    #[test]
    fn validate_url_blocks_userinfo_and_metadata_hostnames() {
        assert!(validate_url("http://user:pass@example.com/", true).is_err());
        assert!(validate_url("http://metadata.google.internal/", true).is_err());
        assert!(validate_url("http://metadata/", true).is_err());
        assert!(validate_url("http://metadata.azure.com/", true).is_err());
    }

    #[test]
    fn validate_url_rejects_non_http_schemes() {
        assert!(validate_url("ftp://example.com/", true).is_err());
        assert!(validate_url("file:///etc/passwd", true).is_err());
        assert!(validate_url("javascript:alert(1)", true).is_err());
    }
}
