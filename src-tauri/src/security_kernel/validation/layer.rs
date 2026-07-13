use std::path::{Path, PathBuf};
use url::Url;
use uuid::Uuid;

use crate::shared::error::AppError;
use crate::security_kernel::permission::Operation;

use super::path::{CanonicalPath, validate_path, validate_path_with_base, validate_path_for_creation};
use super::url::{validate_url, UrlValidationConfig, validate_url_for_ssrf};
use super::id::{validate_uuid, validate_uuid_v4, validate_uuid_v7};
use super::endpoint::{NetworkEndpoint, validate_endpoint, create_ai_endpoint, create_strict_endpoint, create_internal_endpoint};

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub url_config: UrlValidationConfig,
    pub fs_base: Option<PathBuf>,
    pub allow_local_fs: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            url_config: UrlValidationConfig::default(),
            fs_base: None,
            allow_local_fs: true,
        }
    }
}

impl ValidationConfig {
    pub fn with_fs_base(base: PathBuf) -> Self {
        Self {
            url_config: UrlValidationConfig::default(),
            fs_base: Some(base),
            allow_local_fs: true,
        }
    }

    pub fn with_url_config(url_config: UrlValidationConfig) -> Self {
        Self {
            url_config,
            fs_base: None,
            allow_local_fs: true,
        }
    }

    pub fn strict() -> Self {
        Self {
            url_config: UrlValidationConfig::default(),
            fs_base: None,
            allow_local_fs: false,
        }
    }
}

pub struct ValidationLayer {
    config: ValidationConfig,
}

impl ValidationLayer {
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
        }
    }

    pub fn with_config(config: ValidationConfig) -> Self {
        Self { config }
    }

    pub fn validate_operation(&self, op: &Operation) -> Result<(), AppError> {
        match op {
            Operation::Filesystem { scope, operation, path } => {
                self.validate_fs_operation(scope, operation, path)?;
            }
            Operation::Shell { scope, command, args } => {
                self.validate_shell_operation(scope, command, args)?;
            }
            Operation::Network { scope, endpoint, method } => {
                self.validate_network_operation(scope, endpoint, method)?;
            }
        }
        Ok(())
    }

    fn validate_fs_operation(
        &self,
        _scope: &crate::security_kernel::permission::FsScope,
        _operation: &crate::security_kernel::permission::FsOperation,
        path: &str,
    ) -> Result<(), AppError> {
        let path_obj = Path::new(path);

        if let Some(base) = &self.config.fs_base {
            validate_path_with_base(path_obj, base)?;
        } else if self.config.allow_local_fs {
            validate_path(path_obj, None)?;
        } else {
            return Err(AppError::forbidden("Local filesystem access is not allowed"));
        }

        Ok(())
    }

    fn validate_shell_operation(
        &self,
        _scope: &crate::security_kernel::permission::ShellScope,
        command: &str,
        args: &[String],
    ) -> Result<(), AppError> {
        if command.is_empty() {
            return Err(AppError::missing_field("command"));
        }

        if command.contains("..") {
            return Err(AppError::path_traversal());
        }

        if command.chars().any(|c| c.is_control()) {
            return Err(AppError::invalid_input("Command contains control characters"));
        }

        for arg in args {
            if arg.contains("..") {
                return Err(AppError::path_traversal());
            }
            if arg.chars().any(|c| c.is_control()) {
                return Err(AppError::invalid_input("Argument contains control characters"));
            }
        }

        Ok(())
    }

    fn validate_network_operation(
        &self,
        scope: &crate::security_kernel::permission::NetworkScope,
        endpoint: &str,
        method: &str,
    ) -> Result<(), AppError> {
        if endpoint.is_empty() {
            return Err(AppError::missing_field("endpoint"));
        }

        if method.is_empty() {
            return Err(AppError::missing_field("method"));
        }

        let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        if !valid_methods.contains(&method.to_uppercase().as_str()) {
            return Err(AppError::invalid_input(format!("Invalid HTTP method: {}", method)));
        }

        let url = Url::parse(endpoint).map_err(|e| {
            AppError::invalid_format(format!("Invalid endpoint URL: {}", e))
        })?;

        let network_endpoint = scope_to_endpoint(scope);
        validate_endpoint(&url, &network_endpoint)?;

        Ok(())
    }

    pub fn validate_path(&self, path: &Path) -> Result<CanonicalPath, AppError> {
        if let Some(base) = &self.config.fs_base {
            validate_path_with_base(path, base)
        } else if self.config.allow_local_fs {
            validate_path(path, None)
        } else {
            Err(AppError::forbidden("Local filesystem access is not allowed"))
        }
    }

    pub fn validate_path_for_creation(&self, path: &Path) -> Result<CanonicalPath, AppError> {
        if let Some(base) = &self.config.fs_base {
            validate_path_for_creation(path, base)
        } else if self.config.allow_local_fs {
            validate_path(path, None)
        } else {
            Err(AppError::forbidden("Local filesystem access is not allowed"))
        }
    }

    pub fn validate_url(&self, url: &Url) -> Result<(), AppError> {
        validate_url(url, &self.config.url_config)
    }

    pub fn validate_url_str(&self, url_str: &str) -> Result<Url, AppError> {
        let url = Url::parse(url_str).map_err(|e| {
            AppError::invalid_format(format!("Invalid URL: {}", e))
        })?;
        self.validate_url(&url)?;
        Ok(url)
    }

    pub fn validate_url_for_ssrf(&self, url: &Url) -> Result<(), AppError> {
        validate_url_for_ssrf(url)
    }

    pub fn validate_id(&self, id: &str, name: &str) -> Result<Uuid, AppError> {
        validate_uuid(id, name)
    }

    pub fn validate_id_v4(&self, id: &str, name: &str) -> Result<Uuid, AppError> {
        validate_uuid_v4(id, name)
    }

    pub fn validate_id_v7(&self, id: &str, name: &str) -> Result<Uuid, AppError> {
        validate_uuid_v7(id, name)
    }

    pub fn validate_endpoint(&self, url: &Url, endpoint: &NetworkEndpoint) -> Result<(), AppError> {
        validate_endpoint(url, endpoint)
    }

    pub fn config(&self) -> &ValidationConfig {
        &self.config
    }

    pub fn set_fs_base(&mut self, base: PathBuf) {
        self.config.fs_base = Some(base);
    }

    pub fn set_url_config(&mut self, url_config: UrlValidationConfig) {
        self.config.url_config = url_config;
    }
}

impl Default for ValidationLayer {
    fn default() -> Self {
        Self::new()
    }
}

fn scope_to_endpoint(scope: &crate::security_kernel::permission::NetworkScope) -> NetworkEndpoint {
    use crate::security_kernel::permission::NetworkScope;

    match scope {
        NetworkScope::Provider { endpoint } => {
            let host = endpoint.host.clone();
            create_strict_endpoint(vec![host])
        }
        NetworkScope::MCP { allow_localhost, .. } => {
            if *allow_localhost {
                create_internal_endpoint()
            } else {
                create_ai_endpoint()
            }
        }
        NetworkScope::Plugin { endpoint } => {
            create_strict_endpoint(vec![endpoint.host.clone()])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validation_layer_new() {
        let layer = ValidationLayer::new();
        assert!(layer.config.fs_base.is_none());
    }

    #[test]
    fn test_validation_layer_with_fs_base() {
        let base = PathBuf::from("/tmp");
        let layer = ValidationLayer::with_config(ValidationConfig::with_fs_base(base.clone()));
        assert_eq!(layer.config.fs_base, Some(base));
    }

    #[test]
    fn test_validate_url_str_valid() {
        let layer = ValidationLayer::new();
        let result = layer.validate_url_str("https://example.com/path");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_str_invalid() {
        let layer = ValidationLayer::new();
        let result = layer.validate_url_str("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_id_valid() {
        let layer = ValidationLayer::new();
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(layer.validate_id(id, "test_id").is_ok());
    }

    #[test]
    fn test_validate_id_empty() {
        let layer = ValidationLayer::new();
        assert!(layer.validate_id("", "test_id").is_err());
    }
}