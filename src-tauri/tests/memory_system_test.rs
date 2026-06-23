use mnemosyne_lib::domain::agents::{MemorySystem, MemoryEntry, MemoryType};
use mnemosyne_lib::infra::memory::MemoryStore;
use tempfile::TempDir;

#[test]
fn test_memory_system_create() {
    let memory = MemorySystem::new(10);
    assert_eq!(memory.main_context_len(), 0);
    assert_eq!(memory.archival_store_len(), 0);
}

#[test]
fn test_archive_and_search() {
    let mut memory = MemorySystem::new(10);

    memory.archive(MemoryEntry {
        id: "1".to_string(),
        content: "Alice is a tall woman with red hair".to_string(),
        entry_type: MemoryType::Character,
        chapter: Some(1),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.archive(MemoryEntry {
        id: "2".to_string(),
        content: "The castle stands on a mountain peak".to_string(),
        entry_type: MemoryType::Setting,
        chapter: Some(1),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.archive(MemoryEntry {
        id: "3".to_string(),
        content: "Alice argued with Bob about the throne".to_string(),
        entry_type: MemoryType::Dialogue,
        chapter: Some(2),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    assert_eq!(memory.archival_store_len(), 3);

    let results = memory.search_memory("Alice", 5);
    assert!(!results.is_empty());
    assert!(results.iter().any(|e| e.content.contains("Alice")));

    let results = memory.search_memory("castle", 5);
    assert!(!results.is_empty());
    assert!(results.iter().any(|e| e.content.contains("castle")));

    let results = memory.search_memory("dragon", 5);
    assert!(results.is_empty());
}

#[test]
fn test_page_in_page_out() {
    let mut memory = MemorySystem::new(2);

    memory.archive(MemoryEntry {
        id: "1".to_string(),
        content: "First entry".to_string(),
        entry_type: MemoryType::Fact,
        chapter: None,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.archive(MemoryEntry {
        id: "2".to_string(),
        content: "Second entry".to_string(),
        entry_type: MemoryType::Fact,
        chapter: None,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.page_in("1").unwrap();
    assert_eq!(memory.main_context_len(), 1);

    memory.page_in("2").unwrap();
    assert_eq!(memory.main_context_len(), 2);

    memory.page_out("1").unwrap();
    assert_eq!(memory.main_context_len(), 1);
}

#[test]
fn test_format_main_context() {
    let mut memory = MemorySystem::new(10);

    memory.archive(MemoryEntry {
        id: "1".to_string(),
        content: "Key fact about the world".to_string(),
        entry_type: MemoryType::Setting,
        chapter: None,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        tags: vec![],
    });

    memory.page_in("1").unwrap();

    let formatted = memory.format_main_context();
    assert!(formatted.contains("Key fact about the world"));
}

#[tokio::test]
async fn test_memory_store_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path().to_path_buf());

    let memory = store.get_or_create("test-book", 10).await;
    {
        let mut mem = memory.write().await;
        mem.archive(MemoryEntry {
            id: "1".to_string(),
            content: "Persistent memory entry".to_string(),
            entry_type: MemoryType::Fact,
            chapter: Some(1),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tags: vec![],
        });
    }

    store.save_book("test-book").await;

    let store2 = MemoryStore::new(temp_dir.path().to_path_buf());
    let memory2 = store2.get_or_create("test-book", 10).await;
    let mem = memory2.read().await;
    let results = mem.search_memory("Persistent", 5);
    assert!(!results.is_empty());
    assert!(results[0].content.contains("Persistent memory entry"));
}

#[tokio::test]
async fn test_memory_stats() {
    let temp_dir = TempDir::new().unwrap();
    let store = MemoryStore::new(temp_dir.path().to_path_buf());

    let memory = store.get_or_create("stats-book", 10).await;
    {
        let mut mem = memory.write().await;
        mem.archive(MemoryEntry {
            id: "1".to_string(),
            content: "Entry one".to_string(),
            entry_type: MemoryType::Fact,
            chapter: None,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tags: vec![],
        });
        mem.archive(MemoryEntry {
            id: "2".to_string(),
            content: "Entry two".to_string(),
            entry_type: MemoryType::Fact,
            chapter: None,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            tags: vec![],
        });
    }

    let (main, archival) = store.stats("stats-book").await;
    assert_eq!(main, 0);
    assert_eq!(archival, 2);
}
