/**
 * ═══════════════════════════════════════════════════════════════════════════
 * SettingsPage - 设置页面入口
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { useEffect } from "react";
import { useAppState, useAppDispatch } from "@/lib/app-context";
import { DEFAULT_SETTINGS_PAGE } from "@/types";

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * 设置默认入口：重定向到默认设置子页
 * 真正的设置子页都是独立路由（settings.general / settings.model 等），
 * 通过 sidebar 点击直接路由跳转，与其他主页面一致。
 */
export function SettingsPage() {
  const { currentPage } = useAppState();
  const dispatch = useAppDispatch();

  // ── 路由重定向 ────────────────────────────────────────────────────────────

  useEffect(() => {
    if (currentPage === "settings") {
      dispatch({ type: "SET_PAGE", payload: DEFAULT_SETTINGS_PAGE });
    }
  }, [currentPage, dispatch]);

  return null;
}