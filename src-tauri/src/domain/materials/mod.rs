// 辅助材料系统 —— 导入外部参考资料(URL/文件)并按关键词检索。
//
// 模块职责:
// - ingest: 抓取/读取来源 → 按 mime 分发解析(HTML 剥 script/style / 文本直读 / PDF 抽取)
//           → 生成 id → 写入 materials 目录(markdown 正文 + JSON 清单)
// - retrieve: 小写化 + Unicode 词分割 → 评分(title/source/正文命中) → 片段

pub mod commands;
pub mod ingest;
pub mod retrieve;
pub mod types;
