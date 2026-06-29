import { createContext, useContext, useState, useCallback, type ReactNode } from "react";
import { locales, type Locale } from "@/shared/locales";

// t 的类型从 locales 推导（已放宽字面量类型为 string）
type TranslationKeys = (typeof locales)["en"];

const STORAGE_KEY_LOCALE = "mnemosyne-locale";

interface I18nContextValue {
  locale: Locale;
  t: TranslationKeys;
  setLocale: (locale: Locale) => void;
}

function getInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_LOCALE);
    if (stored === "en" || stored === "zh") return stored;
  } catch {
    // localStorage 不可用，使用默认值
  }
  return "en";
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);

  const setLocale = useCallback((newLocale: Locale) => {
    setLocaleState(newLocale);
    try {
      localStorage.setItem(STORAGE_KEY_LOCALE, newLocale);
    } catch {
      // localStorage 不可用，跳过持久化
    }
  }, []);

  const value: I18nContextValue = {
    locale,
    t: locales[locale],
    setLocale,
  };

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}
