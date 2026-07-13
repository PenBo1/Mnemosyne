// AI 设置相关类型 —— 单一真理源位于 @/shared/settings（含运行时函数）。
// 此处仅做 type-only 重导出，避免重复定义导致 barrel 聚合时重名冲突。
export type { AiModelConfig, AppSettings, LogLevel } from "@/services/settings";
