/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 日志查看器入口 - 独立窗口入口点
 * ═══════════════════════════════════════════════════════════════════════════
 */

import React from "react";
import ReactDOM from "react-dom/client";
import { LogViewer } from "./LogViewer";
import "../styles/index.css";

// ── 常量定义 ────────────────────────────────────────────────────────────────

const STORAGE_KEY_THEME = "mnemosyne-theme";

// ── 主题工具函数 ────────────────────────────────────────────────────────────────

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function getInitialTheme(): "light" | "dark" {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_THEME);
    if (stored === "light" || stored === "dark") return stored;
    if (stored === "system") return getSystemTheme();
  } catch {
    // 忽略错误
  }
  return getSystemTheme();
}

function applyTheme(theme: "light" | "dark") {
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(theme);
}

// ── 初始化 ────────────────────────────────────────────────────────────────

// 初始化主题
applyTheme(getInitialTheme());

// 监听系统主题变化
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => {
  try {
    const stored = localStorage.getItem(STORAGE_KEY_THEME);
    if (stored === "system" || !stored) {
      applyTheme(e.matches ? "dark" : "light");
    }
  } catch {
    // 忽略错误
  }
});

// ── 渲染应用 ────────────────────────────────────────────────────────────────

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <LogViewer />
    </React.StrictMode>
  );
}