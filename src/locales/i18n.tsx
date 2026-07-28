/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 国际化管理 - 多语言支持与翻译管理
 * ═══════════════════════════════════════════════════════════════════════════
 */

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
import { setTranslations } from "@/locales/i18n-store";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type ShapeOf<T> = T extends string
  ? string
  : T extends number
    ? number
    : T extends (infer U)[]
      ? ShapeOf<U>[]
      : T extends object
        ? { [K in keyof T]: ShapeOf<T[K]> }
        : T;

export type Translations = ShapeOf<TranslationKeys>;

const STORAGE_KEY_LOCALE = "mnemosyne-locale";

interface I18nContextValue {
  locale: Locale;
  t: Translations;
  setLocale: (locale: Locale) => void;
}

// ── 工具函数 ────────────────────────────────────────────────────────────────

function getInitialLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_LOCALE);
    if (stored === "en" || stored === "zh") return stored;
  } catch {
  }
  return "en";
}

// ── Context 定义 ────────────────────────────────────────────────────────────────

const I18nContext = createContext<I18nContextValue | null>(null);

// ── Provider 组件 ────────────────────────────────────────────────────────────────

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getInitialLocale);
  // 初始用内置 en 翻译，zh 在 useEffect 中异步加载
  const [t, setT] = useState<Translations>(builtinTranslations as Translations);

  // ── 语言切换时异步加载翻译文件 ────────────────────────────────────────────────────────────────
  // zh 按需 import，减小首屏 bundle
  useEffect(() => {
    let cancelled = false;
    void loadLocale(locale).then((translations) => {
      if (!cancelled) {
        setT(translations as Translations);
        // 同步到模块级 i18n-store，让非 React 上下文（如 zustand store）也能读到最新翻译
        setTranslations(locale, translations as Translations);
      }
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

// ── Hooks ────────────────────────────────────────────────────────────────

export function useI18n() {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within I18nProvider");
  return ctx;
}

export { getLocaleLabel };