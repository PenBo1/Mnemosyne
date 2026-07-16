import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useMemo,
  type ReactNode,
} from "react";
import {
  builtinTranslations,
  loadLocale,
  getLocaleLabel,
  type Locale,
  type TranslationKeys,
} from "@/locales";

type ShapeOf<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends (infer U)[]
      ? ShapeOf<U>[]
      : T extends object
        ? { [K in keyof T]: ShapeOf<T[K]> }
        : T;

type Translations = ShapeOf<TranslationKeys>;

const STORAGE_KEY_LOCALE = "mnemosyne-locale";

interface I18nContextValue {
  locale: Locale;
  t: Translations;
  setLocale: (locale: Locale) => void;
}

function getInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_LOCALE);
    if (stored === "en" || stored === "zh") return stored;
  } catch {
  }
  return "en";
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);
  // 初始用内置 en 翻译，zh 在 useEffect 中异步加载
  const [t, setT] = useState<Translations>(builtinTranslations as Translations);

  // locale 变化时异步加载翻译文件（zh 按需 import，减小首屏 bundle）
  useEffect(() => {
    let cancelled = false;
    void loadLocale(locale).then((translations) => {
      if (!cancelled) setT(translations as Translations);
    });
    return () => { cancelled = true; };
  }, [locale]);

  const setLocale = useCallback((newLocale: Locale) => {
    setLocaleState(newLocale);
    try {
      localStorage.setItem(STORAGE_KEY_LOCALE, newLocale);
    } catch {
    }
  }, []);

  const value = useMemo<I18nContextValue>(
    () => ({ locale, t, setLocale }),
    [locale, t, setLocale],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

export { getLocaleLabel };
