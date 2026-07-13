// Script/Storyboard/InteractiveFilm Runner —— 剧本/分镜/互动影游编排器。
//
// 3 个独立编排流程：
// - run_script_creation: 剧本创作（dramas/{projectId}/）
// - run_storyboard_creation: 分镜创作（storyboards/{projectId}/）
// - run_interactive_film_creation: 互动影游创作（interactive-films/{projectId}/）
//
// 文件系统操作用 std::fs 同步 API（与 short_fiction_runner.rs 一致），LLM 调用走 AgentEngine。
// 互动影游的 story-graph 生成未实现，仅落盘其他产物。

use std::path::{Path, PathBuf};
use crate::core::agent::engine::AgentEngine;
use crate::shared::error::AppError;
use super::super::agents::script_storyboard::{
    self, InteractiveFilmCreationInput, ScriptCreationInput, ScriptTargetFormat,
    StoryboardCreationInput,
};
use super::super::types::Language;

// ── StoryboardImageAsset 类型 ───────────────────────────────

/// 分镜图资产变体
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoryboardImageAssetVariant {
    pub id: String,
    pub path: String,
    pub status: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub created_at: Option<String>,
    pub error: Option<String>,
}

/// 分镜图资产
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoryboardImageAsset {
    pub shot_id: String,
    pub prompt: String,
    pub source_refs: Vec<String>,
    pub variants: Vec<StoryboardImageAssetVariant>,
    pub selected_path: Option<String>,
    pub status: String,
}

/// 分镜资产清单
#[derive(Debug, Clone, serde::Serialize)]
pub struct StoryboardAssetsManifest {
    pub version: u32,
    pub kind: String,
    pub title: String,
    pub project_id: String,
    pub base_dir: String,
    pub storyboard_path: String,
    pub image_prompts_path: String,
    pub assets_dir: String,
    pub source_dir: String,
    pub generated_dir: String,
    pub selected_dir: String,
    pub created_at: String,
    pub assets: Vec<StoryboardImageAsset>,
}

// ── 运行选项 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScriptCreationRunOptions {
    pub project_root: PathBuf,
    pub title: String,
    pub instruction: String,
    pub source_kind: Option<String>,
    pub target_format: Option<ScriptTargetFormat>,
    pub source_text: Option<String>,
    pub source_path: Option<String>,
    pub requirements: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub language: Option<Language>,
    pub project_id: Option<String>,
    pub out_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoryboardCreationRunOptions {
    pub project_root: PathBuf,
    pub title: String,
    pub instruction: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub source_path: Option<String>,
    pub requirements: Option<String>,
    pub visual_style: Option<String>,
    pub aspect_ratio: Option<String>,
    pub granularity: Option<String>,
    pub max_shots: Option<u32>,
    pub language: Option<Language>,
    pub project_id: Option<String>,
    pub out_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InteractiveFilmCreationRunOptions {
    pub project_root: PathBuf,
    pub title: String,
    pub instruction: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub source_path: Option<String>,
    pub requirements: Option<String>,
    pub target_audience: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub budget: Option<String>,
    pub reference_mode: Option<String>,
    pub language: Option<Language>,
    pub project_id: Option<String>,
    pub out_dir: Option<String>,
}

// ── 运行结果 ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScriptCreationRunResult {
    pub project_id: String,
    pub base_dir: String,
    pub spec_path: String,
    pub script_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoryboardCreationRunResult {
    pub project_id: String,
    pub base_dir: String,
    pub spec_path: String,
    pub storyboard_path: String,
    pub image_prompts_path: String,
    pub assets_manifest_path: String,
    pub assets_dir: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InteractiveFilmCreationRunResult {
    pub project_id: String,
    pub base_dir: String,
    pub story_graph_path: String,
    pub spec_path: String,
    pub story_tree_path: String,
    pub flags_path: String,
    pub script_path: String,
    pub storyboard_path: String,
    pub image_prompts_path: String,
    pub assets_manifest_path: String,
    pub assets_dir: String,
}

// ═══════════════════════════════════════════════════════════════
//  运行函数
// ═══════════════════════════════════════════════════════════════

/// 运行剧本创作。
pub async fn run_script_creation(
    engine: &AgentEngine,
    options: &ScriptCreationRunOptions,
) -> Result<ScriptCreationRunResult, AppError> {
    let language = options.language.unwrap_or_default();
    let project_id = resolve_project_id(&options.project_id, &options.title);
    let base_dir = resolve_project_base_dir(
        options.out_dir.as_deref().unwrap_or("dramas"),
        &project_id,
    )?;
    let source_text = resolve_source_text(&options.project_root, &options.source_text, &options.source_path)?;
    let input = ScriptCreationInput {
        title: options.title.clone(),
        source_kind: options.source_kind.clone(),
        target_format: options.target_format,
        source_text,
        requirements: Some(merge_requirements(&options.instruction, &options.requirements, language)),
        episode_count: options.episode_count,
        episode_duration: options.episode_duration.clone(),
        language: Some(language),
    };

    tracing::info!("[script] 写剧本创作规格");
    let spec = script_storyboard::render_script_spec(&input);
    write_project_text(&options.project_root, &format!("{}/script-spec.md", base_dir), &spec)?;

    tracing::info!("[script] 写剧本正文");
    let script = script_storyboard::write_script(engine, &input).await?;
    write_project_text(&options.project_root, &format!("{}/script.md", base_dir), &script)?;

    let status = RunStatusJson {
        status: "completed".to_string(),
        kind: "script".to_string(),
        title: options.title.clone(),
        completed_at: chrono::Utc::now().to_rfc3339(),
    };
    write_project_json(&options.project_root, &format!("{}/status.json", base_dir), &status)?;

    Ok(ScriptCreationRunResult {
        project_id,
        base_dir: base_dir.clone(),
        spec_path: rel_path(&base_dir, "script-spec.md"),
        script_path: rel_path(&base_dir, "script.md"),
    })
}

/// 运行分镜创作。
pub async fn run_storyboard_creation(
    engine: &AgentEngine,
    options: &StoryboardCreationRunOptions,
) -> Result<StoryboardCreationRunResult, AppError> {
    let language = options.language.unwrap_or_default();
    let project_id = resolve_project_id(&options.project_id, &options.title);
    let base_dir = resolve_project_base_dir(
        options.out_dir.as_deref().unwrap_or("storyboards"),
        &project_id,
    )?;
    let source_text = resolve_source_text(&options.project_root, &options.source_text, &options.source_path)?;
    let input = StoryboardCreationInput {
        title: options.title.clone(),
        source_kind: options.source_kind.clone(),
        source_text,
        requirements: Some(merge_requirements(&options.instruction, &options.requirements, language)),
        visual_style: options.visual_style.clone(),
        aspect_ratio: options.aspect_ratio.clone(),
        granularity: options.granularity.clone(),
        max_shots: options.max_shots,
        language: Some(language),
    };

    tracing::info!("[storyboard] 写分镜创作规格");
    let spec = script_storyboard::render_storyboard_spec(&input);
    write_project_text(&options.project_root, &format!("{}/storyboard-spec.md", base_dir), &spec)?;

    tracing::info!("[storyboard] 写分镜和图像提示词");
    let storyboard = script_storyboard::write_storyboard(engine, &input).await?;
    write_project_text(&options.project_root, &format!("{}/storyboard.md", base_dir), &storyboard)?;

    let prompts = script_storyboard::extract_image_prompts(&storyboard);
    let image_prompts = format_numbered_prompts(&prompts);
    write_project_text(&options.project_root, &format!("{}/image-prompts.md", base_dir), &image_prompts)?;

    ensure_project_dir(&options.project_root, &format!("{}/assets/source", base_dir))?;
    ensure_project_dir(&options.project_root, &format!("{}/assets/generated", base_dir))?;
    ensure_project_dir(&options.project_root, &format!("{}/assets/selected", base_dir))?;

    let manifest = create_storyboard_assets_manifest(
        &options.title,
        &project_id,
        &base_dir,
        &rel_path(&base_dir, "storyboard.md"),
        &rel_path(&base_dir, "image-prompts.md"),
        &prompts,
        &chrono::Utc::now().to_rfc3339(),
    );
    write_project_json(&options.project_root, &format!("{}/assets.json", base_dir), &manifest)?;

    let status = RunStatusJson {
        status: "completed".to_string(),
        kind: "storyboard".to_string(),
        title: options.title.clone(),
        completed_at: chrono::Utc::now().to_rfc3339(),
    };
    write_project_json(&options.project_root, &format!("{}/status.json", base_dir), &status)?;

    Ok(StoryboardCreationRunResult {
        project_id,
        base_dir: base_dir.clone(),
        spec_path: rel_path(&base_dir, "storyboard-spec.md"),
        storyboard_path: rel_path(&base_dir, "storyboard.md"),
        image_prompts_path: rel_path(&base_dir, "image-prompts.md"),
        assets_manifest_path: rel_path(&base_dir, "assets.json"),
        assets_dir: rel_path(&base_dir, "assets"),
    })
}

/// 运行互动影游创作。
/// story-graph 生成未实现，仅落盘其他产物。
pub async fn run_interactive_film_creation(
    engine: &AgentEngine,
    options: &InteractiveFilmCreationRunOptions,
) -> Result<InteractiveFilmCreationRunResult, AppError> {
    let language = options.language.unwrap_or_default();
    let project_id = resolve_project_id(&options.project_id, &options.title);
    let base_dir = resolve_project_base_dir(
        options.out_dir.as_deref().unwrap_or("interactive-films"),
        &project_id,
    )?;
    let source_text = resolve_source_text(&options.project_root, &options.source_text, &options.source_path)?;
    let input = InteractiveFilmCreationInput {
        title: options.title.clone(),
        source_kind: options.source_kind.clone(),
        source_text,
        requirements: Some(merge_requirements(&options.instruction, &options.requirements, language)),
        target_audience: options.target_audience.clone(),
        episode_count: options.episode_count,
        episode_duration: options.episode_duration.clone(),
        budget: options.budget.clone(),
        reference_mode: options.reference_mode.clone(),
        language: Some(language),
    };

    tracing::info!("[interactive-film] 写互动影游创作规格");
    let spec = script_storyboard::render_interactive_film_spec(&input);
    write_project_text(&options.project_root, &format!("{}/interactive-spec.md", base_dir), &spec)?;

    tracing::info!("[interactive-film] 写剧情树、旗标、剧本、分镜和图像提示词");
    let package_markdown = script_storyboard::write_interactive_film(engine, &input).await?;

    let story_tree = required_section(
        &package_markdown,
        &["剧情树", "Story Tree", "Branching Story Tree"],
        &package_markdown,
    );
    let flags = required_section(
        &package_markdown,
        &[
            "旗标与变量系统说明",
            "变量与旗标表",
            "变量和旗标表",
            "变量表",
            "旗标表",
            "Variables and Flags",
            "Flag Table",
        ],
        &package_markdown,
    );
    let script = required_section(
        &package_markdown,
        &["互动剧本", "Interactive Script", "Script"],
        &package_markdown,
    );
    let storyboard = required_section(
        &package_markdown,
        &[
            "分镜与图像提示词",
            "分镜表",
            "Storyboard and Image Prompts",
            "Storyboard",
        ],
        &package_markdown,
    );

    let prompts = script_storyboard::extract_image_prompts(&storyboard);
    let image_prompts = format_numbered_prompts(&prompts);

    let story_graph_path = rel_path(&base_dir, "story-graph.json");

    write_project_text(&options.project_root, &format!("{}/story-tree.md", base_dir), &story_tree)?;
    write_project_text(&options.project_root, &format!("{}/flags.md", base_dir), &flags)?;
    let normalized_script = script_storyboard::normalize_episode_end_labels(&script, 1);
    write_project_text(&options.project_root, &format!("{}/script.md", base_dir), &normalized_script)?;
    write_project_text(&options.project_root, &format!("{}/storyboard.md", base_dir), &storyboard)?;
    write_project_text(&options.project_root, &format!("{}/image-prompts.md", base_dir), &image_prompts)?;

    ensure_project_dir(&options.project_root, &format!("{}/assets/source", base_dir))?;
    ensure_project_dir(&options.project_root, &format!("{}/assets/generated", base_dir))?;
    ensure_project_dir(&options.project_root, &format!("{}/assets/selected", base_dir))?;

    let manifest = create_storyboard_assets_manifest(
        &options.title,
        &project_id,
        &base_dir,
        &rel_path(&base_dir, "storyboard.md"),
        &rel_path(&base_dir, "image-prompts.md"),
        &prompts,
        &chrono::Utc::now().to_rfc3339(),
    );
    write_project_json(&options.project_root, &format!("{}/assets.json", base_dir), &manifest)?;

    // story-graph 生成未实现
    tracing::info!("[interactive-film] story-graph 生成未实现，跳过 story-graph.json");

    let status = RunStatusJson {
        status: "completed".to_string(),
        kind: "interactive_film".to_string(),
        title: options.title.clone(),
        completed_at: chrono::Utc::now().to_rfc3339(),
    };
    write_project_json(&options.project_root, &format!("{}/status.json", base_dir), &status)?;

    Ok(InteractiveFilmCreationRunResult {
        project_id,
        base_dir: base_dir.clone(),
        story_graph_path,
        spec_path: rel_path(&base_dir, "interactive-spec.md"),
        story_tree_path: rel_path(&base_dir, "story-tree.md"),
        flags_path: rel_path(&base_dir, "flags.md"),
        script_path: rel_path(&base_dir, "script.md"),
        storyboard_path: rel_path(&base_dir, "storyboard.md"),
        image_prompts_path: rel_path(&base_dir, "image-prompts.md"),
        assets_manifest_path: rel_path(&base_dir, "assets.json"),
        assets_dir: rel_path(&base_dir, "assets"),
    })
}

// ═══════════════════════════════════════════════════════════════
//  资产清单构造（public）
// ═══════════════════════════════════════════════════════════════

/// 构造分镜资产清单。
pub fn create_storyboard_assets_manifest(
    title: &str,
    project_id: &str,
    base_dir: &str,
    storyboard_path: &str,
    image_prompts_path: &str,
    image_prompts: &[String],
    created_at: &str,
) -> StoryboardAssetsManifest {
    let assets_dir = format!("{}/assets", base_dir);
    let assets = image_prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| StoryboardImageAsset {
            shot_id: format!("shot-{:03}", index + 1),
            prompt: prompt.clone(),
            source_refs: Vec::new(),
            variants: Vec::new(),
            selected_path: None,
            status: "prompt_ready".to_string(),
        })
        .collect();

    StoryboardAssetsManifest {
        version: 1,
        kind: "storyboard_assets".to_string(),
        title: title.to_string(),
        project_id: project_id.to_string(),
        base_dir: to_posix_str(base_dir),
        storyboard_path: to_posix_str(storyboard_path),
        image_prompts_path: to_posix_str(image_prompts_path),
        assets_dir: to_posix_str(&assets_dir),
        source_dir: format!("{}/source", to_posix_str(&assets_dir)),
        generated_dir: format!("{}/generated", to_posix_str(&assets_dir)),
        selected_dir: format!("{}/selected", to_posix_str(&assets_dir)),
        created_at: created_at.to_string(),
        assets,
    }
}

// ═══════════════════════════════════════════════════════════════
//  内部辅助函数（private）
// ═══════════════════════════════════════════════════════════════

/// 运行状态 JSON
#[derive(Debug, serde::Serialize)]
struct RunStatusJson {
    status: String,
    kind: String,
    title: String,
    completed_at: String,
}

/// 解析项目 ID。
fn resolve_project_id(project_id: &Option<String>, title: &str) -> String {
    match project_id {
        Some(id) if !id.trim().is_empty() => safe_segment(id),
        _ => safe_segment(&slugify(title)),
    }
}

/// 规范化输出目录。
fn normalize_output_dir(value: &str) -> Result<String, AppError> {
    let text = value.trim().trim_matches(|c| c == '/');
    if text.is_empty() || text.contains("..") || text.contains('\0') {
        return Err(AppError::invalid_input(format!(
            "Invalid output directory: {}",
            value
        )));
    }
    Ok(text.to_string())
}

/// 解析项目基础目录。
fn resolve_project_base_dir(out_dir: &str, project_id: &str) -> Result<String, AppError> {
    let output_dir = normalize_output_dir(out_dir)?;
    let basename = Path::new(&output_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    Ok(if basename == project_id {
        output_dir
    } else {
        format!("{}/{}", output_dir, project_id)
    })
}

/// 解析源文本。
fn resolve_source_text(
    project_root: &Path,
    source_text: &Option<String>,
    source_path: &Option<String>,
) -> Result<Option<String>, AppError> {
    if let Some(text) = source_text.as_deref() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }
    if let Some(path) = source_path.as_deref() {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            if trimmed.contains("..") {
                return Err(AppError::path_traversal());
            }
            let full = project_root.join(trimmed);
            let content = std::fs::read_to_string(&full)
                .map_err(|_| AppError::file_not_found(full.to_string_lossy().to_string()))?;
            return Ok(Some(content));
        }
    }
    Ok(None)
}

/// 合并用户指令和补充要求。
fn merge_requirements(
    instruction: &str,
    requirements: &Option<String>,
    language: Language,
) -> String {
    let extra_label = if language == Language::En {
        "Additional requirements:"
    } else {
        "补充要求："
    };
    let instruction_trimmed = instruction.trim();
    match requirements
        .as_deref()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
    {
        Some(req) => format!("{}\n\n{}\n{}", instruction_trimmed, extra_label, req),
        None => instruction_trimmed.to_string(),
    }
}

/// 尝试多个标题抽取小节，找不到则用 fallback。
fn required_section(raw: &str, headings: &[&str], fallback: &str) -> String {
    for heading in headings {
        if let Some(section) = script_storyboard::extract_markdown_section(raw, heading) {
            let trimmed = section.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    fallback.trim().to_string()
}

/// 格式化编号提示词列表。
fn format_numbered_prompts(prompts: &[String]) -> String {
    if prompts.is_empty() {
        return String::new();
    }
    prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| format!("{}. {}", index + 1, prompt))
        .collect::<Vec<_>>()
        .join("\n")
}

/// slugify。
fn slugify(value: &str) -> String {
    let lower: String = value.to_lowercase();
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in lower.chars() {
        if c == '\'' || c == '"' {
            continue;
        }
        if c.is_alphanumeric() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    let truncated: String = trimmed.chars().take(60).collect();
    if truncated.is_empty() {
        format!("script-{}", chrono::Utc::now().timestamp_millis())
    } else {
        truncated
    }
}

/// 文件系统安全段。
fn safe_segment(value: &str) -> String {
    let after_dangerous: String = value
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '\0' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect();
    let after_ws = regex::Regex::new(r"\s+")
        .map(|re| re.replace_all(&after_dangerous, "-").to_string())
        .unwrap_or(after_dangerous);
    let trimmed = after_ws.trim_matches('-');
    let truncated: String = trimmed.chars().take(80).collect();
    if truncated.is_empty() || truncated == "." || truncated == ".." {
        format!("script-{}", chrono::Utc::now().timestamp_millis())
    } else {
        truncated
    }
}

/// POSIX 路径。
fn to_posix_str(path: &str) -> String {
    path.replace('\\', "/")
}

/// 拼接相对路径。
fn rel_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        to_posix_str(name)
    } else {
        format!("{}/{}", to_posix_str(base), to_posix_str(name))
    }
}

/// 写项目文本文件。
fn write_project_text(
    project_root: &Path,
    relative_path: &str,
    content: &str,
) -> Result<(), AppError> {
    if relative_path.contains("..") {
        return Err(AppError::path_traversal());
    }
    let full = project_root.join(relative_path);
    write_text(&full, content)
}

/// 写项目 JSON 文件。
fn write_project_json<T: serde::Serialize>(
    project_root: &Path,
    relative_path: &str,
    value: &T,
) -> Result<(), AppError> {
    if relative_path.contains("..") {
        return Err(AppError::path_traversal());
    }
    let full = project_root.join(relative_path);
    let content = serde_json::to_string_pretty(value)?;
    write_text(&full, &content)
}

/// 确保项目目录存在。
fn ensure_project_dir(project_root: &Path, relative_path: &str) -> Result<(), AppError> {
    if relative_path.contains("..") {
        return Err(AppError::path_traversal());
    }
    let full = project_root.join(relative_path);
    std::fs::create_dir_all(&full)?;
    Ok(())
}

/// 写文本文件（创建父目录，末尾加换行）。
fn write_text(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", content.trim_end()))?;
    Ok(())
}

// ── 单元测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── slugify ──

    #[test]
    fn slugify_lowercases_and_dashes() {
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    #[test]
    fn slugify_preserves_cjk() {
        assert_eq!(slugify("暗流 涌动"), "暗流-涌动");
    }

    #[test]
    fn slugify_truncates_to_60() {
        let long = "a".repeat(100);
        let result = slugify(&long);
        assert_eq!(result.len(), 60);
    }

    #[test]
    fn slugify_empty_returns_fallback() {
        let result = slugify("!!!");
        assert!(result.starts_with("script-"), "expected fallback, got: {}", result);
    }

    // ── safe_segment ──

    #[test]
    fn safe_segment_replaces_dangerous_chars() {
        assert_eq!(safe_segment("a/b:c"), "a-b-c");
    }

    #[test]
    fn safe_segment_truncates_to_80() {
        let long = "a".repeat(100);
        let result = safe_segment(&long);
        assert_eq!(result.len(), 80);
    }

    #[test]
    fn safe_segment_rejects_dot() {
        let result = safe_segment(".");
        assert!(result.starts_with("script-"));
    }

    // ── normalize_output_dir ──

    #[test]
    fn normalize_output_dir_trims_slashes() {
        assert_eq!(normalize_output_dir("/dramas/").unwrap(), "dramas");
    }

    #[test]
    fn normalize_output_dir_rejects_traversal() {
        assert!(normalize_output_dir("../etc").is_err());
    }

    #[test]
    fn normalize_output_dir_rejects_empty() {
        assert!(normalize_output_dir("").is_err());
        assert!(normalize_output_dir("/").is_err());
    }

    // ── resolve_project_base_dir ──

    #[test]
    fn resolve_project_base_dir_appends_project_id() {
        let result = resolve_project_base_dir("dramas", "my-project").unwrap();
        assert_eq!(result, "dramas/my-project");
    }

    #[test]
    fn resolve_project_base_dir_keeps_when_basename_matches() {
        let result = resolve_project_base_dir("dramas/my-project", "my-project").unwrap();
        assert_eq!(result, "dramas/my-project");
    }

    // ── merge_requirements ──

    #[test]
    fn merge_requirements_zh_appends_extra() {
        let result = merge_requirements("主线指令", &Some("额外要求".to_string()), Language::Zh);
        assert!(result.contains("主线指令"));
        assert!(result.contains("补充要求："));
        assert!(result.contains("额外要求"));
    }

    #[test]
    fn merge_requirements_en_uses_english_label() {
        let result = merge_requirements("main instruction", &Some("extra".to_string()), Language::En);
        assert!(result.contains("Additional requirements:"));
        assert!(result.contains("extra"));
    }

    #[test]
    fn merge_requirements_empty_requirements_returns_instruction() {
        let result = merge_requirements("指令", &None, Language::Zh);
        assert_eq!(result, "指令");
    }

    // ── required_section ──

    #[test]
    fn required_section_finds_first_matching_heading() {
        let markdown = "## 剧情树\n\ntree content\n\n## 其他\n\nother\n";
        let result = required_section(markdown, &["剧情树", "Story Tree"], "fallback");
        assert!(result.contains("tree content"));
    }

    #[test]
    fn required_section_falls_back_when_not_found() {
        let markdown = "## Other\n\ncontent\n";
        let result = required_section(markdown, &["剧情树"], "fallback content");
        assert_eq!(result, "fallback content");
    }

    // ── create_storyboard_assets_manifest ──

    #[test]
    fn manifest_has_correct_metadata() {
        let prompts = vec!["sunset".to_string(), "night".to_string()];
        let manifest = create_storyboard_assets_manifest(
            "Test",
            "proj-1",
            "storyboards/proj-1",
            "storyboards/proj-1/storyboard.md",
            "storyboards/proj-1/image-prompts.md",
            &prompts,
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.kind, "storyboard_assets");
        assert_eq!(manifest.title, "Test");
        assert_eq!(manifest.project_id, "proj-1");
        assert_eq!(manifest.assets.len(), 2);
        assert_eq!(manifest.assets[0].shot_id, "shot-001");
        assert_eq!(manifest.assets[0].prompt, "sunset");
        assert_eq!(manifest.assets[0].status, "prompt_ready");
        assert_eq!(manifest.assets[1].shot_id, "shot-002");
        assert_eq!(manifest.assets[1].prompt, "night");
        assert_eq!(manifest.source_dir, "storyboards/proj-1/assets/source");
        assert_eq!(manifest.generated_dir, "storyboards/proj-1/assets/generated");
        assert_eq!(manifest.selected_dir, "storyboards/proj-1/assets/selected");
    }

    #[test]
    fn manifest_empty_prompts() {
        let manifest = create_storyboard_assets_manifest(
            "Test",
            "proj-1",
            "storyboards/proj-1",
            "storyboards/proj-1/storyboard.md",
            "storyboards/proj-1/image-prompts.md",
            &[],
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(manifest.assets.len(), 0);
    }

    // ── format_numbered_prompts ──

    #[test]
    fn format_numbered_prompts_empty() {
        assert_eq!(format_numbered_prompts(&[]), "");
    }

    #[test]
    fn format_numbered_prompts_lists() {
        let prompts = vec!["a".to_string(), "b".to_string()];
        let result = format_numbered_prompts(&prompts);
        assert_eq!(result, "1. a\n2. b");
    }

    // ── to_posix_str ──

    #[test]
    fn to_posix_str_converts_backslashes() {
        assert_eq!(to_posix_str("dramas\\proj-1"), "dramas/proj-1");
    }

    // ── rel_path ──

    #[test]
    fn rel_path_joins_segments() {
        assert_eq!(rel_path("dramas/proj-1", "script.md"), "dramas/proj-1/script.md");
    }

    #[test]
    fn rel_path_empty_base() {
        assert_eq!(rel_path("", "script.md"), "script.md");
    }

    // ── resolve_project_id ──

    #[test]
    fn resolve_project_id_uses_provided() {
        let result = resolve_project_id(&Some("custom-id".to_string()), "Title");
        assert_eq!(result, "custom-id");
    }

    #[test]
    fn resolve_project_id_slugifies_title() {
        let result = resolve_project_id(&None, "My Cool Drama");
        assert_eq!(result, "my-cool-drama");
    }

    #[test]
    fn resolve_project_id_empty_provided_falls_back() {
        let result = resolve_project_id(&Some("".to_string()), "My Drama");
        assert_eq!(result, "my-drama");
    }
}
