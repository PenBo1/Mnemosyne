// Effort Level —— Agent 投入程度控制
//
// - Model 决定"能力"(会不会),Effort 决定"态度"(愿不愿意努力做)
// - 小模型 + 高 Effort 可能比大模型 + 低 Effort 效果更好
//
// 四档:
// - low:快速回复,最小工具调用,适合简单问答
// - medium:默认,平衡速度与质量
// - high:深度分析,多轮验证,适合复杂任务
// - ultra:ultracode 模式,多 agent 并行,适合超大型任务
//
// 行为参数(由 Rust 端 EffortLevel::params() 提供):
// - low:    5 轮工具 / 10 文件 / 0 测试 / 0 验证 / 2k tokens
// - medium: 20 轮工具 / 50 文件 / 1 测试 / 1 验证 / 8k tokens
// - high:   50 轮工具 / 200 文件 / 3 测试 / 2 验证 / 16k tokens
// - ultra:  100 轮工具 / 1000 文件 / 5 测试 / 3 验证 / 32k tokens

export type EffortLevel = "low" | "medium" | "high" | "ultra";

export const DEFAULT_EFFORT: EffortLevel = "medium";

/** Effort 选项(仅 value,标签和描述通过 i18n 解析,避免硬编码语言字符串) */
export const EFFORT_OPTIONS: ReadonlyArray<{ value: EffortLevel }> = [
  { value: "low" },
  { value: "medium" },
  { value: "high" },
  { value: "ultra" },
];

/** i18n key 前缀,实际 key 为 `agentChat.effort.{value}.label` / `.description` */
export const EFFORT_I18N_PREFIX = "effortOptions";

export function isValidEffort(s: unknown): s is EffortLevel {
  return s === "low" || s === "medium" || s === "high" || s === "ultra";
}
