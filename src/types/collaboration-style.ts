// Collaboration Style —— 协作风格系统
//
// 四种风格:
// - efficient:高效极简 —— 简洁直接,聚焦解决问题
// - thoughtful:深思熟虑 —— 充分分析,权衡取舍
// - patient:温和耐心 —— 循序渐进,解释原理
// - decisive:果断执行 —— 行动导向,快速决策
//
// 与 Effort 正交:Effort 控制"做多少",Style 控制"怎么做"
// 风格对应的 prompt 片段会合并到 custom_instructions 注入 system prompt

export type CollaborationStyle = "efficient" | "thoughtful" | "patient" | "decisive";

export const DEFAULT_COLLABORATION_STYLE: CollaborationStyle = "efficient";

/** 风格选项(仅 value,标签和描述通过 i18n 解析,避免硬编码语言字符串) */
export const COLLABORATION_STYLE_OPTIONS: ReadonlyArray<{ value: CollaborationStyle }> = [
  { value: "efficient" },
  { value: "thoughtful" },
  { value: "patient" },
  { value: "decisive" },
];

/** i18n key 前缀,实际 key 为 `agentChat.collaborationStyleOptions.{value}.label` / `.description` */
export const COLLABORATION_STYLE_I18N_PREFIX = "collaborationStyleOptions";

/** Rust 端 from_str 也接受 "minimal" 作为 efficient 的别名 */
export function isValidCollaborationStyle(s: unknown): s is CollaborationStyle {
  return (
    s === "efficient" ||
    s === "thoughtful" ||
    s === "patient" ||
    s === "decisive"
  );
}
