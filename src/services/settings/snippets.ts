/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 代码片段管理服务
 * ═══════════════════════════════════════════════════════════════════════════
 */

import type { Snippet } from "./types";
import { loadSettings, saveSettings } from "./settings-core";

// ── Handle 验证与规范化 ────────────────────────────────────────────────────────

const HANDLE_RE = /^[a-z0-9][a-z0-9-]*$/;

export function normalizeHandle(raw: string): string {
  return raw
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function isValidHandle(h: string): boolean {
  return HANDLE_RE.test(h);
}

export function newSnippetId(): string {
  return `sn-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

// ── Snippet 持久化 ────────────────────────────────────────────────────────────────

export async function loadSnippets(): Promise<Snippet[]> {
  const settings = await loadSettings();
  return settings.ai.snippets;
}

export async function saveSnippets(list: Snippet[]): Promise<void> {
  const settings = await loadSettings();
  settings.ai.snippets = list;
  await saveSettings({ ai: settings.ai }, settings);
}