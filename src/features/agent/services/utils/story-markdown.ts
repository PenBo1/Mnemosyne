// Story markdown 渲染 —— hooks / summaries 的 markdown 表格生成。
//
// 简化点：原版通过 hook-lifecycle.ts + hook-policy.ts 推导 hook 的
// payoff timing enum 再本地化展示。Mnemosyne 的 architect 阶段直接展示
// 原始 payoffTiming 字符串（LLM 写在表里的"立即/近期/中程/慢烧/终局"等），
// 不依赖 hook-lifecycle —— 后续 P2 阶段 7 三层记忆接入时再补 timing 推导。
//
// 纯函数，无外部依赖（除 StoredHook / StoredSummary 类型）。

import type { StoredHook, StoredSummary } from "@/types/hook";

export function renderSummarySnapshot(
  summaries: ReadonlyArray<StoredSummary>,
  language: "zh" | "en" = "zh",
): string {
  if (summaries.length === 0) return "- none";

  const headers = language === "en"
    ? [
      "| chapter | title | characters | events | stateChanges | hookActivity | mood | chapterType |",
      "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    : [
      "| 章节 | 标题 | 出场人物 | 关键事件 | 状态变化 | 伏笔动态 | 情绪基调 | 章节类型 |",
      "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ];

  return [
    ...headers,
    ...summaries.map((summary) => [
      summary.chapter,
      summary.title,
      summary.characters,
      summary.events,
      summary.stateChanges,
      summary.hookActivity,
      summary.mood,
      summary.chapterType,
    ].map(escapeTableCell).join(" | ")).map((row) => `| ${row} |`),
  ].join("\n");
}

export function renderHookSnapshot(
  hooks: ReadonlyArray<StoredHook>,
  language: "zh" | "en" = "zh",
): string {
  if (hooks.length === 0) return "- none";

  const headers = language === "en"
    ? [
      "| hook_id | start_chapter | type | status | last_advanced | expected_payoff | payoff_timing | depends_on | pays_off_in_arc | core_hook | half_life | promoted | notes |",
      "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    : [
      "| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 升级 | 备注 |",
      "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ];

  return [
    ...headers,
    ...hooks.map((hook) => [
      hook.hookId,
      hook.startChapter,
      hook.type,
      hook.status,
      hook.lastAdvancedChapter,
      hook.expectedPayoff,
      // 简化：直接展示原始字符串，不做 timing enum 推导
      hook.payoffTiming ?? "",
      renderDependsOnCell(hook.dependsOn ?? [], language),
      hook.paysOffInArc ?? "",
      renderCoreHookCell(hook.coreHook === true, language),
      renderHalfLifeCell(hook.halfLifeChapters),
      renderPromotedCell(hook.promoted, language),
      hook.notes,
    ].map((cell) => escapeTableCell(String(cell))).join(" | ")).map((row) => `| ${row} |`),
  ].join("\n");
}

function renderHalfLifeCell(value: number | undefined): string {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) return "";
  return String(Math.trunc(value));
}

function renderPromotedCell(value: boolean | undefined, language: "zh" | "en"): string {
  if (value === undefined) return "";
  if (language === "en") return value ? "true" : "false";
  return value ? "是" : "否";
}

function renderDependsOnCell(ids: ReadonlyArray<string>, language: "zh" | "en"): string {
  if (ids.length === 0) return language === "en" ? "none" : "无";
  return `[${ids.join(", ")}]`;
}

function renderCoreHookCell(isCore: boolean, language: "zh" | "en"): string {
  if (language === "en") return isCore ? "true" : "false";
  return isCore ? "是" : "否";
}

/** 解析 markdown 表格的所有行（每行是一个 cell 数组）。 */
export function parseMarkdownTableRows(markdown: string): string[][] {
  return markdown
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("|"))
    .filter((line) => !line.includes("---"))
    .map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()))
    .filter((cells) => cells.some(Boolean));
}

/**
 * 解析 pending_hooks.md 为 StoredHook[]。
 *
 * 支持 5 种行形态（按列数自动识别）：
 *   7 列 legacy pre-timing / 8 列 Phase 5-6 / 11 列 Phase 7 compact /
 *   12 列 Phase 7 hotfix 1（含 half_life）/ 13 列 Phase 7 hotfix 2（含 promoted）
 *
 * payoffTiming 保留原始字符串（不做 enum 推导，与 architect 一致）。
 * 无表格行时回退到 bullet list 解析（- 开头）。
 */
export function parsePendingHooksMarkdown(markdown: string): StoredHook[] {
  const tableRows = parseMarkdownTableRows(markdown)
    .filter((row) => (row[0] ?? "").toLowerCase() !== "hook_id");

  if (tableRows.length > 0) {
    return tableRows
      .filter((row) => normalizeHookId(row[0]).length > 0)
      .map((row) => parsePendingHookRow(row));
  }

  return markdown
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("-"))
    .map((line) => line.replace(/^-\s*/, ""))
    .filter(Boolean)
    .map((line, index) => ({
      hookId: `hook-${index + 1}`,
      startChapter: 0,
      type: "unspecified",
      status: "open",
      lastAdvancedChapter: 0,
      expectedPayoff: "",
      payoffTiming: undefined,
      notes: line,
    }));
}

function parsePendingHookRow(row: ReadonlyArray<string | undefined>): StoredHook {
  const phase7Promoted = row.length >= 13;
  const phase7HalfLife = row.length === 12;
  const phase7Compact = row.length === 11;
  const phase7 = phase7Promoted || phase7HalfLife || phase7Compact;
  const legacyShape = row.length < 8;
  const payoffTiming = legacyShape ? undefined : normalizeHookPayoffTiming(row[6]);
  const notes = phase7Promoted
    ? (row[12] ?? "")
    : phase7HalfLife
      ? (row[11] ?? "")
      : phase7Compact
        ? (row[10] ?? "")
        : legacyShape
          ? (row[6] ?? "")
          : (row[7] ?? "");

  const base: StoredHook = {
    hookId: normalizeHookId(row[0]),
    startChapter: parseStrictChapterInteger(row[1]),
    type: row[2] ?? "",
    status: row[3] ?? "open",
    lastAdvancedChapter: parseStrictChapterInteger(row[4]),
    expectedPayoff: row[5] ?? "",
    payoffTiming,
    notes,
  };

  if (!phase7) return base;

  return {
    ...base,
    dependsOn: parseDependsOn(row[7] ?? ""),
    paysOffInArc: (row[8] ?? "").trim(),
    coreHook: parseBooleanCell(row[9]),
    halfLifeChapters: (phase7HalfLife || phase7Promoted) ? parseOptionalInt(row[10]) : undefined,
    promoted: phase7Promoted ? parseOptionalBooleanCell(row[11]) : undefined,
  };
}

/**
 * 规范化 payoffTiming 字符串。
 *
 * story-markdown 层直接保留原始字符串（不做 enum 推导）：
 * Mnemosyne 直接保留原始字符串，LLM 写什么就存什么
 * （"立即"/"近期"/"中程"/"慢烧"/"终局" 等中文，或 "immediate"/"near"/"mid"/"slow"/"endgame" 等英文）。
 */
function normalizeHookPayoffTiming(value: string | undefined): string | undefined {
  const trimmed = (value ?? "").trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

export function normalizeHookId(value: string | undefined): string {
  let normalized = (value ?? "").trim();
  let previous = "";
  while (normalized && normalized !== previous) {
    previous = normalized;
    normalized = normalized
      .replace(/^\[(.+?)\]\([^)]+\)$/u, "$1")
      .replace(/^\*\*(.+)\*\*$/u, "$1")
      .replace(/^__(.+)__$/u, "$1")
      .replace(/^\*(.+)\*$/u, "$1")
      .replace(/^_(.+)_$/u, "$1")
      .replace(/^`(.+)`$/u, "$1")
      .replace(/^~~(.+)~~$/u, "$1")
      .trim();
  }
  normalized = normalized
    .replace(/-{2,}/g, "-")
    .replace(/^-+|-+$/g, "")
    .trim();
  return /[a-z0-9\u4e00-\u9fff]/iu.test(normalized) ? normalized : "";
}

function parseStrictChapterInteger(value: string | undefined): number {
  if (!value) return 0;
  const stripped = normalizeHookId(value);
  return /^\d+$/.test(stripped) ? parseInt(stripped, 10) : 0;
}

function parseOptionalBooleanCell(cell: string | undefined): boolean | undefined {
  const normalized = (cell ?? "").trim();
  if (!normalized) return undefined;
  const lower = normalized.toLowerCase();
  if (/^(true|yes|y|是|核心|core|1|✓|✔|promoted|已升级)$/.test(lower)) return true;
  if (/^(false|no|n|否|未升级|seed|0|✗|✘)$/.test(lower)) return false;
  return undefined;
}

function parseDependsOn(cell: string): ReadonlyArray<string> {
  const trimmed = cell.trim();
  if (!trimmed) return [];
  const lower = trimmed.toLowerCase();
  if (lower === "none" || lower === "n/a" || lower === "-" || trimmed === "无") return [];

  // Accept [H01, H02] or H01, H02 or H01/H02.
  const stripped = trimmed.replace(/^[\[\(]\s*/, "").replace(/\s*[\]\)]$/, "");
  return stripped
    .split(/[,，、\/]+/)
    .map((item) => normalizeHookId(item))
    .filter((item) => item.length > 0);
}

function parseBooleanCell(cell: string | undefined): boolean {
  const normalized = (cell ?? "").trim().toLowerCase();
  if (!normalized) return false;
  return /^(true|yes|y|是|核心|core|1|✓|✔)$/.test(normalized);
}

function parseOptionalInt(cell: string | undefined): number | undefined {
  const normalized = (cell ?? "").trim();
  if (!normalized) return undefined;
  const match = normalized.match(/\d+/);
  if (!match) return undefined;
  const value = parseInt(match[0], 10);
  return Number.isFinite(value) && value > 0 ? value : undefined;
}

function escapeTableCell(value: string | number): string {
  return String(value).replace(/\|/g, "\\|").trim();
}
