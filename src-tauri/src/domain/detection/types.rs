// 检测系统类型定义。所有结构使用 camelCase 序列化。

/// 单次检测结果。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    /// 0-1,越高越像 AI 生成
    pub score: f64,
    /// gptzero / originality / custom
    pub provider: String,
    /// 检测时间(ISO 8601)
    pub detected_at: String,
    /// 原始响应(可选,便于前端展示细节)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// 检测历史条目(按 book 落盘)。
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectionHistoryEntry {
    pub book_id: String,
    pub chapter_number: u32,
    /// detect / rewrite
    pub action: String,
    /// 同一 chapter 的第几次尝试(从 1 起)
    pub attempt: u32,
    pub score: f64,
    pub provider: String,
    pub detected_at: String,
}

/// 单章检测统计行。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChapterDetectionRow {
    pub chapter_number: u32,
    pub original_score: f64,
    pub final_score: f64,
    pub rewrite_attempts: u32,
}

/// 检测聚合统计。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectionStats {
    pub total_detections: u32,
    pub total_rewrites: u32,
    pub avg_original_score: f64,
    pub avg_final_score: f64,
    pub pass_rate: f64,
    pub chapter_breakdown: Vec<ChapterDetectionRow>,
}
