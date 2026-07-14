// 风格分析器 —— 纯文本统计分析(无 LLM 依赖)。
//
// 功能:
// - 提取句长/段落长度/词汇多样性(TTR)/句首模式/修辞特征等统计指纹
// - 中英文双轨:中文按字符计量,英文按单词计量
// - 修辞模式:中文 6 种 + 英文 4 种(regex 识别)

pub mod analyzer;
pub mod commands;
pub mod types;
