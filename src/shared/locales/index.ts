import en from "./en";
import zh from "./zh";

export type Locale = "en" | "zh";

// en 作为 schema 来源（source of truth）
export type TranslationKeys = typeof en;

// 递归将字面量类型替换为 string/number 宽类型，用于 zh 与 en 的形状一致性校验
// 避免 as const 导致的字面量类型不兼容（中英文值必然不同）
type DeepReplace<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends (infer U)[]
      ? DeepReplace<U>[]
      : T extends object
        ? { [K in keyof T]: DeepReplace<T[K]> }
        : T;

// 编译时强制 zh 与 en 形状一致：键集相同、结构相同（值类型放宽为 string）
type ShapeOf<T> = DeepReplace<T>;
const _typeCheck: ShapeOf<TranslationKeys> = zh;
void _typeCheck;

export const locales: Record<Locale, ShapeOf<TranslationKeys>> = {
  en,
  zh,
};

export function getLocaleLabel(locale: Locale): string {
  const labels: Record<Locale, string> = {
    en: "English",
    zh: "中文",
  };
  return labels[locale];
}
