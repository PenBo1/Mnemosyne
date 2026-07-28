// 模块级 i18n 状态镜像：让 store / 非 React 模块也能读取当前翻译。
//
// 背景：i18n.tsx 用 React Context 暴露 t，但 zustand store（如 chat-store）
// 不是 React 组件，无法调用 useI18n()。本模块通过模块级变量镜像当前翻译，
// 由 I18nProvider 在 locale 变化时调用 setTranslations() 同步。
//
// 注意：模块加载时使用内置 en 翻译作为初值，与 I18nProvider 的初始状态一致。
// 在 I18nProvider 挂载前调用 getTranslations() 也能拿到兜底翻译。

import { builtinTranslations, type Locale } from "@/locales";

type Translations = typeof builtinTranslations;

let currentTranslations: Translations = builtinTranslations;
let currentLocale: Locale = "en";

/** I18nProvider 在 locale / translations 变化时调用，同步到模块级状态。 */
export function setTranslations(locale: Locale, translations: Translations): void {
  currentLocale = locale;
  currentTranslations = translations;
}

/** 获取当前翻译对象（非 React 上下文，如 zustand store 内部使用）。 */
export function getTranslations(): Translations {
  return currentTranslations;
}

/** 获取当前 locale。 */
export function getCurrentLocale(): Locale {
  return currentLocale;
}
