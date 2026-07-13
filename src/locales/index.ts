import en from "./en";
import zh from "./zh";

export type Locale = "en" | "zh";

export type TranslationKeys = typeof en;

type DeepReplace<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends (infer U)[]
      ? DeepReplace<U>[]
      : T extends object
        ? { [K in keyof T]: DeepReplace<T[K]> }
        : T;

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