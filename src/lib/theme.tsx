/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 主题管理 - 应用主题切换与管理
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { createContext, useContext, useState, useCallback, useEffect, useMemo, type ReactNode } from "react";
import { setWindowTheme } from "@/features/settings/services/general";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type Theme = "light" | "dark" | "system";

interface ThemeContextValue {
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
}

// ── 常量定义 ────────────────────────────────────────────────────────────────

const STORAGE_KEY_THEME = "mnemosyne-theme";

// ── 工具函数 ────────────────────────────────────────────────────────────────

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function getInitialTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_THEME);
    if (stored === "light" || stored === "dark" || stored === "system") return stored;
  } catch {
  }
  return "system";
}

function resolveTheme(theme: Theme): "light" | "dark" {
  return theme === "system" ? getSystemTheme() : theme;
}

// ── Context 定义 ────────────────────────────────────────────────────────────────

const ThemeContext = createContext<ThemeContextValue | null>(null);

// ── Provider 组件 ────────────────────────────────────────────────────────────────

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(getInitialTheme);
  const [resolvedTheme, setResolvedTheme] = useState<"light" | "dark">(() =>
    resolveTheme(theme)
  );

  const applyTheme = useCallback((t: Theme) => {
    const resolved = resolveTheme(t);
    setResolvedTheme(resolved);
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(resolved);
    void setWindowTheme(resolved);
  }, []);

  // ── 应用主题 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    applyTheme(theme);
  }, [applyTheme, theme]);

  // ── 监听系统主题变化 ────────────────────────────────────────────────────────────────
  useEffect(() => {
    if (theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("system");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme, applyTheme]);

  const setTheme = useCallback(
    (newTheme: Theme) => {
      setThemeState(newTheme);
      applyTheme(newTheme);
      try {
        localStorage.setItem(STORAGE_KEY_THEME, newTheme);
      } catch {
      }
    },
    [applyTheme]
  );

  // 使用 useMemo 稳定 context value 引用，避免 theme 不变时所有 consumer 无意义重渲染
  const value = useMemo<ThemeContextValue>(
    () => ({ theme, resolvedTheme, setTheme }),
    [theme, resolvedTheme, setTheme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

// ── Hooks ────────────────────────────────────────────────────────────────

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}