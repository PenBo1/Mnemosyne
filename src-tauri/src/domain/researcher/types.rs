// 研究员系统类型定义。所有结构使用 camelCase 序列化。

/// 研究来源(LLM 综合知识时引用的来源,可能为空)。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSource {
    pub title: String,
    pub url: String,
    pub snippet: String,
    /// 发布日期(ISO 8601,LLM 可能无法提供,留空)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

/// 研究论断。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResearchClaim {
    pub claim: String,
    pub sources: Vec<ResearchSource>,
    /// 0.0-1.0
    pub confidence: f64,
}

/// 研究报告。
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResearchReport {
    pub query: String,
    /// quick / standard / deep
    pub depth: String,
    pub claims: Vec<ResearchClaim>,
    pub conflicts: Vec<String>,
    pub unknowns: Vec<String>,
    pub creative_implications: Vec<String>,
    /// Markdown 渲染结果
    pub markdown: String,
}

/// 研究请求(前端 camelCase)。
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResearchInput {
    pub query: String,
    /// quick / standard / deep,缺省为 standard
    #[serde(default)]
    pub depth: Option<String>,
}
