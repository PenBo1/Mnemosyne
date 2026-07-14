// 学习偏好的 IPC 命令 —— 暴露 learned_preferences 表的查询能力给前端。
//
// 设计:
// - 查询类:list / list_by_key / list_high_confidence
// - 维护类:delete(用户主动否认偏好)
// - 触发类:analyze(让用户主动请求"分析我的偏好")
// - 系统类:decay_stale(系统调用,前端不直接用,留给定时任务)
// - 合并类:merge_to_user_profile(高置信度偏好合并到 UserProfile)

use tauri::State;

use crate::core::agent::commands::AgentState;
use crate::domain::user::store::UserProfileStore;
use crate::domain::user::types::{ReaderType, UserProfile, WordCountPreference};
use crate::infrastructure::db::connection::Database;
use crate::infrastructure::db::state::DbState;
use crate::infrastructure::db::stores::learned_preferences::LearnedPreferenceRow;
use crate::shared::error::{AppError, IpcResponse};

/// 列出所有学习到的偏好(按 confidence 降序)
#[tauri::command]
pub async fn learned_preferences_list(
    state: State<'_, DbState>,
) -> Result<IpcResponse<Vec<LearnedPreferenceRow>>, AppError> {
    let rows = state.db.list_learned_preferences()?;
    Ok(IpcResponse::ok(rows))
}

/// 按 preference_key 列出偏好(同一 key 可能有多个 value)
#[tauri::command]
pub async fn learned_preferences_list_by_key(
    state: State<'_, DbState>,
    key: String,
) -> Result<IpcResponse<Vec<LearnedPreferenceRow>>, AppError> {
    if key.trim().is_empty() {
        return Err(AppError::invalid_input("key cannot be empty"));
    }
    if key.len() > 64 {
        return Err(AppError::invalid_input("key too long (max 64 chars)"));
    }
    let rows = state.db.list_learned_preferences_by_key(&key)?;
    Ok(IpcResponse::ok(rows))
}

/// 列出高置信度偏好(>= threshold,默认 0.7)
///
/// 用于 UI 展示"已确认的偏好",以及合并到 UserProfile 的候选
#[tauri::command]
pub async fn learned_preferences_list_high_confidence(
    state: State<'_, DbState>,
    threshold: Option<f64>,
) -> Result<IpcResponse<Vec<LearnedPreferenceRow>>, AppError> {
    let t = threshold.unwrap_or(0.7);
    if !(0.0..=1.0).contains(&t) {
        return Err(AppError::invalid_input("threshold must be in [0.0, 1.0]"));
    }
    let rows = state.db.list_high_confidence_preferences(Some(t))?;
    Ok(IpcResponse::ok(rows))
}

/// 删除指定偏好(用户主动否认)
#[tauri::command]
pub async fn learned_preferences_delete(
    state: State<'_, DbState>,
    id: String,
) -> Result<IpcResponse<bool>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::invalid_input("id cannot be empty"));
    }
    let deleted = state.db.delete_learned_preference(&id)?;
    if !deleted {
        return Err(AppError::not_found("Preference not found"));
    }
    Ok(IpcResponse::ok(deleted))
}

/// 主动分析 session 的用户偏好(用户点击"分析我的偏好"时触发)
///
/// 与 AgentEngine.analyze_user_preferences 的区别:
/// - IPC 命令通过 AgentState 获取 AgentEngine
/// - 业务逻辑在 AgentEngine 内部
/// - 返回提取的偏好数量
#[tauri::command]
pub async fn learned_preferences_analyze(
    state: State<'_, AgentState>,
    session_id: String,
) -> Result<IpcResponse<usize>, AppError> {
    if session_id.trim().is_empty() {
        return Err(AppError::invalid_input("session_id cannot be empty"));
    }
    let count = state.engine.analyze_user_preferences(&session_id).await?;
    Ok(IpcResponse::ok(count))
}

/// 衰减长期未观察的偏好(定时任务调用,前端一般不直接用)
///
/// cutoff_days:距今多少天未观察则衰减(默认 30)
#[tauri::command]
pub async fn learned_preferences_decay_stale(
    state: State<'_, DbState>,
    cutoff_days: Option<i64>,
) -> Result<IpcResponse<u64>, AppError> {
    let days = cutoff_days.unwrap_or(30);
    if days < 1 {
        return Err(AppError::invalid_input("cutoff_days must be >= 1"));
    }
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let affected = state.db.decay_stale_preferences(&cutoff.to_rfc3339())?;
    Ok(IpcResponse::ok(affected))
}

/// 合并高置信度偏好到 UserProfile(用户主动触发或定时任务调用)
///
/// threshold:置信度阈值,默认 0.7(对应 occurrence_count >= 7 的稳定期偏好)
///
/// 合并规则:
/// - 查询 confidence >= threshold 的偏好
/// - 按 preference_key 映射到 UserProfile 字段(见 apply_preference_to_profile)
/// - 高置信度偏好覆盖 UserProfile 中的对应字段
/// - 用户可随时通过 user_update_profile 手动覆盖
#[tauri::command]
pub async fn user_merge_learned_preferences(
    state: State<'_, DbState>,
    threshold: Option<f64>,
) -> Result<IpcResponse<MergeResult>, AppError> {
    let t = threshold.unwrap_or(0.7);
    if !(0.0..=1.0).contains(&t) {
        return Err(AppError::invalid_input("threshold must be in [0.0, 1.0]"));
    }
    let mut store = UserProfileStore::new(state.data_dir.root());
    let result = merge_to_user_profile(&state.db, &mut store, Some(t))?;
    Ok(IpcResponse::ok(result))
}

/// 合并结果
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    /// 成功合并的偏好数量
    pub merged_count: usize,
    /// 跳过的偏好数量(value 为空或无法映射)
    pub skipped_count: usize,
    /// 已应用的 preference_key 列表
    pub applied_keys: Vec<String>,
}

/// 合并高置信度的 learned_preferences 到 UserProfile。
///
/// 流程:
/// 1. 查询 confidence >= threshold 的偏好(按 confidence 降序)
/// 2. 逐条映射到 UserProfile 字段
/// 3. 若有字段被更新,持久化 UserProfile
///
/// 设计参考:learned_preferences.rs 注释 "高置信度的 learned_preferences 通过 merge_to_user_profile() 合并"
pub fn merge_to_user_profile(
    db: &Database,
    store: &mut UserProfileStore,
    threshold: Option<f64>,
) -> Result<MergeResult, AppError> {
    let prefs = db.list_high_confidence_preferences(threshold)?;
    let mut profile = store.get().clone();
    let mut merged_count = 0usize;
    let mut skipped_count = 0usize;
    let mut applied_keys = Vec::new();

    for pref in &prefs {
        if apply_preference_to_profile(&mut profile, pref) {
            merged_count += 1;
            applied_keys.push(pref.preference_key.clone());
        } else {
            skipped_count += 1;
        }
    }

    if merged_count > 0 {
        store.update(profile)?;
    }
    Ok(MergeResult {
        merged_count,
        skipped_count,
        applied_keys,
    })
}

/// 将单条偏好应用到 UserProfile,返回是否成功修改。
///
/// 支持的 preference_key:
/// - `language` → profile.language
/// - `name` / `user_name` → profile.name
/// - `tone` → profile.tone
/// - `genre` → profile.genres(去重追加)
/// - `instruction` / `custom_instruction` → profile.custom_instructions(去重追加)
/// - `style.formality` / `style.pacing` / `style.description_density` / `style.dialogue_style`
/// - `reader_type` → profile.reader_type(young_adult/general/literary/genre/web_novel/自定义)
/// - `word_count.min` / `word_count.max` / `word_count.target` → profile.word_count_preference
fn apply_preference_to_profile(profile: &mut UserProfile, pref: &LearnedPreferenceRow) -> bool {
    let value = pref.preference_value.trim();
    if value.is_empty() {
        return false;
    }
    match pref.preference_key.as_str() {
        "language" => {
            profile.language = value.to_string();
            true
        }
        "name" | "user_name" => {
            profile.name = value.to_string();
            true
        }
        "tone" => {
            profile.tone = Some(value.to_string());
            true
        }
        "genre" => {
            if !profile.genres.iter().any(|g| g == value) {
                profile.genres.push(value.to_string());
            }
            true
        }
        "instruction" | "custom_instruction" => {
            if !profile.custom_instructions.iter().any(|i| i == value) {
                profile.custom_instructions.push(value.to_string());
            }
            true
        }
        "style.formality" => {
            profile.style.formality = value.to_string();
            true
        }
        "style.pacing" => {
            profile.style.pacing = value.to_string();
            true
        }
        "style.description_density" => {
            profile.style.description_density = value.to_string();
            true
        }
        "style.dialogue_style" => {
            profile.style.dialogue_style = value.to_string();
            true
        }
        "reader_type" => {
            profile.reader_type = match value {
                "young_adult" => ReaderType::YoungAdult,
                "general" => ReaderType::General,
                "literary" => ReaderType::Literary,
                "genre" => ReaderType::Genre,
                "web_novel" => ReaderType::WebNovel,
                s => ReaderType::Custom(s.to_string()),
            };
            true
        }
        "word_count.min" | "word_count.max" | "word_count.target" => {
            match value.parse::<u32>() {
                Ok(n) => {
                    let wc = profile.word_count_preference.get_or_insert(WordCountPreference {
                        min_words: 0,
                        max_words: 0,
                        target_words: 0,
                    });
                    match pref.preference_key.as_str() {
                        "word_count.min" => wc.min_words = n,
                        "word_count.max" => wc.max_words = n,
                        "word_count.target" => wc.target_words = n,
                        _ => unreachable!(),
                    }
                    true
                }
                Err(_) => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::types::{ReaderType, UserProfile, WritingStyle};

    fn make_preference(key: &str, value: &str) -> LearnedPreferenceRow {
        LearnedPreferenceRow {
            id: format!("test-{}", key),
            preference_key: key.to_string(),
            preference_value: value.to_string(),
            confidence: 0.8,
            occurrence_count: 7,
            learned_from: None,
            last_seen_at: chrono::Utc::now().to_rfc3339(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn apply_language_updates_profile() {
        let mut profile = UserProfile::default();
        let pref = make_preference("language", "zh");
        assert!(apply_preference_to_profile(&mut profile, &pref));
        assert_eq!(profile.language, "zh");
    }

    #[test]
    fn apply_genre_appends_without_duplicate() {
        let mut profile = UserProfile::default();
        profile.genres.push("sci-fi".to_string());

        let pref1 = make_preference("genre", "fantasy");
        assert!(apply_preference_to_profile(&mut profile, &pref1));
        assert_eq!(profile.genres.len(), 2);

        // 重复的 genre 不追加
        let pref2 = make_preference("genre", "sci-fi");
        assert!(apply_preference_to_profile(&mut profile, &pref2));
        assert_eq!(profile.genres.len(), 2);
    }

    #[test]
    fn apply_reader_type_maps_known_values() {
        let mut profile = UserProfile::default();
        assert!(apply_preference_to_profile(
            &mut profile,
            &make_preference("reader_type", "literary")
        ));
        assert_eq!(profile.reader_type, ReaderType::Literary);
    }

    #[test]
    fn apply_reader_type_custom_fallback() {
        let mut profile = UserProfile::default();
        assert!(apply_preference_to_profile(
            &mut profile,
            &make_preference("reader_type", "academic")
        ));
        assert_eq!(profile.reader_type, ReaderType::Custom("academic".to_string()));
    }

    #[test]
    fn apply_word_count_creates_preference_if_absent() {
        let mut profile = UserProfile::default();
        assert!(profile.word_count_preference.is_none());
        assert!(apply_preference_to_profile(
            &mut profile,
            &make_preference("word_count.target", "5000")
        ));
        let wc = profile.word_count_preference.expect("wc should exist");
        assert_eq!(wc.target_words, 5000);
    }

    #[test]
    fn apply_word_count_invalid_value_returns_false() {
        let mut profile = UserProfile::default();
        assert!(!apply_preference_to_profile(
            &mut profile,
            &make_preference("word_count.target", "not_a_number")
        ));
        assert!(profile.word_count_preference.is_none());
    }

    #[test]
    fn apply_empty_value_returns_false() {
        let mut profile = UserProfile::default();
        let pref = make_preference("language", "   ");
        assert!(!apply_preference_to_profile(&mut profile, &pref));
    }

    #[test]
    fn apply_unknown_key_returns_false() {
        let mut profile = UserProfile::default();
        let pref = make_preference("unknown_key", "value");
        assert!(!apply_preference_to_profile(&mut profile, &pref));
    }

    #[test]
    fn apply_style_fields_update_writing_style() {
        let mut profile = UserProfile::default();
        assert!(apply_preference_to_profile(
            &mut profile,
            &make_preference("style.formality", "formal")
        ));
        assert_eq!(profile.style.formality, "formal");
        assert_eq!(profile.style.pacing, WritingStyle::default().pacing);
    }

    #[test]
    fn merge_to_user_profile_persists_changes() {
        let db = Database::connect_in_memory().expect("db");
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut store = UserProfileStore::new(tmp.path());

        // 插入 2 条高置信度偏好 + 1 条低置信度(不达 0.7)
        db.upsert_learned_preference("language", "zh", None).ok();
        for _ in 0..7 {
            db.upsert_learned_preference("tone", "concise", None).ok();
        }
        db.upsert_learned_preference("unknown_key", "x", None).ok();
        for _ in 0..7 {
            db.upsert_learned_preference("unknown_key", "y", None).ok();
        }

        let result = merge_to_user_profile(&db, &mut store, Some(0.7)).unwrap();
        // language(0.2) 不达 0.7,tone(0.8) 达 0.7,unknown_key(0.8) 跳过
        assert_eq!(result.merged_count, 1);
        assert_eq!(result.applied_keys, vec!["tone".to_string()]);
        assert_eq!(store.get().tone.as_deref(), Some("concise"));
    }
}
