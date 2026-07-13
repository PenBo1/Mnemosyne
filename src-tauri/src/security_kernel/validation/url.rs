use url::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use crate::shared::error::AppError;

#[allow(dead_code)]
const ALLOWED_SCHEMES: [&str; 2] = ["https", "http"];

const PRIVATE_IPV4_RANGES: [(Ipv4Addr, Ipv4Addr); 5] = [
    (Ipv4Addr::new(10, 0, 0, 0), Ipv4Addr::new(10, 255, 255, 255)),
    (Ipv4Addr::new(172, 16, 0, 0), Ipv4Addr::new(172, 31, 255, 255)),
    (Ipv4Addr::new(192, 168, 0, 0), Ipv4Addr::new(192, 168, 255, 255)),
    (Ipv4Addr::new(127, 0, 0, 0), Ipv4Addr::new(127, 255, 255, 255)),
    (Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(0, 255, 255, 255)),
];

const LOOPBACK_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

#[derive(Debug, Clone)]
pub struct UrlValidationConfig {
    pub allowed_schemes: Vec<String>,
    pub allow_private_ips: bool,
    pub allow_localhost: bool,
    pub dns_pinned_ips: Vec<IpAddr>,
}

impl Default for UrlValidationConfig {
    fn default() -> Self {
        Self {
            allowed_schemes: vec!["https".to_string()],
            allow_private_ips: false,
            allow_localhost: false,
            dns_pinned_ips: vec![],
        }
    }
}

pub fn validate_url(url: &Url, config: &UrlValidationConfig) -> Result<(), AppError> {
    let scheme = url.scheme();
    if !config.allowed_schemes.iter().any(|s| s == scheme) {
        return Err(AppError::invalid_input(format!(
            "URL scheme '{}' is not allowed. Allowed schemes: {}",
            scheme,
            config.allowed_schemes.join(", ")
        )));
    }

    let host = url.host_str();
    if host.is_none() {
        return Err(AppError::invalid_input("URL has no host"));
    }

    let host = host.unwrap();

    if let Ok(ip) = host.parse::<IpAddr>() {
        validate_ip_address(&ip, config)?;
    } else {
        if !config.allow_localhost && (host == "localhost" || host == "local") {
            return Err(AppError::forbidden("localhost is not allowed"));
        }

        if config.dns_pinned_ips.is_empty() {
            tracing::warn!(
                host = host,
                "DNS resolution validation not yet implemented - host will be resolved at runtime"
            );
        }
    }

    let port = url.port();
    if let Some(p) = port {
        if p == 0 {
            return Err(AppError::invalid_input("Port 0 is not valid"));
        }
    }

    Ok(())
}

fn validate_ip_address(ip: &IpAddr, config: &UrlValidationConfig) -> Result<(), AppError> {
    if config.dns_pinned_ips.contains(ip) {
        return Ok(());
    }

    if is_private_ip(ip) && !config.allow_private_ips {
        return Err(AppError::forbidden(format!(
            "Private IP address {} is not allowed",
            ip
        )));
    }

    if is_loopback_ip(ip) && !config.allow_localhost {
        return Err(AppError::forbidden(format!(
            "Loopback IP address {} is not allowed",
            ip
        )));
    }

    Ok(())
}

pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            for (start, end) in PRIVATE_IPV4_RANGES {
                if u32_from_ipv4(*v4) >= u32_from_ipv4(start)
                    && u32_from_ipv4(*v4) <= u32_from_ipv4(end)
                {
                    return true;
                }
            }
            false
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || is_unique_local_ipv6(*v6)
        }
    }
}

pub fn is_loopback_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => *v6 == LOOPBACK_IPV6 || v6.is_loopback(),
    }
}

fn u32_from_ipv4(ip: Ipv4Addr) -> u32 {
    u32::from_be_bytes(ip.octets())
}

fn is_unique_local_ipv6(ip: Ipv6Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 0xfc || octets[0] == 0xfd
}

pub fn parse_and_validate_url(url_str: &str, config: &UrlValidationConfig) -> Result<Url, AppError> {
    let url = Url::parse(url_str).map_err(|e| {
        AppError::invalid_format(format!("Invalid URL format: {}", e))
    })?;

    validate_url(&url, config)?;

    Ok(url)
}

pub fn validate_url_for_ssrf(url: &Url) -> Result<(), AppError> {
    let config = UrlValidationConfig {
        allowed_schemes: vec!["https".to_string(), "http".to_string()],
        allow_private_ips: false,
        allow_localhost: false,
        dns_pinned_ips: vec![],
    };

    validate_url(url, &config)
}

pub fn check_dns_pinning(_host: &str) -> Result<Vec<IpAddr>, AppError> {
    tracing::warn!("DNS pinning not yet implemented - returning empty IP list");

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_https_allowed() {
        let url = Url::parse("https://example.com/path").unwrap();
        let config = UrlValidationConfig::default();
        assert!(validate_url(&url, &config).is_ok());
    }

    #[test]
    fn test_validate_url_http_blocked_by_default() {
        let url = Url::parse("http://example.com/path").unwrap();
        let config = UrlValidationConfig::default();
        assert!(validate_url(&url, &config).is_err());
    }

    #[test]
    fn test_validate_url_localhost_blocked() {
        let url = Url::parse("https://localhost/path").unwrap();
        let config = UrlValidationConfig::default();
        assert!(validate_url(&url, &config).is_err());
    }

    #[test]
    fn test_validate_url_private_ip_blocked() {
        let url = Url::parse("https://192.168.1.1/path").unwrap();
        let config = UrlValidationConfig::default();
        assert!(validate_url(&url, &config).is_err());
    }

    #[test]
    fn test_is_private_ip() {
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn test_is_loopback_ip() {
        assert!(is_loopback_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_loopback_ip(&IpAddr::V6(LOOPBACK_IPV6)));
        assert!(!is_loopback_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }
}