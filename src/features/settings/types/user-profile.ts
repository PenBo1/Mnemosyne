// 用户画像前端类型 —— 对应 Rust 的 `domain/user/types.rs::UserProfile`。
//
// 注意：后端 UserProfile / WritingStyle 结构体未添加 `#[serde(rename_all = "camelCase")]`，
// 因此 serde 按字段原名（snake_case）序列化。为遵循 AGENTS.md
// "Payload 和 IPC 字段语义必须一致，禁止读写语义错配" 的约束，
// 此处 TS 字段名与后端保持一致（snake_case），不进行 camelCase 转换。
//
// ReaderType 是 Rust enum，serde 默认序列化规则：
// - 单元变体 → 变体名字符串："YoungAdult" / "General" / "Literary" / "Genre" / "WebNovel"
// - 新类型变体 Custom(String) → {"Custom": "..."}

/** 写作风格配置（对应后端 WritingStyle） */
export interface WritingStyle {
  /** 正式程度（standard/formal/casual） */
  formality: string;
  /** 节奏（fast/moderate/slow） */
  pacing: string;
  /** 描写密度（sparse/moderate/dense） */
  description_density: string;
  /** 对话风格（natural/stylized/minimal） */
  dialogue_style: string;
}

/** 读者类型（对应后端 ReaderType enum 的 serde 序列化形式） */
export type ReaderType =
  | "YoungAdult"
  | "General"
  | "Literary"
  | "Genre"
  | "WebNovel"
  | { Custom: string };

/** 字数偏好配置（对应后端 WordCountPreference） */
export interface WordCountPreference {
  /** 最小字数 */
  min_words: number;
  /** 最大字数 */
  max_words: number;
  /** 目标字数 */
  target_words: number;
}

/** 用户画像（对应后端 UserProfile） */
export interface UserProfile {
  /** 用户名称 */
  name: string;
  /** 语言偏好 */
  language: string;
  /** 写作风格 */
  style: WritingStyle;
  /** 读者类型 */
  reader_type: ReaderType;
  /** 喜好题材 */
  genres: string[];
  /** 自定义指令 */
  custom_instructions: string[];
  /** 语调偏好 */
  tone: string | null;
  /** 字数偏好 */
  word_count_preference: WordCountPreference | null;
}

/** ReaderType 的 5 个标准枚举值（不含 Custom） */
export const READER_TYPE_VARIANTS = [
  "YoungAdult",
  "General",
  "Literary",
  "Genre",
  "WebNovel",
] as const;

/** 判断 ReaderType 是否为 Custom 变体 */
export function isCustomReaderType(rt: ReaderType): rt is { Custom: string } {
  return typeof rt === "object" && rt !== null && "Custom" in rt;
}

/** 获取 ReaderType 的字符串标识（用于 select value） */
export function readerTypeKey(rt: ReaderType): string {
  if (isCustomReaderType(rt)) return "Custom";
  return rt;
}

/** 获取 Custom 类型的自定义文本（非 Custom 返回空串） */
export function customReaderTypeValue(rt: ReaderType): string {
  if (isCustomReaderType(rt)) return rt.Custom;
  return "";
}

/** 构造默认 UserProfile（对齐后端 Default 实现） */
export function defaultUserProfile(): UserProfile {
  return {
    name: "Writer",
    language: "auto",
    style: {
      formality: "standard",
      pacing: "moderate",
      description_density: "moderate",
      dialogue_style: "natural",
    },
    reader_type: "General",
    genres: [],
    custom_instructions: [],
    tone: null,
    word_count_preference: null,
  };
}
