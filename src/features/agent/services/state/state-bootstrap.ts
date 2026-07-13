// 结构化 state 引导器。
//
// 迁移要点：
// 1. import 路径调整：
//    - `../models/runtime-state.js`（schema + 类型）→ `@/shared/types/runtime-state`
//    - `./memory-db.js`（类型 StoredHook / Fact）→ StoredHook 改 `@/shared/types/hook`；
//      Fact 未迁移到 @/shared/types/hook（原实现的 Fact 含可选 id 主键，服务于 node:sqlite
//      MemoryDB，Mnemosyne 用 Rust rusqlite 替代），故本文件局部定义 Fact 接口（见下）。
//    - `../utils/story-markdown.js`（normalizeHookId / parseMarkdownTableRows / parsePendingHooksMarkdown）
//      → `../utils/story-markdown`（已迁移）
//    - `../utils/hook-lifecycle.js`（normalizeHookPayoffTiming）→ 简化版已跳过该函数，
//      且 state-bootstrap 原版 import 但未实际调用（未使用 import），直接移除。
//    - `../utils/path-utils`（joinPath）替代 `node:path/join`
//    - `@/services/ipc`（ipc / ipcVoid）+ `@/shared/types/app`（FileEntry）替代 `node:fs/promises`
//
// 2. I/O 改造（node:fs/promises → Tauri IPC）：
//    - readFile → ipc<string>("fs_read_file", ...)（按场景加 .catch(() => "")）
//    - readdir → ipc<FileEntry[]>("fs_list_directory", ...)，取 entries.map(e => e.name)
//    - stat → ipc<string>("fs_read_file", ...) 试读，成功即存在
//    - writeFile → ipcVoid("fs_write_file", ...)（按 AGENTS.md 用 ipcVoid 而非 ipc<void>）
//    - mkdir → ipcVoid("fs_create_directory", ...)
//    - join → joinPath
//
// 3. 缺失函数内联：原 story-markdown.ts 导出 parseChapterSummariesMarkdown /
//    parseCurrentStateFacts / isStateTableHeaderRow / isCurrentChapterLabel /
//    inferFactSubject / parseInteger 6 个函数，但 Mnemosyne 已迁移的 story-markdown.ts
//    未含这些函数。因"不修改已存在文件"约束，不能向 story-markdown.ts 补加，故将这 6 个函数
//    verbatim 内联到本文件（parseChapterSummariesMarkdown / parseCurrentStateFacts 为 export，
//    其余为 private helper）。re-export 调整：normalizeHookId / parsePendingHooksMarkdown
//    仍从 story-markdown re-export；parseChapterSummariesMarkdown / parseCurrentStateFacts 改本文件 export。
//
// 4. loadJsonIfValid / loadHooksStateIfValid 适配：原版用 try/catch + `/ENOENT/` 正则区分
//    "文件不存在"（静默）与"文件损坏"（告警）。I/O 改为 IPC 后错误消息不再含 ENOENT，
//    故改用 `.catch(() => "")` + `if (!raw) return null` 显式处理"不存在"分支，
//    行为等价（不存在→静默返回 null，损坏→告警）。
//
// 5. strict 模式适配（Mnemosyne tsconfig 启用 noUnusedLocals）：
//    - bootstrapStructuredStateFromMarkdown 中 `const summariesState/hooksState/currentState`
//      原版赋值后从未读取，仅用于触发 loadOrBootstrap* 的副作用（写文件 + push createdFiles）。
//      改为直接 `await loadOrBootstrap*({...})` 调用，副作用完全等价，不保留无意义赋值。
//    - parseStrictIntegerWithWarning / parseStrictIntegerCell 在源文件中即为死代码链
//      （parseStrictIntegerWithWarning 从未被调用，parseStrictIntegerCell 仅被前者调用），
//      迁移时一并删除以通过 noUnusedLocals；连带从 import 移除仅在此死代码中使用的
//      normalizeHookId（re-export 语句独立保留）。均不涉及函数逻辑/正则/常量改动。
//
// 架构约束：P2 阶段 7 三层记忆接入时评估是否迁移到 Rust 端 DataDir/file_storage，
//          当前为前端 Tauri IPC 版本。
//
// 业务逻辑零改动（解析正则、schema.parse 调用、合并/去重/修复逻辑全部保留）。

import { ipc, ipcVoid } from "@/services/ipc";
import type { FileEntry } from "@/types/app";
import {
  ChapterSummariesStateSchema,
  CurrentStateStateSchema,
  HooksStateSchema,
  StateManifestSchema,
  type ChapterSummariesState,
  type CurrentStateState,
  type HooksState,
  type HookStatus,
  type StateManifest,
} from "@/types/runtime-state";
import type { StoredHook, StoredSummary } from "@/types/hook";
import {
  parseMarkdownTableRows,
  parsePendingHooksMarkdown,
} from "../utils/story-markdown";
import { joinPath } from "../utils/path-utils";

// re-export：normalizeHookId / parsePendingHooksMarkdown 仍来自 story-markdown。
// parseChapterSummariesMarkdown / parseCurrentStateFacts 因 story-markdown 未迁移，
// 改为本文件 export（见下方定义）。
export { normalizeHookId, parsePendingHooksMarkdown } from "../utils/story-markdown";

/**
 * 局部 Fact 类型 —— 原实现的 Fact 未迁移到 @/shared/types/hook
 * （原实现的 Fact 含可选 id 主键，服务于 node:sqlite MemoryDB；Mnemosyne 用 Rust
 * rusqlite 替代，运行时 DB 类型在 Rust 端定义）。此处保留与原实现一致的形状，
 * 供 parseCurrentStateFacts 返回类型使用。
 * TODO: 后续若 @/shared/types/hook 补 Fact 类型，可移除本局部定义并改回 import。
 */
interface Fact {
  readonly id?: number;
  readonly subject: string;
  readonly predicate: string;
  readonly object: string;
  readonly validFromChapter: number;
  readonly validUntilChapter: number | null;
  readonly sourceChapter: number;
}

export interface BootstrapStructuredStateResult {
  readonly createdFiles: ReadonlyArray<string>;
  readonly warnings: ReadonlyArray<string>;
  readonly manifest: StateManifest;
}

interface MarkdownBootstrapState {
  readonly summariesState: ChapterSummariesState;
  readonly hooksState: { readonly hooks: ReadonlyArray<StoredHook> };
  readonly currentState: CurrentStateState;
  readonly durableStoryProgress: number;
}

// ── 内联自原 story-markdown.ts（Mnemosyne story-markdown.ts 迁移时未含）──────

export function parseChapterSummariesMarkdown(markdown: string): StoredSummary[] {
  const rows = parseMarkdownTableRows(markdown)
    .filter((row) => /^\d+$/.test(row[0] ?? ""));

  return rows.map((row) => ({
    chapter: parseInt(row[0]!, 10),
    title: row[1] ?? "",
    characters: row[2] ?? "",
    events: row[3] ?? "",
    stateChanges: row[4] ?? "",
    hookActivity: row[5] ?? "",
    mood: row[6] ?? "",
    chapterType: row[7] ?? "",
  }));
}

export function parseCurrentStateFacts(
  markdown: string,
  fallbackChapter: number,
): Fact[] {
  const tableRows = parseMarkdownTableRows(markdown);
  const fieldValueRows = tableRows
    .filter((row) => row.length >= 2)
    .filter((row) => !isStateTableHeaderRow(row));

  if (fieldValueRows.length > 0) {
    const chapterFromTable = fieldValueRows.find((row) => isCurrentChapterLabel(row[0] ?? ""));
    const stateChapter = parseInteger(chapterFromTable?.[1]) || fallbackChapter;

    return fieldValueRows
      .filter((row) => !isCurrentChapterLabel(row[0] ?? ""))
      .flatMap((row): Fact[] => {
        const label = (row[0] ?? "").trim();
        const value = (row[1] ?? "").trim();
        if (!label || !value) return [];

        return [{
          subject: inferFactSubject(label),
          predicate: label,
          object: value,
          validFromChapter: stateChapter,
          validUntilChapter: null,
          sourceChapter: stateChapter,
        }];
      });
  }

  const bulletFacts = markdown
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("-"))
    .map((line) => line.replace(/^-\s*/, ""))
    .filter(Boolean);

  return bulletFacts.map((line, index) => ({
    subject: "current_state",
    predicate: `note_${index + 1}`,
    object: line,
    validFromChapter: fallbackChapter,
    validUntilChapter: null,
    sourceChapter: fallbackChapter,
  }));
}

function isStateTableHeaderRow(row: ReadonlyArray<string>): boolean {
  const first = (row[0] ?? "").trim().toLowerCase();
  const second = (row[1] ?? "").trim().toLowerCase();
  return (first === "字段" && second === "值") || (first === "field" && second === "value");
}

function isCurrentChapterLabel(label: string): boolean {
  return /^(当前章节|current chapter)$/i.test(label.trim());
}

function inferFactSubject(label: string): string {
  if (/^(当前位置|current location)$/i.test(label)) return "protagonist";
  if (/^(主角状态|protagonist state)$/i.test(label)) return "protagonist";
  if (/^(当前目标|current goal)$/i.test(label)) return "protagonist";
  if (/^(当前限制|current constraint)$/i.test(label)) return "protagonist";
  if (/^(当前敌我|current alliances|current relationships)$/i.test(label)) return "protagonist";
  if (/^(当前冲突|current conflict)$/i.test(label)) return "protagonist";
  return "current_state";
}

function parseInteger(value: string | undefined): number {
  if (!value) return 0;
  const match = value.match(/\d+/);
  return match ? parseInt(match[0], 10) : 0;
}

// ── 入口：从 markdown 引导 / 重写结构化 state ──────────────────────────────

export async function bootstrapStructuredStateFromMarkdown(params: {
  readonly bookDir: string;
  readonly fallbackChapter?: number;
}): Promise<BootstrapStructuredStateResult> {
  const storyDir = joinPath(params.bookDir, "story");
  const stateDir = joinPath(storyDir, "state");
  const manifestPath = joinPath(stateDir, "manifest.json");
  const currentStatePath = joinPath(stateDir, "current_state.json");
  const hooksPath = joinPath(stateDir, "hooks.json");
  const summariesPath = joinPath(stateDir, "chapter_summaries.json");

  await ipcVoid("fs_create_directory", { path: stateDir });

  const createdFiles: string[] = [];
  const warnings: string[] = [];
  const existingManifest = await loadJsonIfValid(manifestPath, StateManifestSchema, warnings, "manifest.json");
  const language = existingManifest?.language ?? await resolveRuntimeLanguage(params.bookDir);
  const markdownState = await loadMarkdownBootstrapState({
    bookDir: params.bookDir,
    storyDir,
    fallbackChapter: params.fallbackChapter ?? 0,
    warnings,
  });

  await loadOrBootstrapSummaries({
    storyDir,
    statePath: summariesPath,
    createdFiles,
    warnings,
    bootstrapState: markdownState.summariesState,
  });
  await loadOrBootstrapHooks({
    storyDir,
    statePath: hooksPath,
    createdFiles,
    warnings,
    bootstrapState: markdownState.hooksState,
  });
  await loadOrBootstrapCurrentState({
    storyDir,
    statePath: currentStatePath,
    fallbackChapter: markdownState.durableStoryProgress,
    createdFiles,
    warnings,
    bootstrapState: markdownState.currentState,
  });
  // Only trust durable artifact progress (chapter files + index).
  // currentState.chapter comes from markdown which can contain
  // hallucinated numbers (e.g. year 1988 parsed as chapter 1988).
  const derivedProgress = markdownState.durableStoryProgress;
  if ((existingManifest?.lastAppliedChapter ?? 0) > derivedProgress) {
    appendWarning(
      warnings,
      `manifest lastAppliedChapter normalized from ${existingManifest?.lastAppliedChapter ?? 0} to ${derivedProgress}`,
    );
  }

  const manifest = StateManifestSchema.parse({
    schemaVersion: 2,
    language,
    lastAppliedChapter: derivedProgress,
    projectionVersion: existingManifest?.projectionVersion ?? 1,
    migrationWarnings: uniqueStrings([
      ...(existingManifest?.migrationWarnings ?? []),
      ...warnings,
    ]),
  });

  await ipcVoid("fs_write_file", { path: manifestPath, content: JSON.stringify(manifest, null, 2) });
  if (!existingManifest) {
    createdFiles.push("manifest.json");
  }

  return {
    createdFiles,
    warnings: manifest.migrationWarnings,
    manifest,
  };
}

export async function rewriteStructuredStateFromMarkdown(params: {
  readonly bookDir: string;
  readonly fallbackChapter?: number;
}): Promise<BootstrapStructuredStateResult> {
  const storyDir = joinPath(params.bookDir, "story");
  const stateDir = joinPath(storyDir, "state");
  const manifestPath = joinPath(stateDir, "manifest.json");
  const currentStatePath = joinPath(stateDir, "current_state.json");
  const hooksPath = joinPath(stateDir, "hooks.json");
  const summariesPath = joinPath(stateDir, "chapter_summaries.json");

  await ipcVoid("fs_create_directory", { path: stateDir });

  const warnings: string[] = [];
  const existingManifest = await loadJsonIfValid(manifestPath, StateManifestSchema, warnings, "manifest.json");
  const language = existingManifest?.language ?? await resolveRuntimeLanguage(params.bookDir);
  const markdownState = await loadMarkdownBootstrapState({
    bookDir: params.bookDir,
    storyDir,
    fallbackChapter: params.fallbackChapter ?? 0,
    warnings,
  });
  const summariesState = markdownState.summariesState;
  const hooksState = markdownState.hooksState;
  const currentState = markdownState.currentState;

  const manifest = StateManifestSchema.parse({
    schemaVersion: 2,
    language,
    lastAppliedChapter: markdownState.durableStoryProgress,
    projectionVersion: existingManifest?.projectionVersion ?? 1,
    migrationWarnings: uniqueStrings([
      ...(existingManifest?.migrationWarnings ?? []),
      ...warnings,
    ]),
  });

  await Promise.all([
    ipcVoid("fs_write_file", { path: manifestPath, content: JSON.stringify(manifest, null, 2) }),
    ipcVoid("fs_write_file", { path: currentStatePath, content: JSON.stringify(currentState, null, 2) }),
    ipcVoid("fs_write_file", { path: hooksPath, content: JSON.stringify(hooksState, null, 2) }),
    ipcVoid("fs_write_file", { path: summariesPath, content: JSON.stringify(summariesState, null, 2) }),
  ]);

  return {
    createdFiles: [],
    warnings: manifest.migrationWarnings,
    manifest,
  };
}

async function loadOrBootstrapCurrentState(params: {
  readonly storyDir: string;
  readonly statePath: string;
  readonly fallbackChapter: number;
  readonly createdFiles: string[];
  readonly warnings: string[];
  readonly bootstrapState?: CurrentStateState;
  readonly forceBootstrapFromMarkdown?: boolean;
}): Promise<CurrentStateState> {
  if (!params.forceBootstrapFromMarkdown) {
    const existing = await loadJsonIfValid(
      params.statePath,
      CurrentStateStateSchema,
      params.warnings,
      "current_state.json",
    );
    if (existing) {
      return existing;
    }
  }

  const currentState = params.bootstrapState ?? await loadMarkdownCurrentState({
    storyDir: params.storyDir,
    fallbackChapter: params.fallbackChapter,
    warnings: params.warnings,
  });
  const existed = await pathExists(params.statePath);
  await ipcVoid("fs_write_file", { path: params.statePath, content: JSON.stringify(currentState, null, 2) });
  if (!existed) {
    params.createdFiles.push("current_state.json");
  }
  return currentState;
}

async function loadOrBootstrapHooks(params: {
  readonly storyDir: string;
  readonly statePath: string;
  readonly createdFiles: string[];
  readonly warnings: string[];
  readonly bootstrapState?: { readonly hooks: ReadonlyArray<StoredHook> };
  readonly forceBootstrapFromMarkdown?: boolean;
}) {
  if (!params.forceBootstrapFromMarkdown) {
    const existing = await loadHooksStateIfValid(
      params.statePath,
      params.warnings,
      "hooks.json",
    );
    if (existing) {
      if (existing.repaired) {
        await ipcVoid("fs_write_file", { path: params.statePath, content: JSON.stringify(existing.state, null, 2) });
      }
      return existing.state;
    }
  }

  const hooksState = params.bootstrapState ?? await loadMarkdownHooksState({
    storyDir: params.storyDir,
    warnings: params.warnings,
  });
  const existed = await pathExists(params.statePath);
  await ipcVoid("fs_write_file", { path: params.statePath, content: JSON.stringify(hooksState, null, 2) });
  if (!existed) {
    params.createdFiles.push("hooks.json");
  }
  return hooksState;
}

async function loadOrBootstrapSummaries(params: {
  readonly storyDir: string;
  readonly statePath: string;
  readonly createdFiles: string[];
  readonly warnings: string[];
  readonly bootstrapState?: ChapterSummariesState;
  readonly forceBootstrapFromMarkdown?: boolean;
}): Promise<ChapterSummariesState> {
  if (!params.forceBootstrapFromMarkdown) {
    const existing = await loadJsonIfValid(
      params.statePath,
      ChapterSummariesStateSchema,
      params.warnings,
      "chapter_summaries.json",
    );
    if (existing) {
      // Always deduplicate even when loading from JSON (stale data may have duplicates)
      const dedupedExisting = deduplicateSummaryRows(existing.rows);
      if (dedupedExisting.length < existing.rows.length) {
        const repaired = ChapterSummariesStateSchema.parse({ rows: dedupedExisting });
        await ipcVoid("fs_write_file", { path: params.statePath, content: JSON.stringify(repaired, null, 2) });
        return repaired;
      }
      return existing;
    }
  }

  const summariesState = params.bootstrapState ?? await loadMarkdownSummariesState(params.storyDir);
  const existed = await pathExists(params.statePath);
  await ipcVoid("fs_write_file", { path: params.statePath, content: JSON.stringify(summariesState, null, 2) });
  if (!existed) {
    params.createdFiles.push("chapter_summaries.json");
  }
  return summariesState;
}

// ── Markdown → 结构化 state 解析（含字段标准化与告警）──────────────────────

function parsePendingHooksStateMarkdown(markdown: string, warnings: string[]) {
  const parsedHooks = parsePendingHooksMarkdown(markdown);
  if (parsedHooks.length > 0) {
    return HooksStateSchema.parse({
      hooks: parsedHooks.map((hook) => ({
        ...hook,
        type: normalizeHookType(hook.type, warnings, hook.hookId),
        status: normalizeHookStatus(hook.status, warnings, hook.hookId),
      })),
    });
  }

  return HooksStateSchema.parse({
    hooks: markdown
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.startsWith("-"))
      .map((line) => line.replace(/^-\s*/, ""))
      .filter(Boolean)
      .map((line, index) => ({
        hookId: `hook-${index + 1}`,
        startChapter: 0,
        type: "unspecified",
        status: "open" as HookStatus,
        lastAdvancedChapter: 0,
        expectedPayoff: "",
        payoffTiming: undefined,
        notes: line,
      })),
  });
}

function parseCurrentStateStateMarkdown(
  markdown: string,
  fallbackChapter: number,
  warnings: string[],
): CurrentStateState {
  const tableRows = parseMarkdownTableRows(markdown);
  const fieldValueRows = tableRows
    .filter((row) => row.length >= 2)
    .filter((row) => !isStateTableHeaderRow(row));

  if (fieldValueRows.length > 0) {
    const chapterFromTable = fieldValueRows.find((row) => isCurrentChapterLabel(row[0] ?? ""));
    const stateChapter = parseIntegerWithFallback(
      chapterFromTable?.[1],
      fallbackChapter,
      warnings,
      "current_state:chapter",
    );

    return CurrentStateStateSchema.parse({
      chapter: stateChapter,
      facts: fieldValueRows
        .filter((row) => !isCurrentChapterLabel(row[0] ?? ""))
        .flatMap((row): Fact[] => {
          const label = (row[0] ?? "").trim();
          const value = (row[1] ?? "").trim();
          if (!label || !value) return [];

          return [{
            subject: inferFactSubject(label),
            predicate: label,
            object: value,
            validFromChapter: stateChapter,
            validUntilChapter: null,
            sourceChapter: stateChapter,
          }];
        }),
    });
  }

  const bulletFacts = markdown
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("-"))
    .map((line) => line.replace(/^-\s*/, ""))
    .filter(Boolean);

  return CurrentStateStateSchema.parse({
    chapter: Math.max(0, fallbackChapter),
    facts: bulletFacts.map((line, index) => ({
      subject: "current_state",
      predicate: `note_${index + 1}`,
      object: line,
      validFromChapter: Math.max(0, fallbackChapter),
      validUntilChapter: null,
      sourceChapter: Math.max(0, fallbackChapter),
    })),
  });
}

// ── 运行时语言 / 持久化故事进度解析 ────────────────────────────────────────

async function resolveRuntimeLanguage(bookDir: string): Promise<"zh" | "en"> {
  // I/O 改造：readFile → ipc<string>("fs_read_file", ...)。
  // fs_read_file 失败（文件不存在等）→ .catch(() => "") → JSON.parse 抛错 → catch → "en"。
  try {
    const raw = await ipc<string>("fs_read_file", { path: joinPath(bookDir, "book.json") });
    const parsed = JSON.parse(raw) as { language?: unknown };
    return parsed.language === "zh" ? "zh" : "en";
  } catch {
    return "en";
  }
}

export async function resolveDurableStoryProgress(params: {
  readonly bookDir: string;
  readonly fallbackChapter?: number;
}): Promise<number> {
  const explicitFallback = normalizeExplicitChapter(params.fallbackChapter);
  const durableArtifactProgress = await resolveContiguousArtifactChapterProgress(params.bookDir);
  return Math.max(durableArtifactProgress, explicitFallback);
}

// ── JSON / Hooks 状态加载（含 IPC 适配 + 输入修复）─────────────────────────

async function loadJsonIfValid<T>(
  path: string,
  schema: { parse(value: unknown): T },
  warnings: string[],
  fileLabel: string,
): Promise<T | null> {
  // I/O 改造：readFile → ipc<string>("fs_read_file", ...)。
  // fs_read_file 在文件不存在时由 Rust 端返回 not_found 错误 → .catch(() => "") 降级为空串。
  // 空串 → 视为"不存在"，静默返回 null；非空但 JSON 损坏 → schema/parse 抛错 → 告警 + null。
  // 等价于原版 try/catch + /ENOENT/ 区分"不存在 vs 损坏"的语义。
  const raw = await ipc<string>("fs_read_file", { path }).catch(() => "");
  if (!raw) return null;
  try {
    return schema.parse(JSON.parse(raw));
  } catch {
    appendWarning(warnings, `${fileLabel} invalid, rebuilt from markdown`);
    return null;
  }
}

async function loadHooksStateIfValid(
  path: string,
  warnings: string[],
  fileLabel: string,
): Promise<{ readonly state: HooksState; readonly repaired: boolean } | null> {
  // 同 loadJsonIfValid 的 IPC 适配策略
  const raw = await ipc<string>("fs_read_file", { path }).catch(() => "");
  if (!raw) return null;
  try {
    const repaired = repairHooksStateInput(JSON.parse(raw), warnings);
    return {
      state: HooksStateSchema.parse(repaired.value),
      repaired: repaired.changed,
    };
  } catch {
    appendWarning(warnings, `${fileLabel} invalid, rebuilt from markdown`);
    return null;
  }
}

function repairHooksStateInput(value: unknown, warnings: string[]): { readonly value: unknown; readonly changed: boolean } {
  if (!isRecord(value) || !Array.isArray(value.hooks)) {
    return { value, changed: false };
  }

  let changed = false;
  const hooks = value.hooks.map((hook, index) => {
    if (!isRecord(hook)) return hook;
    const hookId = typeof hook.hookId === "string" && hook.hookId.trim()
      ? hook.hookId.trim()
      : `hooks[${index}]`;
    if (typeof hook.type === "string" && hook.type.trim().length > 0) {
      if (hook.type === hook.type.trim()) {
        return hook;
      }
      changed = true;
      return { ...hook, type: hook.type.trim() };
    }

    changed = true;
    appendWarning(warnings, `${hookId}: empty hook type normalized to "unspecified"`);
    return {
      ...hook,
      type: "unspecified",
    };
  });

  return {
    value: changed ? { ...value, hooks } : value,
    changed,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// ── Markdown Bootstrap 状态聚合加载 ────────────────────────────────────────

async function loadMarkdownBootstrapState(params: {
  readonly bookDir: string;
  readonly storyDir: string;
  readonly fallbackChapter: number;
  readonly warnings: string[];
}): Promise<MarkdownBootstrapState> {
  const summariesState = await loadMarkdownSummariesState(params.storyDir);
  const hooksState = await loadMarkdownHooksState({
    storyDir: params.storyDir,
    warnings: params.warnings,
  });
  const explicitFallback = normalizeExplicitChapter(params.fallbackChapter);
  const durableArtifactProgress = await resolveContiguousArtifactChapterProgress(params.bookDir);
  const authoritativeProgress = Math.max(explicitFallback, durableArtifactProgress);
  const currentState = await loadMarkdownCurrentState({
    storyDir: params.storyDir,
    fallbackChapter: authoritativeProgress,
    warnings: params.warnings,
  });

  return {
    summariesState,
    hooksState,
    currentState,
    durableStoryProgress: authoritativeProgress,
  };
}

async function loadMarkdownSummariesState(storyDir: string): Promise<ChapterSummariesState> {
  // I/O 改造：readFile(...).catch(() => "") → ipc<string>("fs_read_file", ...).catch(() => "")
  const markdown = await ipc<string>("fs_read_file", { path: joinPath(storyDir, "chapter_summaries.md") }).catch(() => "");
  const rawRows = parseChapterSummariesMarkdown(markdown);
  return ChapterSummariesStateSchema.parse({
    rows: deduplicateSummaryRows(rawRows),
  });
}

async function loadMarkdownHooksState(params: {
  readonly storyDir: string;
  readonly warnings: string[];
}) {
  const markdown = await ipc<string>("fs_read_file", { path: joinPath(params.storyDir, "pending_hooks.md") }).catch(() => "");
  return parsePendingHooksStateMarkdown(markdown, params.warnings);
}

async function loadMarkdownCurrentState(params: {
  readonly storyDir: string;
  readonly fallbackChapter: number;
  readonly warnings: string[];
}): Promise<CurrentStateState> {
  const markdown = await ipc<string>("fs_read_file", { path: joinPath(params.storyDir, "current_state.md") }).catch(() => "");
  return parseCurrentStateStateMarkdown(markdown, params.fallbackChapter, params.warnings);
}

// ── 持久化章节文件进度解析（durable artifact truth）─────────────────────────

async function resolveContiguousArtifactChapterProgress(bookDir: string): Promise<number> {
  const chapterNumbers = await loadDurableArtifactChapterNumbers(bookDir);
  return resolveContiguousChapterPrefix(chapterNumbers);
}

async function loadDurableArtifactChapterNumbers(bookDir: string): Promise<number[]> {
  const chaptersDir = joinPath(bookDir, "chapters");
  const indexPath = joinPath(chaptersDir, "index.json");
  // I/O 改造：readFile → ipc<string>("fs_read_file", ...)；
  //          readdir → ipc<FileEntry[]>("fs_list_directory", ...) 取 entries.map(e => e.name)。
  const [indexChapters, fileChapters] = await Promise.all([
    ipc<string>("fs_read_file", { path: indexPath })
      .then((raw) => {
        const parsed = JSON.parse(raw) as Array<{ number?: unknown }>;
        return parsed
          .map((entry) => entry?.number)
          .filter((entry): entry is number => typeof entry === "number" && Number.isInteger(entry) && entry > 0);
      })
      .catch(() => [] as number[]),
    ipc<FileEntry[]>("fs_list_directory", { path: chaptersDir })
      .then((entries) => entries.flatMap((entry) => {
        const match = entry.name.match(/^(\d+)_/);
        return match ? [parseInt(match[1]!, 10)] : [];
      }))
      .catch(() => [] as number[]),
  ]);
  return [...indexChapters, ...fileChapters];
}

async function pathExists(path: string): Promise<boolean> {
  // I/O 改造：stat → ipc<string>("fs_read_file", ...) 试读。
  // fs_read_file 对目录返回 invalid_input，对不存在文件返回 not_found，
  // 两者均被 .catch 降级为 false；仅当文件可读时返回 true。
  // 本文件中 pathExists 仅用于 .json 状态文件路径检查，不涉及目录，语义等价。
  return ipc<string>("fs_read_file", { path }).then(() => true).catch(() => false);
}

// ── 纯函数：去重 / 章节前缀 / 字段标准化 / 整数解析 / 警告去重 ──────────────

function deduplicateSummaryRows<T extends { chapter: number }>(rows: ReadonlyArray<T>): T[] {
  const byChapter = new Map<number, T>();
  for (const row of rows) {
    byChapter.set(row.chapter, row);
  }
  return [...byChapter.values()].sort((a, b) => a.chapter - b.chapter);
}

export function resolveContiguousChapterPrefix(chapterNumbers: ReadonlyArray<number>): number {
  const chapters = new Set(
    chapterNumbers.filter((chapter): chapter is number => Number.isInteger(chapter) && chapter > 0),
  );
  let contiguousChapter = 0;
  while (chapters.has(contiguousChapter + 1)) {
    contiguousChapter += 1;
  }
  return contiguousChapter;
}

function normalizeHookStatus(value: string | undefined, warnings: string[], hookId: string): HookStatus {
  const normalized = (value ?? "").trim().toLowerCase();
  if (!normalized) return "open";
  if (/(resolved|closed|done|paid[_ -]?off|已回收|回收|完成|已解决|已兑现|兑现)/i.test(normalized)) return "resolved";
  if (/(deferred|paused|hold|dormant|inactive|unplanted|unseeded|not[_ -]?started|not[_ -]?active|搁置|延后|延期|暂缓|休眠|未激活|未启动|待启动|未推进|尚未推进)/i.test(normalized)) return "deferred";
  if (/(confirmed[_ -]?hit|confirmed|advanced|progressing|progress|active|pressured|命中|已确认命中|已推进|推进|进行中|持续推进|重大推进)/i.test(normalized)) return "progressing";
  if (/(open|pending|seeded|planted|待定|未回收|已埋|已种下|已铺垫)/i.test(normalized)) return "open";
  appendWarning(warnings, `${hookId}:status normalized from "${value ?? ""}" to "open"`);
  return "open";
}

function normalizeHookType(value: string | undefined, warnings: string[], hookId: string): string {
  const normalized = (value ?? "").trim();
  if (normalized) return normalized;
  appendWarning(warnings, `${hookId}: empty hook type normalized to "unspecified"`);
  return "unspecified";
}

function parseIntegerWithFallback(
  value: string | undefined,
  fallback: number,
  warnings: string[],
  fieldLabel: string,
): number {
  if (!value) return Math.max(0, fallback);
  const match = value.match(/\d+/);
  if (!match) {
    appendWarning(warnings, `${fieldLabel} normalized from "${value}" to ${Math.max(0, fallback)}`);
    return Math.max(0, fallback);
  }
  return parseInt(match[0], 10);
}

function normalizeExplicitChapter(value: number | undefined): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) {
    return 0;
  }
  return value;
}

function appendWarning(warnings: string[], warning: string): void {
  if (!warnings.includes(warning)) {
    warnings.push(warning);
  }
}

function uniqueStrings(values: ReadonlyArray<string>): string[] {
  return [...new Set(values.filter((value) => value.trim().length > 0))];
}
