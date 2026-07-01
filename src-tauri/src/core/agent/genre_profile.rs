// S7.1: 题材规则 profile —— 定义、解析、加载（项目级 → 内置 → other fallback）
//
// 移植自 inkos `genre-profile.ts` + `rules-reader.ts`。与 inkos 的差异：
// - 用 serde_yaml 替代 js-yaml 解析 frontmatter
// - 内置 profile 文件用 include_str! 编译时嵌入（而非运行时 readdir）
// - 字段默认值由 #[serde(default)] 提供（缺失字段不报错）
// - frontmatter 切分用按行扫描（避免 regex 跨平台 `\r\n` 陷阱）

use crate::shared::errors::AppError;
use serde::Deserialize;

/// 题材 profile 结构化字段（对应 inkos GenreProfile）。
///
/// YAML frontmatter 用 camelCase 命名（对齐 inkos 习惯），serde 自动映射到 snake_case 字段。
/// 缺失的布尔/字符串字段会用 Default 填充，不会让解析失败。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenreProfile {
    pub name: String,
    pub id: String,
    #[serde(default = "default_language")]
    pub language: String,
    pub chapter_types: Vec<String>,
    pub fatigue_words: Vec<String>,
    #[serde(default)]
    pub numerical_system: bool,
    #[serde(default)]
    pub power_scaling: bool,
    #[serde(default)]
    pub era_research: bool,
    #[serde(default)]
    pub pacing_rule: String,
    #[serde(default)]
    pub satisfaction_types: Vec<String>,
    #[serde(default)]
    pub audit_dimensions: Vec<u32>,
}

fn default_language() -> String {
    "zh".to_string()
}

/// 解析后的题材 profile（结构化字段 + Markdown body 正文）。
#[derive(Debug, Clone)]
pub struct ParsedGenreProfile {
    pub profile: GenreProfile,
    pub body: String,
}

/// 内置题材 profile 注册表（编译时 include_str! 嵌入）。
///
/// 顺序按字母排序，便于人工核对。`other` 作为 fallback 必须存在。
const BUILTIN_GENRES: &[(&str, &str)] = &[
    ("cozy", include_str!("genre_profiles/cozy.md")),
    ("cultivation", include_str!("genre_profiles/cultivation.md")),
    ("dungeon-core", include_str!("genre_profiles/dungeon-core.md")),
    ("horror", include_str!("genre_profiles/horror.md")),
    ("isekai", include_str!("genre_profiles/isekai.md")),
    ("litrpg", include_str!("genre_profiles/litrpg.md")),
    ("other", include_str!("genre_profiles/other.md")),
    ("progression", include_str!("genre_profiles/progression.md")),
    ("romantasy", include_str!("genre_profiles/romantasy.md")),
    ("sci-fi", include_str!("genre_profiles/sci-fi.md")),
    ("system-apocalypse", include_str!("genre_profiles/system-apocalypse.md")),
    ("tower-climber", include_str!("genre_profiles/tower-climber.md")),
    ("urban", include_str!("genre_profiles/urban.md")),
    ("xianxia", include_str!("genre_profiles/xianxia.md")),
    ("xuanhuan", include_str!("genre_profiles/xuanhuan.md")),
];

/// 从 frontmatter + Markdown body 中解析题材 profile。
///
/// 对齐 inkos `parseGenreProfile`：
/// 1. 按行扫描寻找首对 `---` 围栏，围栏内为 YAML frontmatter，之后为 body
/// 2. frontmatter 用 serde_yaml 解析为 GenreProfile（camelCase → snake_case）
/// 3. body trim 后保留为题材规则正文（题材禁忌、叙事指导等）
///
/// frontmatter 缺失或 YAML 解析失败返回 `AppError::bad_request`。
pub fn parse_genre_profile(raw: &str) -> Result<ParsedGenreProfile, AppError> {
    let split = parse_frontmatter_split(raw)
        .ok_or_else(|| AppError::bad_request("Genre profile missing YAML frontmatter (--- ... ---)"))?;

    let profile: GenreProfile = serde_yaml::from_str(&split.front)
        .map_err(|e| AppError::bad_request(format!("Genre profile YAML parse failed: {}", e)))?;

    Ok(ParsedGenreProfile {
        profile,
        body: split.body.trim().to_string(),
    })
}

/// frontmatter 切分结果。
struct FrontmatterSplit {
    front: String,
    body: String,
}

/// 按行扫描切分 frontmatter 与 body。
///
/// 规则：
/// - 跳过前导空白行
/// - 第一行必须 trim 后等于 `---`（允许 `--- ` 这种带尾随空格的写法）
/// - 收集后续行到 front，直到遇到下一个 trim 后等于 `---` 的行
/// - 剩余行归入 body
/// - 找不到闭合 `---` 返回 None
fn parse_frontmatter_split(raw: &str) -> Option<FrontmatterSplit> {
    let mut lines = raw.lines();

    // 跳过前导空白行
    let first = loop {
        let line = lines.next()?;
        if !line.trim().is_empty() {
            break line;
        }
    };

    if first.trim() != "---" {
        return None;
    }

    let mut front = String::new();
    let mut body = String::new();
    let mut found_close = false;
    let mut front_first_line = true;

    for line in lines {
        if !found_close {
            if line.trim() == "---" {
                found_close = true;
                continue;
            }
            if !front_first_line {
                front.push('\n');
            }
            front.push_str(line);
            front_first_line = false;
        } else {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }

    if !found_close {
        return None;
    }
    Some(FrontmatterSplit { front, body })
}

/// 加载题材 profile。查找顺序（对齐 inkos `rules-reader.readGenreProfile`）：
/// 1. 项目级：`{book_dir}/genres/{genre_id}.md`（用户自定义覆盖内置）
/// 2. 内置：BUILTIN_GENRES 中的同名条目（编译时嵌入）
/// 3. Fallback：内置 `other.md`（通用题材）
///
/// 三层都失败时返回错误（仅当 `other.md` 内置文件本身损坏时才会发生）。
pub fn read_genre_profile(
    book_dir: &std::path::Path,
    genre_id: &str,
) -> Result<ParsedGenreProfile, AppError> {
    // 1. 项目级覆盖
    let project_path = book_dir.join("genres").join(format!("{}.md", genre_id));
    if let Ok(raw) = std::fs::read_to_string(&project_path) {
        return parse_genre_profile(&raw);
    }

    // 2. 内置
    if let Some(raw) = builtin_genre_raw(genre_id) {
        return parse_genre_profile(raw);
    }

    // 3. Fallback other
    let fallback_raw = builtin_genre_raw("other")
        .ok_or_else(|| AppError::internal("Built-in other.md genre profile is missing"))?;
    parse_genre_profile(fallback_raw)
}

/// 列出所有可用题材（项目级覆盖内置，去重）。
///
/// 对齐 inkos `listAvailableGenres`：返回 `(id, name, source)` 列表，按 id 排序。
/// 项目级文件覆盖同 id 的内置条目。
pub fn list_available_genres(book_dir: &std::path::Path) -> Vec<GenreEntry> {
    let mut results: std::collections::BTreeMap<String, GenreEntry> =
        std::collections::BTreeMap::new();

    // 内置优先（项目级会覆盖同 id 条目）
    for (id, raw) in BUILTIN_GENRES {
        if let Ok(parsed) = parse_genre_profile(raw) {
            results.entry(id.to_string()).or_insert_with(|| GenreEntry {
                id: id.to_string(),
                name: parsed.profile.name.clone(),
                source: GenreSource::Builtin,
            });
        }
    }

    // 项目级覆盖
    let project_dir = book_dir.join("genres");
    if let Ok(entries) = std::fs::read_dir(&project_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if !file_name.ends_with(".md") {
                continue;
            }
            let id = file_name.trim_end_matches(".md").to_string();
            if let Ok(raw) = std::fs::read_to_string(entry.path()) {
                if let Ok(parsed) = parse_genre_profile(&raw) {
                    results.insert(
                        id.clone(),
                        GenreEntry {
                            id,
                            name: parsed.profile.name,
                            source: GenreSource::Project,
                        },
                    );
                }
            }
        }
    }

    results.into_values().collect()
}

/// 题材条目（id + 显示名 + 来源）。
#[derive(Debug, Clone)]
pub struct GenreEntry {
    pub id: String,
    pub name: String,
    pub source: GenreSource,
}

/// 题材来源：内置或项目级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreSource {
    Builtin,
    Project,
}

/// 查找内置 genre profile 原文。
fn builtin_genre_raw(id: &str) -> Option<&'static str> {
    BUILTIN_GENRES
        .iter()
        .find(|(builtin_id, _)| *builtin_id == id)
        .map(|(_, raw)| *raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BUILTIN_GENRES 完整性 ──────────────────────────────────

    #[test]
    fn test_builtin_genres_count() {
        assert_eq!(BUILTIN_GENRES.len(), 15, "must have exactly 15 builtin genres");
    }

    #[test]
    fn test_builtin_genres_ids_unique() {
        let mut ids: Vec<&str> = BUILTIN_GENRES.iter().map(|(id, _)| *id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "builtin genre ids must be unique");
    }

    #[test]
    fn test_builtin_other_exists() {
        assert!(builtin_genre_raw("other").is_some(), "other.md fallback must exist");
    }

    // ── parse_genre_profile ────────────────────────────────────

    #[test]
    fn test_parse_xuanhuan_full_profile() {
        let raw = include_str!("genre_profiles/xuanhuan.md");
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.id, "xuanhuan");
        assert_eq!(parsed.profile.name, "玄幻");
        assert_eq!(parsed.profile.language, "zh"); // 缺省
        assert!(parsed.profile.numerical_system);
        assert!(parsed.profile.power_scaling);
        assert!(!parsed.profile.era_research);
        assert_eq!(parsed.profile.audit_dimensions.len(), 21);
        assert!(parsed.profile.audit_dimensions.contains(&4)); // 战力崩坏
        assert!(parsed.profile.audit_dimensions.contains(&5)); // 数值检查
        assert!(!parsed.profile.audit_dimensions.contains(&12)); // 年代考据未启用
        assert_eq!(parsed.profile.fatigue_words.len(), 12);
        assert!(parsed.body.contains("题材禁忌"));
        assert!(parsed.body.contains("金手指四维约束"));
    }

    #[test]
    fn test_parse_cozy_en_language() {
        let raw = include_str!("genre_profiles/cozy.md");
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.language, "en");
        assert_eq!(parsed.profile.id, "cozy");
        assert_eq!(parsed.profile.name, "Cozy Fantasy");
        assert!(!parsed.profile.numerical_system);
        assert!(!parsed.profile.power_scaling);
    }

    #[test]
    fn test_parse_other_fallback_profile() {
        let raw = include_str!("genre_profiles/other.md");
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.id, "other");
        assert_eq!(parsed.profile.name, "通用");
        assert_eq!(parsed.profile.language, "zh");
        assert_eq!(parsed.profile.audit_dimensions.len(), 18);
    }

    #[test]
    fn test_parse_urban_era_research_enabled() {
        let raw = include_str!("genre_profiles/urban.md");
        let parsed = parse_genre_profile(raw).unwrap();
        assert!(parsed.profile.era_research, "都市题材应启用年代考据");
        assert!(parsed.profile.audit_dimensions.contains(&12));
    }

    #[test]
    fn test_missing_frontmatter_returns_error() {
        let raw = "no frontmatter here\njust body";
        let result = parse_genre_profile(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_frontmatter_returns_error() {
        let raw = "---\nname: broken\nid: broken\nchapterTypes: [\"x\"]\nfatigueWords: [\"y\"]";
        let result = parse_genre_profile(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_yaml_returns_error() {
        let raw = "---\nname: [unterminated\n---\nbody";
        let result = parse_genre_profile(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_optional_fields_uses_defaults() {
        // 仅必填字段，可选字段缺失应使用 Default
        let raw = "---\nname: minimal\nid: minimal\nchapterTypes: [\"a\"]\nfatigueWords: [\"b\"]\n---\n\nbody";
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.language, "zh"); // default
        assert!(!parsed.profile.numerical_system); // default false
        assert!(!parsed.profile.power_scaling);
        assert!(!parsed.profile.era_research);
        assert_eq!(parsed.profile.pacing_rule, ""); // default empty
        assert!(parsed.profile.satisfaction_types.is_empty());
        assert!(parsed.profile.audit_dimensions.is_empty());
    }

    #[test]
    fn test_frontmatter_with_trailing_whitespace_on_fence() {
        // `--- ` 带尾随空格也应该被识别为围栏
        let raw = "---   \nname: x\nid: x\nchapterTypes: [\"a\"]\nfatigueWords: [\"b\"]\n---   \nbody";
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.id, "x");
        assert_eq!(parsed.body, "body");
    }

    #[test]
    fn test_frontmatter_with_leading_blank_lines() {
        let raw = "\n\n---\nname: x\nid: x\nchapterTypes: [\"a\"]\nfatigueWords: [\"b\"]\n---\nbody";
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.profile.id, "x");
    }

    #[test]
    fn test_body_trimmed() {
        // str::trim() 会移除前导/尾随空白（含换行和空格），但保留 body 内部的空白行
        let raw = "---\nname: x\nid: x\nchapterTypes: [\"a\"]\nfatigueWords: [\"b\"]\n---\n\n\nbody line 1  \n\nbody line 2\n\n\n";
        let parsed = parse_genre_profile(raw).unwrap();
        assert_eq!(parsed.body, "body line 1  \n\nbody line 2");
    }

    // ── 所有内置 profile 必须能解析 ────────────────────────────

    #[test]
    fn test_all_builtin_profiles_parse() {
        for (id, raw) in BUILTIN_GENRES {
            let result = parse_genre_profile(raw);
            assert!(
                result.is_ok(),
                "builtin genre '{}' failed to parse: {:?}",
                id,
                result.err()
            );
            let parsed = result.unwrap();
            assert_eq!(parsed.profile.id, *id, "genre id mismatch for {}", id);
            assert!(
                !parsed.profile.name.is_empty(),
                "genre name empty for {}",
                id
            );
            assert!(
                !parsed.profile.chapter_types.is_empty(),
                "chapter types empty for {}",
                id
            );
            assert!(
                !parsed.profile.fatigue_words.is_empty(),
                "fatigue words empty for {}",
                id
            );
            assert!(
                !parsed.profile.audit_dimensions.is_empty(),
                "audit dimensions empty for {}",
                id
            );
            assert!(
                !parsed.body.is_empty(),
                "body empty for {}",
                id
            );
        }
    }

    // ── builtin_genre_raw ──────────────────────────────────────

    #[test]
    fn test_builtin_lookup_known_id() {
        assert!(builtin_genre_raw("xuanhuan").is_some());
        assert!(builtin_genre_raw("cozy").is_some());
        assert!(builtin_genre_raw("other").is_some());
    }

    #[test]
    fn test_builtin_lookup_unknown_id() {
        assert!(builtin_genre_raw("nonexistent").is_none());
    }

    // ── read_genre_profile 三层查找 ───────────────────────────

    #[test]
    fn test_read_genre_profile_builtin() {
        let temp_dir = tempfile::tempdir().unwrap();
        // book_dir 下没有 genres/xuanhuan.md，应该回退到内置
        let result = read_genre_profile(temp_dir.path(), "xuanhuan").unwrap();
        assert_eq!(result.profile.id, "xuanhuan");
        assert_eq!(result.profile.name, "玄幻");
    }

    #[test]
    fn test_read_genre_profile_unknown_falls_back_to_other() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = read_genre_profile(temp_dir.path(), "totally-unknown-genre").unwrap();
        assert_eq!(result.profile.id, "other");
        assert_eq!(result.profile.name, "通用");
    }

    #[test]
    fn test_read_genre_profile_project_overrides_builtin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let genres_dir = temp_dir.path().join("genres");
        std::fs::create_dir_all(&genres_dir).unwrap();
        std::fs::write(
            genres_dir.join("xuanhuan.md"),
            "---\nname: 自定义玄幻\nid: xuanhuan\nchapterTypes: [\"test\"]\nfatigueWords: [\"test\"]\nnumericalSystem: false\npowerScaling: false\neraResearch: false\npacingRule: \"test\"\nsatisfactionTypes: [\"test\"]\nauditDimensions: [1]\n---\n\nbody\n",
        ).unwrap();

        let result = read_genre_profile(temp_dir.path(), "xuanhuan").unwrap();
        assert_eq!(result.profile.name, "自定义玄幻");
        assert_eq!(result.body, "body");
        assert_eq!(result.profile.audit_dimensions, vec![1]);
    }

    #[test]
    fn test_read_genre_profile_empty_id_falls_back_to_other() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = read_genre_profile(temp_dir.path(), "").unwrap();
        assert_eq!(result.profile.id, "other");
    }

    // ── list_available_genres ──────────────────────────────────

    #[test]
    fn test_list_available_genres_includes_all_builtin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let list = list_available_genres(temp_dir.path());
        assert!(list.len() >= 15, "should include at least 15 builtin genres");
        assert!(list.iter().any(|e| e.id == "xuanhuan" && e.source == GenreSource::Builtin));
        assert!(list.iter().any(|e| e.id == "other" && e.source == GenreSource::Builtin));
        assert!(list.iter().any(|e| e.id == "cozy" && e.source == GenreSource::Builtin));
    }

    #[test]
    fn test_list_available_genres_sorted_by_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let list = list_available_genres(temp_dir.path());
        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "genre list should be sorted by id");
    }

    #[test]
    fn test_list_available_genres_project_overrides_builtin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let genres_dir = temp_dir.path().join("genres");
        std::fs::create_dir_all(&genres_dir).unwrap();
        std::fs::write(
            genres_dir.join("xuanhuan.md"),
            "---\nname: 项目自定义\nid: xuanhuan\nchapterTypes: [\"x\"]\nfatigueWords: [\"y\"]\n---\nbody\n",
        ).unwrap();

        let list = list_available_genres(temp_dir.path());
        let xuanhuan = list.iter().find(|e| e.id == "xuanhuan").unwrap();
        assert_eq!(xuanhuan.name, "项目自定义");
        assert_eq!(xuanhuan.source, GenreSource::Project);
    }

    #[test]
    fn test_list_available_genres_project_adds_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let genres_dir = temp_dir.path().join("genres");
        std::fs::create_dir_all(&genres_dir).unwrap();
        std::fs::write(
            genres_dir.join("custom.md"),
            "---\nname: 自定义题材\nid: custom\nchapterTypes: [\"x\"]\nfatigueWords: [\"y\"]\n---\nbody\n",
        ).unwrap();

        let list = list_available_genres(temp_dir.path());
        assert!(list.iter().any(|e| e.id == "custom" && e.source == GenreSource::Project));
        assert!(list.len() >= 16, "should include 15 builtin + 1 project");
    }

    #[test]
    fn test_list_available_genres_ignores_non_md_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let genres_dir = temp_dir.path().join("genres");
        std::fs::create_dir_all(&genres_dir).unwrap();
        std::fs::write(genres_dir.join("notes.txt"), "not a genre profile").unwrap();
        std::fs::write(genres_dir.join("README.md"), "# not a genre").unwrap(); // 解析失败会被忽略

        let list = list_available_genres(temp_dir.path());
        assert!(!list.iter().any(|e| e.id == "notes"));
        assert!(!list.iter().any(|e| e.id == "README"));
    }
}
