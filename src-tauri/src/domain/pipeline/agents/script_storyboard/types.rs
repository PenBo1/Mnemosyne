//! ═══════════════════════════════════════════════════════════════════════════
//! Script/Storyboard Types - 类型定义
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! 职责：剧本 / 分镜 / 互动影游创作的输入结构与目标格式定义。

use crate::domain::pipeline::types::Language;

// ── ScriptTargetFormat ──────────────────────────────────────────────────────

/// 剧本目标格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptTargetFormat {
    VerticalShortDrama,
    Screenplay,
    AudioDrama,
    InteractiveScript,
    GeneralScript,
}

impl Default for ScriptTargetFormat {
    fn default() -> Self {
        Self::GeneralScript
    }
}

// ── 输入结构 ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScriptCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub target_format: Option<ScriptTargetFormat>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub language: Option<Language>,
}

#[derive(Debug, Clone)]
pub struct StoryboardCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub visual_style: Option<String>,
    pub aspect_ratio: Option<String>,
    pub granularity: Option<String>,
    pub max_shots: Option<u32>,
    pub language: Option<Language>,
}

#[derive(Debug, Clone)]
pub struct InteractiveFilmCreationInput {
    pub title: String,
    pub source_kind: Option<String>,
    pub source_text: Option<String>,
    pub requirements: Option<String>,
    pub target_audience: Option<String>,
    pub episode_count: Option<u32>,
    pub episode_duration: Option<String>,
    pub budget: Option<String>,
    pub reference_mode: Option<String>,
    pub language: Option<Language>,
}
