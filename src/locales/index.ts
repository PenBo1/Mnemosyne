import en from "./en";

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

/** 内置默认翻译（en），始终同步可用 */
export const builtinTranslations: ShapeOf<TranslationKeys> = en;

/** 已加载的 locale 缓存 */
const loadedLocales = new Map<Locale, ShapeOf<TranslationKeys>>();
loadedLocales.set("en", en);

/** 异步加载 locale（zh 按需动态 import，减小首屏 bundle） */
export async function loadLocale(locale: Locale): Promise<ShapeOf<TranslationKeys>> {
  const cached = loadedLocales.get(locale);
  if (cached) return cached;

  if (locale === "zh") {
    const mod = await import("./zh");
    const zh = mod.default as ShapeOf<TranslationKeys>;
    loadedLocales.set(locale, zh);
    return zh;
  }

  loadedLocales.set("en", en);
  return en;
}

export function getLocaleLabel(locale: Locale): string {
  const labels: Record<Locale, string> = {
    en: "English",
    zh: "中文",
  };
  return labels[locale];
}
