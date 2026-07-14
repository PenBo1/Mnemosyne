// AIGC 检测系统 —— 调用外部检测 API(GPTZero / Originality / 自定义 endpoint)。
//
// 模块职责:
// - detector: 三种 provider,统一归一化为 0-1 分数(越高越像 AI)
// - insights: 按 chapter 分组聚合历史,计算 pass_rate / 均分等统计
//
// 安全约束:
// - 检测 API key 不经 IPC 传递,由命令层从平台密钥存储(secrets)服务端解析
// - 检测历史按 book_id 落盘到 detection_dir,book_id 须通过路径校验

pub mod commands;
pub mod detector;
pub mod insights;
pub mod store;
pub mod types;

/// 密钥在 secrets 存储中的 service 名(供 detection_scan 解析 API key)。
pub const DETECTION_SECRET_SERVICE: &str = "mnemosyne-detection";
