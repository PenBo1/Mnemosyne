use super::loop_core::AgentLoop;

impl AgentLoop {
    pub(crate) fn build_system_prompt(&self) -> String {
        let mut parts = Vec::new();

        parts.push(
            "你是 Mnemosyne，一个专业的 AI 创作助手。你帮助用户进行小说创作、 \
             角色设计、世界观构建、情节分析和趋势研究。\
             你使用提供的工具来读取和写入文件、搜索内容、管理小说数据。\
             请用中文回复。"
                .to_string(),
        );

        parts.push(
            "## 工具使用规则\n\
             - 使用 read_file 读取文件内容\n\
             - 使用 write_file 写入文件\n\
             - 使用 grep 搜索文件内容\n\
             - 使用 glob 查找文件\n\
             - 使用 list_dir 列出目录\n\
             - 使用 novel_info 获取小说信息\n\
             - 使用 chapter_list 获取章节列表\n\
             - 使用 character_list 获取角色列表\n\
             - 使用 world_setting_list 获取世界设定\n\
             - 使用 memory_search 搜索记忆"
                .to_string(),
        );

        parts.push(format!("当前时间: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M")));

        parts.join("\n\n")
    }
}
