//! ═══════════════════════════════════════════════════════════════════════════
//! network_scope - 网络范围定义
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::fmt;

// ── 网络范围 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkScope {
    Provider { endpoint: NetworkEndpoint },
    MCP { endpoint: NetworkEndpoint, allow_localhost: bool },
    Plugin { endpoint: NetworkEndpoint },
}

impl NetworkScope {
    pub fn provider(endpoint: NetworkEndpoint) -> Self {
        Self::Provider { endpoint }
    }

    pub fn mcp(endpoint: NetworkEndpoint, allow_localhost: bool) -> Self {
        Self::MCP { endpoint, allow_localhost }
    }

    pub fn plugin(endpoint: NetworkEndpoint) -> Self {
        Self::Plugin { endpoint }
    }

    pub fn is_provider(&self) -> bool {
        matches!(self, Self::Provider { .. })
    }

    pub fn is_mcp(&self) -> bool {
        matches!(self, Self::MCP { .. })
    }

    pub fn is_plugin(&self) -> bool {
        matches!(self, Self::Plugin { .. })
    }

    pub fn endpoint(&self) -> &NetworkEndpoint {
        match self {
            Self::Provider { endpoint } => endpoint,
            Self::MCP { endpoint, .. } => endpoint,
            Self::Plugin { endpoint } => endpoint,
        }
    }

    pub fn allow_localhost(&self) -> bool {
        match self {
            Self::MCP { allow_localhost, .. } => *allow_localhost,
            _ => false,
        }
    }
}

impl fmt::Display for NetworkScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider { endpoint } => write!(f, "provider:{}", endpoint),
            Self::MCP { endpoint, allow_localhost } => {
                write!(f, "mcp:{}:localhost={}", endpoint, allow_localhost)
            }
            Self::Plugin { endpoint } => write!(f, "plugin:{}", endpoint),
        }
    }
}

// ── 网络端点 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkEndpoint {
    pub host: String,
    pub port: Option<u16>,
    pub protocol: Protocol,
}

impl NetworkEndpoint {
    pub fn new(host: impl Into<String>, port: Option<u16>, protocol: Protocol) -> Self {
        Self { host: host.into(), port, protocol }
    }

    pub fn https(host: impl Into<String>) -> Self {
        Self::new(host, Some(443), Protocol::Https)
    }

    pub fn http(host: impl Into<String>) -> Self {
        Self::new(host, Some(80), Protocol::Http)
    }

    pub fn custom(host: impl Into<String>, port: u16) -> Self {
        Self::new(host, Some(port), Protocol::Custom)
    }

    pub fn localhost(port: u16) -> Self {
        Self::new("127.0.0.1", Some(port), Protocol::Http)
    }

    pub fn is_localhost(&self) -> bool {
        self.host == "127.0.0.1" || self.host == "localhost" || self.host == "::1"
    }
}

impl fmt::Display for NetworkEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.protocol, self.port) {
            (Protocol::Https, Some(443)) | (Protocol::Http, Some(80)) => {
                write!(f, "{}://{}", self.protocol, self.host)
            }
            (Protocol::Https, _) | (Protocol::Http, _) => {
                write!(f, "{}://{}:{}", self.protocol, self.host, self.port.unwrap_or(80))
            }
            (Protocol::Custom, Some(port)) => {
                write!(f, "{}:{}", self.host, port)
            }
            _ => write!(f, "{}", self.host),
        }
    }
}

// ── 协议 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Http,
    Https,
    Custom,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => write!(f, "http"),
            Self::Https => write!(f, "https"),
            Self::Custom => write!(f, "custom"),
        }
    }
}