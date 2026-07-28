//! ═══════════════════════════════════════════════════════════════════════════
//! 用户存储 - 文件系统持久化
//! ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;
use crate::shared::error::AppError;
use super::types::UserProfile;

pub struct UserProfileStore {
    path: PathBuf,
    profile: Option<UserProfile>,
}

impl UserProfileStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let path = data_dir.join("user_profile.json");
        let profile = match Self::load_from_disk(&path) {
            Ok(p) => {
                tracing::info!(path = %path.display(), "[UserProfileStore] Loaded existing profile");
                Some(p)
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "[UserProfileStore] Failed to load, will use default");
                None
            }
        };
        Self { path, profile }
    }

    fn load_from_disk(path: &std::path::Path) -> Result<UserProfile, AppError> {
        let content = crate::infrastructure::fs::fs_utils::read_file(path)?;
        let profile: UserProfile = serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("parse user_profile: {}", e)))?;
        Ok(profile)
    }

    pub fn get(&self) -> &UserProfile {
        match self.profile {
            Some(ref profile) => profile,
            None => { static DEFAULT: std::sync::OnceLock<UserProfile> = std::sync::OnceLock::new(); DEFAULT.get_or_init(UserProfile::default) }
        }
    }

    pub fn get_or_create(&mut self) -> &UserProfile {
        if self.profile.is_none() {
            self.profile = Some(UserProfile::default());
            tracing::info!(path = %self.path.display(), "[UserProfileStore] Created default profile");
            if let Err(e) = self.save() {
                tracing::warn!(error = %e, "[UserProfileStore] Failed to save default profile");
            }
        }
        self.profile.as_ref().unwrap()
    }

    pub fn update(&mut self, profile: UserProfile) -> Result<(), AppError> {
        tracing::info!(path = %self.path.display(), "[UserProfileStore] Updating profile");
        self.profile = Some(profile);
        self.save()?;
        tracing::info!("[UserProfileStore] Profile updated successfully");
        Ok(())
    }

    fn save(&self) -> Result<(), AppError> {
        if let Some(ref profile) = self.profile {
            let json = serde_json::to_string_pretty(profile).map_err(|e| AppError::internal(format!("Serialize error: {}", e)))?;
            std::fs::write(&self.path, json).map_err(|e| AppError::internal(format!("Write error: {}", e)))?;
        }
        Ok(())
    }

    pub fn format_for_prompt(&self) -> String {
        self.get().format_for_prompt()
    }
}