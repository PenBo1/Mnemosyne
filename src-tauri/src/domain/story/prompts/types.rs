use serde::{Deserialize, Serialize};

/// 语言类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Zh,
    En,
}

impl Default for Language {
    fn default() -> Self {
        Self::Zh
    }
}

/// 叙事人称
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NarrativePerson {
    First,
    Third,
}

/// 写作模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WriterMode {
    Full,
    Creative,
}

/// 同人模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FanficMode {
    Canon,
    Au,
    Ooc,
    Cp,
}

/// 题材配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreConfig {
    pub id: String,
    pub name: String,
    pub language: Language,
    pub fatigue_words: Vec<String>,
    pub pacing_rule: String,
    pub chapter_types: Vec<String>,
    pub numerical_system: bool,
    pub power_scaling: bool,
}

impl Default for GenreConfig {
    fn default() -> Self {
        Self {
            id: "other".to_string(),
            name: "其他".to_string(),
            language: Language::Zh,
            fatigue_words: Vec::new(),
            pacing_rule: String::new(),
            chapter_types: vec![
                "过渡".to_string(),
                "冲突".to_string(),
                "高潮".to_string(),
                "收束".to_string(),
            ],
            numerical_system: false,
            power_scaling: false,
        }
    }
}

/// 字数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthSpec {
    pub target: u32,
    pub soft_min: u32,
    pub soft_max: u32,
    pub hard_min: u32,
    pub hard_max: u32,
}

impl LengthSpec {
    pub fn from_chapter_words(chapter_words: u32, _language: &Language) -> Self {
        let target = chapter_words;
        let soft_range = (target as f64 * 0.1) as u32;
        let hard_range = (target as f64 * 0.2) as u32;

        Self {
            target,
            soft_min: target.saturating_sub(soft_range),
            soft_max: target + soft_range,
            hard_min: target.saturating_sub(hard_range),
            hard_max: target + hard_range,
        }
    }
}

/// 书籍规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookRules {
    pub protagonist_name: Option<String>,
    pub personality_lock: Vec<String>,
    pub behavioral_constraints: Vec<String>,
    pub prohibitions: Vec<String>,
    pub narrative_person: Option<NarrativePerson>,
    pub enable_full_cast_tracking: bool,
    pub genre_forbidden: Vec<String>,
}

impl Default for BookRules {
    fn default() -> Self {
        Self {
            protagonist_name: None,
            personality_lock: Vec::new(),
            behavioral_constraints: Vec::new(),
            prohibitions: Vec::new(),
            narrative_person: None,
            enable_full_cast_tracking: false,
            genre_forbidden: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_spec() {
        let spec = LengthSpec::from_chapter_words(3000, &Language::Zh);
        assert_eq!(spec.target, 3000);
        assert_eq!(spec.soft_min, 2700);
        assert_eq!(spec.soft_max, 3300);
        assert_eq!(spec.hard_min, 2400);
        assert_eq!(spec.hard_max, 3600);
    }
}
