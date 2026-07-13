use crate::shared::error::AppError;
use super::store::SecretKey;

pub struct KeyringBackend {
    service_name: String,
}

impl KeyringBackend {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn is_available() -> bool {
        cfg_keyring_available()
    }

    pub fn set(&self, key: &SecretKey, value: &str) -> Result<(), AppError> {
        if !cfg_keyring_available() {
            return Err(AppError::unavailable("OS keyring not available on this platform"));
        }

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let entry = create_entry(&self.service_name, &key.name)?;
            entry.set_password(value).map_err(|e|
                AppError::internal(format!("keyring set failed: {}", e))
            )?;
            Ok(())
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(AppError::unavailable("OS keyring not available on Linux (use file fallback)"))
        }
    }

    pub fn get(&self, key: &SecretKey) -> Result<Option<String>, AppError> {
        if !cfg_keyring_available() {
            return Err(AppError::unavailable("OS keyring not available on this platform"));
        }

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let entry = create_entry(&self.service_name, &key.name)?;
            match entry.get_password() {
                Ok(v) => Ok(Some(v)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(AppError::internal(format!("keyring get failed: {}", e))),
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(AppError::unavailable("OS keyring not available on Linux (use file fallback)"))
        }
    }

    pub fn delete(&self, key: &SecretKey) -> Result<bool, AppError> {
        if !cfg_keyring_available() {
            return Err(AppError::unavailable("OS keyring not available on this platform"));
        }

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let entry = create_entry(&self.service_name, &key.name)?;
            match entry.delete_credential() {
                Ok(()) => Ok(true),
                Err(keyring::Error::NoEntry) => Ok(false),
                Err(e) => Err(AppError::internal(format!("keyring delete failed: {}", e))),
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Err(AppError::unavailable("OS keyring not available on Linux (use file fallback)"))
        }
    }
}

fn cfg_keyring_available() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn create_entry(service: &str, account: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(service, account).map_err(|e|
        AppError::internal(format!("keyring entry creation failed: {}", e))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_availability_check() {
        let expected = cfg!(any(target_os = "macos", target_os = "windows"));
        assert_eq!(KeyringBackend::is_available(), expected);
    }

    #[test]
    fn new_with_service_name() {
        let backend = KeyringBackend::new("mnemosyne");
        assert_eq!(backend.service_name, "mnemosyne");
    }
}