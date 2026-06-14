use std::path::PathBuf;

use crate::errors::AppError;

use super::models::*;

pub struct StoryManager {
    project_root: PathBuf,
}

impl StoryManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn book_dir(&self, _book_id: &str) -> PathBuf {
        self.project_root.clone()
    }

    pub fn chapters_dir(&self, book_id: &str) -> PathBuf {
        self.book_dir(book_id).join("chapters")
    }

    pub fn story_dir(&self, book_id: &str) -> PathBuf {
        self.book_dir(book_id).join("story")
    }

    fn state_file(&self, book_id: &str) -> PathBuf {
        self.story_dir(book_id).join("state.json")
    }

    fn snapshots_dir(&self, book_id: &str) -> PathBuf {
        self.story_dir(book_id).join("snapshots")
    }

    // --- Book Config ---

    pub fn create_book(&self, config: &BookConfig) -> Result<(), AppError> {
        let dir = self.book_dir(&config.id);
        std::fs::create_dir_all(dir.join("chapters"))
            .map_err(|e| AppError::internal(format!("Failed to create chapters dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("story/state"))
            .map_err(|e| AppError::internal(format!("Failed to create story dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("story/snapshots"))
            .map_err(|e| AppError::internal(format!("Failed to create snapshots dir: {}", e)))?;
        std::fs::create_dir_all(dir.join("story/drafts"))
            .map_err(|e| AppError::internal(format!("Failed to create drafts dir: {}", e)))?;

        let config_path = dir.join("book.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&config_path, json)
            .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))?;

        let state = StoryState::default();
        self.save_state(&config.id, &state)?;

        Ok(())
    }

    pub fn load_book_config(&self, book_id: &str) -> Result<BookConfig, AppError> {
        let path = self.book_dir(book_id).join("book.json");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read book config: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse book config: {}", e)))
    }

    pub fn save_book_config(&self, config: &BookConfig) -> Result<(), AppError> {
        let path = self.book_dir(&config.id).join("book.json");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| AppError::internal(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write config: {}", e)))
    }

    pub fn list_books(&self) -> Result<Vec<BookConfig>, AppError> {
        let config_path = self.project_root.join("book.json");
        if !config_path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| AppError::internal(format!("Failed to read book config: {}", e)))?;
        let config = serde_json::from_str::<BookConfig>(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse book config: {}", e)))?;
        Ok(vec![config])
    }

    pub fn delete_book(&self, _book_id: &str) -> Result<(), AppError> {
        let dir = &self.project_root;
        if dir.exists() {
            let _ = std::fs::remove_dir_all(dir.join("chapters"));
            let _ = std::fs::remove_dir_all(dir.join("story"));
            let _ = std::fs::remove_file(dir.join("book.json"));
        }
        Ok(())
    }

    // --- Chapters ---

    pub fn save_chapter(&self, book_id: &str, chapter: &ChapterContent) -> Result<(), AppError> {
        let dir = self.chapters_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create chapters dir: {}", e)))?;

        let filename = format!("{:04}_{}.md", chapter.number, sanitize_filename(&chapter.title));
        let path = dir.join(filename);
        let content = format!("# {}\n\n{}", chapter.title, chapter.content);
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write chapter: {}", e)))
    }

    pub fn load_chapter(&self, book_id: &str, chapter_number: u32) -> Result<Option<ChapterContent>, AppError> {
        let dir = self.chapters_dir(book_id);
        if !dir.exists() {
            return Ok(None);
        }

        let prefix = format!("{:04}_", chapter_number);
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .map_err(|e| AppError::internal(format!("Failed to read chapters dir: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with(&prefix)
            })
            .collect();

        match entries.first() {
            Some(entry) => {
                let content = std::fs::read_to_string(entry.path())
                    .map_err(|e| AppError::internal(format!("Failed to read chapter: {}", e)))?;
                let title = content
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("# ")
                    .to_string();
                let body = content
                    .lines()
                    .skip(2)
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(Some(ChapterContent {
                    number: chapter_number,
                    title,
                    content: body,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn list_chapters(&self, book_id: &str) -> Result<Vec<ChapterMeta>, AppError> {
        let index_path = self.chapters_dir(book_id).join("index.json");
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)
                .map_err(|e| AppError::internal(format!("Failed to read chapter index: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| AppError::internal(format!("Failed to parse chapter index: {}", e)))
        } else {
            Ok(Vec::new())
        }
    }

    pub fn save_chapter_index(&self, book_id: &str, chapters: &[ChapterMeta]) -> Result<(), AppError> {
        let dir = self.chapters_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create chapters dir: {}", e)))?;

        let path = dir.join("index.json");
        let json = serde_json::to_string_pretty(chapters)
            .map_err(|e| AppError::internal(format!("Failed to serialize chapter index: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write chapter index: {}", e)))
    }

    // --- Story State ---

    pub fn load_state(&self, book_id: &str) -> Result<StoryState, AppError> {
        let path = self.state_file(book_id);
        if !path.exists() {
            return Ok(StoryState::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read state: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse state: {}", e)))
    }

    pub fn save_state(&self, book_id: &str, state: &StoryState) -> Result<(), AppError> {
        let dir = self.story_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create story dir: {}", e)))?;

        let path = self.state_file(book_id);
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| AppError::internal(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write state: {}", e)))
    }

    // --- Snapshots ---

    pub fn save_snapshot(&self, book_id: &str, chapter_number: u32, state: &StoryState) -> Result<(), AppError> {
        let dir = self.snapshots_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create snapshots dir: {}", e)))?;

        let snapshot = ChapterSnapshot {
            chapter_number,
            state: state.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let path = dir.join(format!("{:04}.json", chapter_number));
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| AppError::internal(format!("Failed to serialize snapshot: {}", e)))?;
        std::fs::write(&path, json)
            .map_err(|e| AppError::internal(format!("Failed to write snapshot: {}", e)))
    }

    pub fn load_snapshot(&self, book_id: &str, chapter_number: u32) -> Result<Option<ChapterSnapshot>, AppError> {
        let path = self.snapshots_dir(book_id).join(format!("{:04}.json", chapter_number));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read snapshot: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::internal(format!("Failed to parse snapshot: {}", e)))
            .map(Some)
    }

    // --- Control Documents ---

    pub fn save_control_doc(&self, book_id: &str, filename: &str, content: &str) -> Result<(), AppError> {
        let dir = self.story_dir(book_id);
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::internal(format!("Failed to create story dir: {}", e)))?;

        let path = dir.join(filename);
        std::fs::write(&path, content)
            .map_err(|e| AppError::internal(format!("Failed to write control doc: {}", e)))
    }

    pub fn load_control_doc(&self, book_id: &str, filename: &str) -> Result<Option<String>, AppError> {
        let path = self.story_dir(book_id).join(filename);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::internal(format!("Failed to read control doc: {}", e)))?;
        Ok(Some(content))
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .chars()
        .take(50)
        .collect()
}
