// Length Governance zod schemas —— 纯 zod 定义。
//
// 定义字数治理的 4 个数据结构：
// - LengthCountingMode：计数模式（中文字符 / 英文单词）
// - LengthNormalizeMode：归一化模式（扩写 / 压缩 / 不动）
// - LengthSpec：章节字数规格（目标 + 软硬区间）
// - LengthTelemetry：字数遥测（写作过程字数变化）
// - LengthWarning：字数告警

import { z } from "zod";

export const LengthCountingModeSchema = z.enum(["zh_chars", "en_words"]);
export type LengthCountingMode = z.infer<typeof LengthCountingModeSchema>;

export const LengthNormalizeModeSchema = z.enum(["expand", "compress", "none"]);
export type LengthNormalizeMode = z.infer<typeof LengthNormalizeModeSchema>;

export const LengthSpecSchema = z.object({
  target: z.number().int().min(1),
  softMin: z.number().int().min(1),
  softMax: z.number().int().min(1),
  hardMin: z.number().int().min(1),
  hardMax: z.number().int().min(1),
  countingMode: LengthCountingModeSchema,
  normalizeMode: LengthNormalizeModeSchema,
});

export type LengthSpec = z.infer<typeof LengthSpecSchema>;

export const LengthTelemetrySchema = z.object({
  target: z.number().int().min(1),
  softMin: z.number().int().min(1),
  softMax: z.number().int().min(1),
  hardMin: z.number().int().min(1),
  hardMax: z.number().int().min(1),
  countingMode: LengthCountingModeSchema,
  writerCount: z.number().int().min(0),
  postWriterNormalizeCount: z.number().int().min(0),
  postReviseCount: z.number().int().min(0),
  finalCount: z.number().int().min(0),
  normalizeApplied: z.boolean(),
  lengthWarning: z.boolean(),
});

export type LengthTelemetry = z.infer<typeof LengthTelemetrySchema>;

export const LengthWarningSchema = z.object({
  chapter: z.number().int().min(1),
  target: z.number().int().min(1),
  actual: z.number().int().min(0),
  countingMode: LengthCountingModeSchema,
  reason: z.string().min(1),
});

export type LengthWarning = z.infer<typeof LengthWarningSchema>;