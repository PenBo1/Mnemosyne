
use tokio::sync::Mutex;
use super::discovery::SkillManager;
use crate::infrastructure::fs::data_dir::DataDir;

pub struct SkillState {
    pub manager: Mutex<SkillManager>,
}

impl SkillState {
    pub fn new(data_dir: &DataDir) -> Self {
        let mut skill_manager = SkillManager::new();
        if let Some(home) = dirs::home_dir() {
            skill_manager.add_dir(home.join(".mnemosyne").join("skills"));
        }
        skill_manager.add_dir(data_dir.skills_dir());
        if let Err(e) = skill_manager.discover() {
            tracing::warn!("Failed to discover skills: {}", e);
        }
        tracing::info!(count = skill_manager.list().len(), "Skills discovered");
        Self {
            manager: Mutex::new(skill_manager),
        }
    }
}

impl Default for SkillState {
    fn default() -> Self {
        Self {
            manager: Mutex::new(SkillManager::new()),
        }
    }
}