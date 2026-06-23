use mnemosyne_lib::domain::user_profile::{UserProfileStore, UserProfile, WritingStyle, ReaderType, WordCountPreference};
use tempfile::TempDir;

#[test]
fn test_user_profile_default() {
    let profile = UserProfile::default();
    assert_eq!(profile.name, "Writer");
    assert_eq!(profile.language, "auto");
    assert_eq!(profile.reader_type, ReaderType::General);
    assert!(profile.genres.is_empty());
    assert!(profile.custom_instructions.is_empty());
}

#[test]
fn test_user_profile_store_create() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = UserProfileStore::new(temp_dir.path());

    // Should create default profile on first access
    let profile = store.get_or_create();
    assert_eq!(profile.name, "Writer");
    assert!(temp_dir.path().join("user_profile.json").exists());
}

#[test]
fn test_user_profile_store_update() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = UserProfileStore::new(temp_dir.path());
    store.get_or_create();

    let new_profile = UserProfile {
        name: "TestUser".to_string(),
        language: "zh".to_string(),
        style: WritingStyle {
            formality: "literary".to_string(),
            pacing: "slow".to_string(),
            description_density: "rich".to_string(),
            dialogue_style: "stylized".to_string(),
        },
        reader_type: ReaderType::Literary,
        genres: vec!["fantasy".to_string(), "literary fiction".to_string()],
        custom_instructions: vec!["Use classical Chinese idioms".to_string()],
        tone: Some("melancholic".to_string()),
        word_count_preference: Some(WordCountPreference {
            min_words: 2000,
            max_words: 4000,
            target_words: 3000,
        }),
    };

    store.update(new_profile).unwrap();

    let profile = store.get();
    assert_eq!(profile.name, "TestUser");
    assert_eq!(profile.language, "zh");
    assert_eq!(profile.style.formality, "literary");
    assert_eq!(profile.reader_type, ReaderType::Literary);
    assert_eq!(profile.genres.len(), 2);
    assert_eq!(profile.custom_instructions.len(), 1);
    assert_eq!(profile.tone.as_deref(), Some("melancholic"));
}

#[test]
fn test_user_profile_store_persistence() {
    let temp_dir = TempDir::new().unwrap();

    // Create and save
    {
        let mut store = UserProfileStore::new(temp_dir.path());
        let profile = store.get_or_create();
        let mut p = profile.clone();
        p.name = "PersistentUser".to_string();
        store.update(p).unwrap();
    }

    // Load from disk
    let store2 = UserProfileStore::new(temp_dir.path());
    let profile = store2.get();
    assert_eq!(profile.name, "PersistentUser");
}

#[test]
fn test_format_for_prompt() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = UserProfileStore::new(temp_dir.path());
    let profile = store.get_or_create();

    let mut p = profile.clone();
    p.name = "Alice".to_string();
    p.language = "en".to_string();
    p.genres = vec!["fantasy".to_string()];
    p.tone = Some("dark".to_string());
    p.word_count_preference = Some(WordCountPreference {
        min_words: 1500,
        max_words: 3000,
        target_words: 2000,
    });
    store.update(p).unwrap();

    let prompt = store.format_for_prompt();
    assert!(prompt.contains("## User Profile"));
    assert!(prompt.contains("Alice"));
    assert!(prompt.contains("fantasy"));
    assert!(prompt.contains("dark"));
    assert!(prompt.contains("2000"));
}

#[test]
fn test_reader_type_display() {
    assert_eq!(ReaderType::YoungAdult.to_string(), "young_adult");
    assert_eq!(ReaderType::General.to_string(), "general");
    assert_eq!(ReaderType::Literary.to_string(), "literary");
    assert_eq!(ReaderType::Genre.to_string(), "genre");
    assert_eq!(ReaderType::WebNovel.to_string(), "web_novel");
    assert_eq!(ReaderType::Custom("custom_audience".to_string()).to_string(), "custom_audience");
}
