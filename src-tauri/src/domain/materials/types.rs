//! ═══════════════════════════════════════════════════════════════════════════
//! 材料类型 - 材料系统类型定义
//! ═══════════════════════════════════════════════════════════════════════════

// ── 类型定义 ────────────────────────────────────────────────────────────────

/// 已导入的材料资产
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MaterialAsset {
    pub id: String,
    pub title: String,
    /// webpage / pdf / text
    pub kind: String,
    /// reference / worldbuilding / script / storyboard / research / general
    pub purpose: String,
    pub source: String,
    pub mime_type: String,
    /// 相对 materials_dir 的路径(如 `<id>.md`)。
    pub markdown_path: String,
    pub manifest_path: String,
    pub char_count: u32,
    pub excerpt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<u32>,
}

/// 导入材料输入(前端 camelCase)。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestMaterialInput {
    /// url / file
    pub source_kind: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
}

/// 检索材料输入(前端 camelCase)。
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveMaterialsInput {
    pub query: String,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

/// 检索命中的材料片段。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedMaterial {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub purpose: String,
    pub source: String,
    pub markdown_path: String,
    pub score: f64,
    pub excerpt: String,
    pub char_start: u32,
    pub char_end: u32,
}
