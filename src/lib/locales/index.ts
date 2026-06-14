import en from "./en";
import zh from "./zh";

export type Locale = "en" | "zh";

export type TranslationKeys = typeof en;

export const locales: Record<Locale, TranslationKeys> = {
  en,
  zh: zh as unknown as TranslationKeys,
};

export function getLocaleLabel(locale: Locale): string {
  const labels: Record<Locale, string> = {
    en: "English",
    zh: "中文",
  };
  return labels[locale];
}
