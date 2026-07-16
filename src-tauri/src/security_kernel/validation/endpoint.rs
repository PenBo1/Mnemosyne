use url::Url;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use crate::shared::error::AppError;
use super::url::{is_private_ip, is_loopback_ip};

#[derive(Debug, Clone)]
pub struct NetworkEndpoint {
    pub allowed_hosts: HashSet<String>,
    pub allowed_schemes: HashSet<String>,
    pub allowed_ports: HashSet<u16>,
    pub allow_any_host: bool,
    pub allow_any_port: bool,
    pub allow_private_ips: bool,
    pub allow_cidr_ranges: Vec<CidrRange>,
}

impl Default for NetworkEndpoint {
    fn default() -> Self {
        Self {
            allowed_hosts: HashSet::new(),
            allowed_schemes: HashSet::from(["https".to_string()]),
            allowed_ports: HashSet::from([443]),
            allow_any_host: false,
            allow_any_port: false,
            allow_private_ips: false,
            allow_cidr_ranges: vec![],
        }
    }
}

impl NetworkEndpoint {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_allowed_hosts(hosts: Vec<String>) -> Self {
        Self {
            allowed_hosts: HashSet::from_iter(hosts),
            allowed_schemes: HashSet::from(["https".to_string()]),
            allowed_ports: HashSet::from([443]),
            allow_any_host: false,
            allow_any_port: false,
            allow_private_ips: false,
            allow_cidr_ranges: vec![],
        }
    }

    pub fn allow_host(mut self, host: String) -> Self {
        self.allowed_hosts.insert(host);
        self
    }

    pub fn allow_scheme(mut self, scheme: String) -> Self {
        self.allowed_schemes.insert(scheme);
        self
    }

    pub fn allow_port(mut self, port: u16) -> Self {
        self.allowed_ports.insert(port);
        self
    }

    pub fn allow_any_host(mut self) -> Self {
        self.allow_any_host = true;
        self
    }

    pub fn allow_any_port(mut self) -> Self {
        self.allow_any_port = true;
        self
    }

    pub fn allow_private_ips(mut self) -> Self {
        self.allow_private_ips = true;
        self
    }

    pub fn allow_cidr(mut self, cidr: CidrRange) -> Self {
        self.allow_cidr_ranges.push(cidr);
        self
    }
}

#[derive(Debug, Clone)]
pub struct CidrRange {
    pub network: IpAddr,
    pub prefix_len: u8,
}

impl CidrRange {
    pub fn new(network: IpAddr, prefix_len: u8) -> Self {
        Self { network, prefix_len }
    }

    pub fn contains(&self, ip: &IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net_v4), IpAddr::V4(ip_v4)) => {
                let net_u32 = u32::from_be_bytes(net_v4.octets());
                let ip_u32 = u32::from_be_bytes(ip_v4.octets());
                let mask = if self.prefix_len == 0 {
                    0u32
                } else {
                    !0u32 << (32 - self.prefix_len)
                };
                (net_u32 & mask) == (ip_u32 & mask)
            }
            (IpAddr::V6(net_v6), IpAddr::V6(ip_v6)) => {
                let net_u128 = u128::from_be_bytes(net_v6.octets());
                let ip_u128 = u128::from_be_bytes(ip_v6.octets());
                let mask = if self.prefix_len == 0 {
                    0u128
                } else {
                    !0u128 << (128 - self.prefix_len)
                };
                (net_u128 & mask) == (ip_u128 & mask)
            }
            _ => false,
        }
    }
}

pub fn validate_endpoint(url: &Url, endpoint: &NetworkEndpoint) -> Result<(), AppError> {
    let scheme = url.scheme();
    if !endpoint.allowed_schemes.contains(scheme) {
        return Err(AppError::forbidden(format!(
            "Scheme '{}' is not allowed. Allowed schemes: {}",
            scheme,
            endpoint.allowed_schemes.iter().cloned().collect::<Vec<_>>().join(", ")
        )));
    }

    let host = url.host_str();
    if host.is_none() {
        return Err(AppError::invalid_input("URL has no host"));
    }

    let host = host.unwrap();

    if !endpoint.allow_any_host {
        if let Ok(ip) = host.parse::<IpAddr>() {
            validate_ip_endpoint(&ip, endpoint)?;
        } else {
            let host_lower = host.to_lowercase();
            if !endpoint.allowed_hosts.contains(&host_lower) {
                return Err(AppError::forbidden(format!(
                    "Host '{}' is not allowed. Allowed hosts: {}",
                    host,
                    endpoint.allowed_hosts.iter().cloned().collect::<Vec<_>>().join(", ")
                )));
            }
        }
    }

    let port = url.port().unwrap_or({
        match scheme {
            "https" => 443,
            "http" => 80,
            _ => 0,
        }
    });

    if !endpoint.allow_any_port && !endpoint.allowed_ports.contains(&port) {
        return Err(AppError::forbidden(format!(
            "Port {} is not allowed. Allowed ports: {}",
            port,
            endpoint.allowed_ports.iter().cloned().collect::<Vec<_>>()
                .into_iter().map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(())
}

fn validate_ip_endpoint(ip: &IpAddr, endpoint: &NetworkEndpoint) -> Result<(), AppError> {
    for cidr in &endpoint.allow_cidr_ranges {
        if cidr.contains(ip) {
            return Ok(());
        }
    }

    if is_private_ip(ip) && !endpoint.allow_private_ips {
        return Err(AppError::forbidden(format!(
            "Private IP address {} is not allowed",
            ip
        )));
    }

    if is_loopback_ip(ip) {
        return Err(AppError::forbidden(format!(
            "Loopback IP address {} is not allowed",
            ip
        )));
    }

    Ok(())
}

pub fn create_ai_endpoint() -> NetworkEndpoint {
    NetworkEndpoint::new()
        .allow_any_host()
        .allow_any_port()
        .allow_scheme("https".to_string())
        .allow_scheme("http".to_string())
}

pub fn create_strict_endpoint(hosts: Vec<String>) -> NetworkEndpoint {
    NetworkEndpoint::with_allowed_hosts(hosts)
        .allow_port(443)
        .allow_scheme("https".to_string())
}

pub fn create_internal_endpoint() -> NetworkEndpoint {
    NetworkEndpoint::new()
        .allow_private_ips()
        .allow_host("localhost".to_string())
        .allow_port(8080)
        .allow_port(3000)
        .allow_scheme("http".to_string())
        .allow_scheme("https".to_string())
        .allow_cidr(CidrRange::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 0)), 8))
        .allow_cidr(CidrRange::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8))
        .allow_cidr(CidrRange::new(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0)), 12))
        .allow_cidr(CidrRange::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0)), 16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_endpoint_scheme() {
        let endpoint = NetworkEndpoint::new();
        let url = Url::parse("http://example.com/").unwrap();
        let result = validate_endpoint(&url, &endpoint);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_endpoint_allowed_host() {
        let endpoint = NetworkEndpoint::with_allowed_hosts(vec!["api.example.com".to_string()]);
        let url = Url::parse("https://api.example.com/").unwrap();
        assert!(validate_endpoint(&url, &endpoint).is_ok());
    }

    #[test]
    fn test_validate_endpoint_blocked_host() {
        let endpoint = NetworkEndpoint::with_allowed_hosts(vec!["api.example.com".to_string()]);
        let url = Url::parse("https://other.example.com/").unwrap();
        let result = validate_endpoint(&url, &endpoint);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_endpoint_port() {
        let endpoint = NetworkEndpoint::new().allow_any_host().allow_port(8080);
        let url = Url::parse("https://example.com:8080/").unwrap();
        assert!(validate_endpoint(&url, &endpoint).is_ok());
    }

    #[test]
    fn test_validate_endpoint_any_host() {
        let endpoint = NetworkEndpoint::new().allow_any_host();
        let url = Url::parse("https://any-random-host.com/").unwrap();
        assert!(validate_endpoint(&url, &endpoint).is_ok());
    }

    #[test]
    fn test_cidr_range_contains() {
        let cidr = CidrRange::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0)), 16);
        assert!(cidr.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))));
        assert!(cidr.contains(&IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
        assert!(!cidr.contains(&IpAddr::V4(Ipv4Addr::new(192, 169, 0, 0))));
        assert!(!cidr.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
    }

    #[test]
    fn test_create_ai_endpoint() {
        let endpoint = create_ai_endpoint();
        let url = Url::parse("https://api.openai.com/v1/chat").unwrap();
        assert!(validate_endpoint(&url, &endpoint).is_ok());
    }
}