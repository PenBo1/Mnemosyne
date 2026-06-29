use std::path::PathBuf;
use crate::shared::errors::AppError;
use super::types::UserProfile;

pub struct UserProfileStore {
    path: PathBuf,
    profile: Option<UserProfile>,
}

impl UserProfileStore {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let path = data_dir.join("user_profile.json");
        let profile = Self::load_from_disk(&path).ok();
        Self { path, profile }
    }

    fn load_from_disk(path: &std::path::Path) -> Result<UserProfile, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let profile: UserProfile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    pub fn get(&self) -> &UserProfile {
        match self.profile {
            Some(ref profile) => profile,
            None => {
                static DEFAULT: std::sync::OnceLock<UserProfile> = std::sync::OnceLock::new();
                DEFAULT.get_or_init(UserProfile::default)
            }
        }
    }

    pub fn get_or_create(&mut self) -> &UserProfile {
        if self.profile.is_none() {
            self.profile = Some(UserProfile::default());
            let _ = self.save();
        }
        self.profile.as_ref().unwrap()
    }

    pub fn update(&mut self, profile: UserProfile) -> Result<(), AppError> {
        self.profile = Some(profile);
        self.save()
    }

    fn save(&self) -> Result<(), AppError> {
        if let Some(ref profile) = self.profile {
            let json = serde_json::to_string_pretty(profile)
                .map_err(|e| AppError::internal(format!("Serialize error: {}", e)))?;
            std::fs::write(&self.path, json)
                .map_err(|e| AppError::internal(format!("Write error: {}", e)))?;
        }
        Ok(())
    }

    /// Format profile as a prompt section for agents
    pub fn format_for_prompt(&self) -> String {
        let p = self.get();
        let mut sections = Vec::new();

        sections.push(format!("User: {}", p.name));
        sections.push(format!("Language: {}", p.language));
        sections.push(format!("Style: formality={}, pacing={}, descriptions={}, dialogue={}",
            p.style.formality, p.style.pacing, p.style.description_density, p.style.dialogue_style));
        sections.push(format!("Target readers: {}", p.reader_type));

        if !p.genres.is_empty() {
            sections.push(format!("Preferred genres: {}", p.genres.join(", ")));
        }
        if let Some(ref tone) = p.tone {
            sections.push(format!("Tone: {}", tone));
        }
        if let Some(ref wc) = p.word_count_preference {
            sections.push(format!("Word count: {}-{} (target {})", wc.min_words, wc.max_words, wc.target_words));
        }
        for inst in &p.custom_instructions {
            sections.push(format!("Instruction: {}", inst));
        }

        format!("## User Profile\n{}\n", sections.join("\n"))
    }
}
