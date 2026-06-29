use serde::{Deserialize, Serialize};

/// User writing preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// User's display name
    pub name: String,
    /// Preferred writing language ("zh", "en", "auto")
    pub language: String,
    /// Preferred writing style
    pub style: WritingStyle,
    /// Target audience / reader type
    pub reader_type: ReaderType,
    /// Genre preferences
    pub genres: Vec<String>,
    /// Custom instructions that agents should follow
    pub custom_instructions: Vec<String>,
    /// Preferred tone
    pub tone: Option<String>,
    /// Word count preferences per chapter
    pub word_count_preference: Option<WordCountPreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritingStyle {
    /// Formality level: "casual", "standard", "formal", "literary"
    pub formality: String,
    /// Pacing preference: "slow", "moderate", "fast"
    pub pacing: String,
    /// Description density: "minimal", "moderate", "rich"
    pub description_density: String,
    /// Dialogue style: "natural", "stylized", "minimal"
    pub dialogue_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReaderType {
    /// Young adult audience
    YoungAdult,
    /// General fiction
    General,
    /// Literary fiction
    Literary,
    /// Genre fiction (sci-fi, fantasy, etc.)
    Genre,
    /// Web novel readers
    WebNovel,
    /// Custom reader type
    Custom(String),
}

impl std::fmt::Display for ReaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::YoungAdult => write!(f, "young_adult"),
            Self::General => write!(f, "general"),
            Self::Literary => write!(f, "literary"),
            Self::Genre => write!(f, "genre"),
            Self::WebNovel => write!(f, "web_novel"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCountPreference {
    pub min_words: u32,
    pub max_words: u32,
    pub target_words: u32,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: "Writer".to_string(),
            language: "auto".to_string(),
            style: WritingStyle::default(),
            reader_type: ReaderType::General,
            genres: Vec::new(),
            custom_instructions: Vec::new(),
            tone: None,
            word_count_preference: None,
        }
    }
}

impl Default for WritingStyle {
    fn default() -> Self {
        Self {
            formality: "standard".to_string(),
            pacing: "moderate".to_string(),
            description_density: "moderate".to_string(),
            dialogue_style: "natural".to_string(),
        }
    }
}
