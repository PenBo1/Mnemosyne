// 记忆检索。
//
// P2.2 已接入 rusqlite 加速层：retrieveMemorySelection 优先调用
// memory_retrieve_selection IPC（后端查询 story_facts / chapter_summaries 表），
// 如果 IPC 成功且返回 hasData=true，直接使用 SQLite 数据作为 facts + summaries。
// 如果 IPC 失败或返回空，显式 log warning 并降级到 markdown/JSON 文件读取。
//
// hooks 和 volume_summaries 目前无 SQLite 表（hooks 存于 state.json/pending_hooks.md，
// volume_summaries 仅 markdown），在 IPC 路径下仍从 markdown 读取。
//
// 迁移历史：
// 1. import 路径调整（node:fs/promises → Tauri IPC + joinPath）
// 2. I/O 改造（readFile → ipc("fs_read_file")）
// 3. P2.2: 接入 rusqlite memory_db 加速层（memory_retrieve_selection IPC）
// 4. strict 模式适配（noUnusedLocals）
//
// 业务逻辑零改动（函数逻辑、正则、常量值、排序规则、阈值全部保留）。

import { ipc } from "@/services/ipc";
import type { StoredHook, StoredSummary } from "@/types/hook";
import {
  ChapterSummariesStateSchema,
  CurrentStateStateSchema,
  HooksStateSchema,
} from "@/types/runtime-state";
import {
  bootstrapStructuredStateFromMarkdown,
  parseChapterSummariesMarkdown,
  parseCurrentStateFacts,
} from "../state/state-bootstrap";
import { readCurrentStateWithFallback } from "./outline-paths";
import {
  filterActiveHooks,
  isFuturePlannedHook,
  isHookWithinChapterWindow,
} from "./hook-lifecycle";
import { parsePendingHooksMarkdown } from "./story-markdown";
import { joinPath } from "./path-utils";

// 仅 re-export（不在本文件内调用）的函数用 `export { ... } from` 形式，
// 避免 noUnusedLocals 报"declared but never read"（与 state-bootstrap.ts 迁移一致）。
export {
  isFuturePlannedHook,
  isHookWithinChapterWindow,
} from "./hook-lifecycle";
export {
  parsePendingHooksMarkdown,
  renderHookSnapshot,
  renderSummarySnapshot,
} from "./story-markdown";
export {
  parseChapterSummariesMarkdown,
  parseCurrentStateFacts,
} from "../state/state-bootstrap";

/**
 * 局部 Fact 类型 —— 与后端 RetrievedFact DTO（camelCase 序列化）对齐。
 * 后端 memory_retrieve_selection IPC 直接返回此形状的数据。
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

export interface MemorySelection {
  readonly summaries: ReadonlyArray<StoredSummary>;
  readonly hooks: ReadonlyArray<StoredHook>;
  readonly activeHooks: ReadonlyArray<StoredHook>;
  /**
   * Hooks with recycling pressure — stale hooks that the planner must
   * advance/resolve/defer (and if deferred, justify). Sorted by staleness DESC
   * (most overdue first). See computeRecyclableHooks for the selection rule.
   */
  readonly recyclableHooks: ReadonlyArray<StoredHook>;
  readonly facts: ReadonlyArray<Fact>;
  readonly volumeSummaries: ReadonlyArray<VolumeSummarySelection>;
}

export interface VolumeSummarySelection {
  readonly heading: string;
  readonly content: string;
  readonly anchor: string;
}

// ── P2.2 SQLite IPC 加速层类型 ──────────────────────────────

/** 后端 memory_retrieve_selection IPC 返回结构（camelCase）。 */
interface MemoryRetrievalResult {
  readonly facts: ReadonlyArray<RetrievedFactDto>;
  readonly summaries: ReadonlyArray<RetrievedSummaryDto>;
  readonly hasData: boolean;
}

interface RetrievedFactDto {
  readonly id: number | null;
  readonly subject: string;
  readonly predicate: string;
  readonly object: string;
  readonly validFromChapter: number;
  readonly validUntilChapter: number | null;
  readonly sourceChapter: number;
}

interface RetrievedSummaryDto {
  readonly chapter: number;
  readonly title: string;
  readonly characters: string;
  readonly events: string;
  readonly stateChanges: string;
  readonly hookActivity: string;
  readonly mood: string;
  readonly chapterType: string;
}

/** 从 bookDir 路径提取 book_id（路径最后一段非空组件）。 */
function extractBookId(bookDir: string): string {
  const trimmed = bookDir.replace(/[\\/]+$/, "");
  const parts = trimmed.split(/[\\/]+/);
  return parts[parts.length - 1] ?? "";
}

/**
 * P2.2: 尝试通过 SQLite IPC 检索 facts + summaries。
 *
 * 成功且有数据时返回 MemorySelection（facts/summaries 来自 SQLite，
 * hooks/volume_summaries 仍从 markdown 读取）。
 * 失败或无数据时返回 null，调用方降级到全量 markdown 路径。
 *
 * 不 silent fallback：IPC 失败时 console.warn 显式记录。
 */
async function tryRetrieveSelectionFromSqlite(params: {
  readonly bookDir: string;
  readonly chapterNumber: number;
  readonly goal: string;
  readonly outlineNode?: string;
  readonly mustKeep?: ReadonlyArray<string>;
}): Promise<MemorySelection | null> {
  const bookId = extractBookId(params.bookDir);
  if (!bookId) {
    console.warn("[memory-retrieval] Cannot extract bookId from bookDir, falling back to markdown");
    return null;
  }

  let result: MemoryRetrievalResult;
  try {
    result = await ipc<MemoryRetrievalResult>("memory_retrieve_selection", {
      bookId,
      chapterNumber: params.chapterNumber,
      goal: params.goal,
      includeFacts: true,
      includeHooks: false,
      includeSummaries: true,
      includeVolumeSummaries: false,
      maxItemsPerCategory: 20,
    });
  } catch (err) {
    console.warn(
      "[memory-retrieval] IPC memory_retrieve_selection failed, falling back to markdown:",
      err,
    );
    return null;
  }

  if (!result.hasData) {
    console.warn("[memory-retrieval] IPC returned hasData=false, falling back to markdown");
    return null;
  }

  // IPC 有数据：仍需从 markdown 读取 hooks + volume_summaries（无 SQLite 表）
  const storyDir = joinPath(params.bookDir, "story");
  const [hooksMarkdown, volumeSummariesMarkdown] = await Promise.all([
    ipc<string>("fs_read_file", { path: joinPath(storyDir, "pending_hooks.md") }).catch(() => ""),
    ipc<string>("fs_read_file", { path: joinPath(storyDir, "volume_summaries.md") }).catch(() => ""),
  ]);

  const narrativeQueryTerms = extractQueryTerms(params.goal, params.outlineNode, []);
  const factQueryTerms = extractQueryTerms(params.goal, params.outlineNode, params.mustKeep ?? []);

  const volumeSummaries = selectRelevantVolumeSummaries(
    parseVolumeSummariesMarkdown(volumeSummariesMarkdown),
    narrativeQueryTerms,
  );

  const hooks = parsePendingHooksMarkdown(hooksMarkdown);
  const activeHooks = filterActiveHooks(hooks);

  const facts: Fact[] = result.facts.map((f) => ({
    id: f.id ?? undefined,
    subject: f.subject,
    predicate: f.predicate,
    object: f.object,
    validFromChapter: f.validFromChapter,
    validUntilChapter: f.validUntilChapter,
    sourceChapter: f.sourceChapter,
  }));

  const summaries: StoredSummary[] = result.summaries.map((s) => ({
    chapter: s.chapter,
    title: s.title,
    characters: s.characters,
    events: s.events,
    stateChanges: s.stateChanges,
    hookActivity: s.hookActivity,
    mood: s.mood,
    chapterType: s.chapterType,
  }));

  return {
    summaries: selectRelevantSummaries(summaries, params.chapterNumber, narrativeQueryTerms),
    hooks: selectRelevantHooks(activeHooks, narrativeQueryTerms, params.chapterNumber),
    activeHooks,
    recyclableHooks: computeRecyclableHooks(activeHooks, params.chapterNumber),
    facts: selectRelevantFacts(facts, factQueryTerms),
    volumeSummaries,
  };
}

export async function retrieveMemorySelection(params: {
  readonly bookDir: string;
  readonly chapterNumber: number;
  readonly goal: string;
  readonly outlineNode?: string;
  readonly mustKeep?: ReadonlyArray<string>;
}): Promise<MemorySelection> {
  // P2.2: 优先通过 SQLite IPC 检索 facts + summaries（加速层）
  const ipcSelection = await tryRetrieveSelectionFromSqlite(params);
  if (ipcSelection !== null) {
    return ipcSelection;
  }

  // 降级路径：从 markdown/JSON 文件读取（原有逻辑，保持不变）
  const storyDir = joinPath(params.bookDir, "story");
  const stateDir = joinPath(storyDir, "state");
  const fallbackChapter = Math.max(0, params.chapterNumber - 1);

  await bootstrapStructuredStateFromMarkdown({
    bookDir: params.bookDir,
    fallbackChapter,
  }).catch(() => undefined);

  const [
    currentStateMarkdown,
    hooksMarkdown,
    volumeSummariesMarkdown,
    chapterSummariesMarkdown,
    structuredCurrentState,
    structuredHooks,
    structuredSummaries,
  ] = await Promise.all([
    readCurrentStateWithFallback(params.bookDir),
    ipc<string>("fs_read_file", { path: joinPath(storyDir, "pending_hooks.md") }).catch(() => ""),
    ipc<string>("fs_read_file", { path: joinPath(storyDir, "volume_summaries.md") }).catch(() => ""),
    ipc<string>("fs_read_file", { path: joinPath(storyDir, "chapter_summaries.md") }).catch(() => ""),
    readStructuredState(joinPath(stateDir, "current_state.json"), CurrentStateStateSchema),
    readStructuredState(joinPath(stateDir, "hooks.json"), HooksStateSchema),
    readStructuredState(joinPath(stateDir, "chapter_summaries.json"), ChapterSummariesStateSchema),
  ]);
  const facts = structuredCurrentState?.facts ?? parseCurrentStateFacts(
    currentStateMarkdown,
    fallbackChapter,
  );
  const narrativeQueryTerms = extractQueryTerms(
    params.goal,
    params.outlineNode,
    [],
  );
  const factQueryTerms = extractQueryTerms(
    params.goal,
    params.outlineNode,
    params.mustKeep ?? [],
  );
  const volumeSummaries = selectRelevantVolumeSummaries(
    parseVolumeSummariesMarkdown(volumeSummariesMarkdown),
    narrativeQueryTerms,
  );
  // Hooks stay on the authority path instead of the SQLite acceleration path:
  // the DB table intentionally stores only a small subset and cannot preserve
  // promoted/core/dependency metadata, which is load-bearing for hook debt.
  const hooks = structuredHooks?.hooks ?? parsePendingHooksMarkdown(hooksMarkdown);
  const activeHooks = filterActiveHooks(hooks);

  const summaries = structuredSummaries?.rows ?? parseChapterSummariesMarkdown(chapterSummariesMarkdown);

  return {
    summaries: selectRelevantSummaries(summaries, params.chapterNumber, narrativeQueryTerms),
    hooks: selectRelevantHooks(activeHooks, narrativeQueryTerms, params.chapterNumber),
    activeHooks,
    recyclableHooks: computeRecyclableHooks(activeHooks, params.chapterNumber),
    facts: selectRelevantFacts(facts, factQueryTerms),
    volumeSummaries,
  };
}

/**
 * Phase 9-2: Hooks that the planner MUST address this chapter.
 *
 * An active hook is "recyclable" (i.e., stale enough to force an
 * advance/resolve/defer decision) when any of the following holds:
 *
 *   - pressured / near_payoff / progressing: silent for ≥ 5 chapters
 *   - planted / open: silent for ≥ 10 chapters
 *   - coreHook === true:                      silent for ≥ 8 chapters
 *
 * "Silent" = (chapterNumber − max(startChapter, lastAdvancedChapter)).
 * Future-planted hooks are excluded (they aren't overdue yet).
 * Sorted by silence DESC — most overdue first — so the planner sees the
 * worst debt at the top of its prompt slice.
 */
export function computeRecyclableHooks(
  hooks: ReadonlyArray<StoredHook>,
  chapterNumber: number,
): StoredHook[] {
  return hooks
    .filter((hook) => !isRecycleTerminalStatus(hook.status))
    .filter((hook) => !isFuturePlannedHook(hook, chapterNumber))
    .map((hook) => ({ hook, silence: hookSilence(hook, chapterNumber) }))
    .filter(({ hook, silence }) => silence >= recycleThreshold(hook))
    .sort((a, b) => b.silence - a.silence || a.hook.startChapter - b.hook.startChapter)
    .map(({ hook }) => hook);
}

function isRecycleTerminalStatus(status: string): boolean {
  return /^(resolved|closed|done|已回收|已解决|deferred|paused|hold|延后|延期|搁置|暂缓)$/i.test(status.trim());
}

function hookSilence(hook: StoredHook, chapterNumber: number): number {
  const lastTouch = Math.max(hook.startChapter, hook.lastAdvancedChapter);
  if (lastTouch <= 0) return chapterNumber;
  return Math.max(0, chapterNumber - lastTouch);
}

function recycleThreshold(hook: StoredHook): number {
  const status = hook.status.trim().toLowerCase();
  if (/pressured|near[_\s-]?payoff|progressing|重大推进|持续推进/.test(status)) return 5;
  if (hook.coreHook === true) return 8;
  return 10;
}

export function extractQueryTerms(goal: string, outlineNode: string | undefined, mustKeep: ReadonlyArray<string>): string[] {
  const primaryTerms = uniqueTerms([
    ...extractTermsFromText(stripNegativeGuidance(goal)),
    ...mustKeep.flatMap((item) => extractTermsFromText(item)),
  ]);

  if (primaryTerms.length >= 2) {
    return primaryTerms.slice(0, 12);
  }

  return uniqueTerms([
    ...primaryTerms,
    ...extractTermsFromText(stripNegativeGuidance(outlineNode ?? "")),
  ]).slice(0, 12);
}

async function readStructuredState<T>(
  path: string,
  schema: { parse(value: unknown): T },
): Promise<T | null> {
  try {
    const raw = await ipc<string>("fs_read_file", { path });
    return schema.parse(JSON.parse(raw));
  } catch {
    return null;
  }
}

// 已删除：buildLegacyQueryTerms（源文件中即为死代码 —— 从未被调用，
// 也不导出；保留它仅为防万一的"接口对称"。Mnemosyne 启用 noUnusedLocals，
// 保留会报 TS6133，故删除。删除不影响任何函数逻辑/正则/常量，与 state-bootstrap.ts
// 迁移时删除 parseStrictIntegerWithWarning / parseStrictIntegerCell 死代码链一致。）

function extractTermsFromText(text: string): string[] {
  if (!text.trim()) return [];

  const stopWords = new Set([
    "bring", "focus", "back", "chapter", "clear", "narrative", "before", "opening",
    "track", "the", "with", "from", "that", "this", "into", "still", "cannot",
    "current", "state", "advance", "conflict", "story", "keep", "must", "local",
    "does", "not", "only", "just", "then", "than",
  ]);

  const normalized = text.replace(/第\d+章/g, " ");
  const english = (normalized.match(/[a-z]{4,}/gi) ?? [])
    .map((term) => term.trim())
    .filter((term) => term.length >= 2)
    .filter((term) => !stopWords.has(term.toLowerCase()));

  const chineseSegments = normalized.match(/[\u4e00-\u9fff]{2,}/g) ?? [];
  const chinese = chineseSegments.flatMap((segment) => extractChineseFocusTerms(segment));

  return [...english, ...chinese];
}

function extractChineseFocusTerms(segment: string): string[] {
  const stripped = segment
    .replace(/^(本章|继续|重新|拉回|回到|推进|优先|围绕|聚焦|坚持|保持|把注意力|注意力|将注意力|请把注意力|先把注意力)+/, "")
    .replace(/^(处理|推进|回拉|拉回到)+/, "")
    .trim();

  const target = stripped.length >= 2 ? stripped : segment;
  const terms = new Set<string>();

  if (target.length <= 4) {
    terms.add(target);
  }

  for (let size = 2; size <= 4; size += 1) {
    if (target.length >= size) {
      terms.add(target.slice(-size));
    }
  }

  return [...terms].filter((term) => term.length >= 2);
}

function stripNegativeGuidance(text: string): string {
  if (!text) return "";

  return text
    .replace(/\b(do not|don't|avoid|without|instead of)\b[\s\S]*$/i, " ")
    .replace(/(?:不要|不让|别|禁止|避免|但不允许)[\s\S]*$/u, " ")
    .trim();
}

function uniqueTerms(terms: ReadonlyArray<string>): string[] {
  const result: string[] = [];
  const seen = new Set<string>();

  for (const term of terms) {
    const normalized = term.trim().toLowerCase();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    result.push(term.trim());
  }

  return result;
}

function parseVolumeSummariesMarkdown(markdown: string): VolumeSummarySelection[] {
  if (!markdown.trim()) return [];

  const sections = markdown
    .split(/^##\s+/m)
    .map((section) => section.trim())
    .filter(Boolean);

  return sections.map((section) => {
    const [headingLine, ...bodyLines] = section.split("\n");
    const heading = headingLine?.trim() ?? "";
    const content = bodyLines.join("\n").trim();

    return {
      heading,
      content,
      anchor: slugifyAnchor(heading),
    };
  }).filter((section) => section.heading.length > 0 && section.content.length > 0);
}

function isUnresolvedHook(status: string): boolean {
  return status.trim().length === 0 || /open|待定|推进|active|progressing/i.test(status);
}

function selectRelevantSummaries(
  summaries: ReadonlyArray<StoredSummary>,
  chapterNumber: number,
  queryTerms: ReadonlyArray<string>,
): StoredSummary[] {
  return summaries
    .filter((summary) => summary.chapter < chapterNumber)
    .map((summary) => ({
      summary,
      score: scoreSummary(summary, chapterNumber, queryTerms),
      matched: matchesAny([
        summary.title,
        summary.characters,
        summary.events,
        summary.stateChanges,
        summary.hookActivity,
        summary.chapterType,
      ].join(" "), queryTerms),
    }))
    .filter((entry) => entry.matched || entry.summary.chapter >= chapterNumber - 3)
    .sort((left, right) => right.score - left.score || right.summary.chapter - left.summary.chapter)
    .slice(0, 4)
    .map((entry) => entry.summary)
    .sort((left, right) => left.chapter - right.chapter);
}

function selectRelevantHooks(
  hooks: ReadonlyArray<StoredHook>,
  queryTerms: ReadonlyArray<string>,
  chapterNumber: number,
): StoredHook[] {
  const ranked = hooks
    .map((hook) => ({
      hook,
      score: scoreHook(hook, queryTerms, chapterNumber),
      matched: matchesAny(
        [hook.hookId, hook.type, hook.expectedPayoff, hook.payoffTiming ?? "", hook.notes].join(" "),
        queryTerms,
      ),
    }))
    .filter((entry: { hook: StoredHook; score: number; matched: boolean }) =>
      entry.matched || isUnresolvedHook(entry.hook.status),
    );

  const primary = ranked
    .filter((entry: { hook: StoredHook; score: number; matched: boolean }) =>
      entry.matched || isHookWithinChapterWindow(entry.hook, chapterNumber, 5),
    )
    .sort((left, right) => right.score - left.score || right.hook.lastAdvancedChapter - left.hook.lastAdvancedChapter)
    .slice(0, 6);

  const selectedIds = new Set(primary.map((entry: { hook: StoredHook; score: number; matched: boolean }) => entry.hook.hookId));
  const stale = ranked
    .filter((entry: { hook: StoredHook; score: number; matched: boolean }) =>
      !selectedIds.has(entry.hook.hookId)
      && !isFuturePlannedHook(entry.hook, chapterNumber)
      && isUnresolvedHook(entry.hook.status),
    )
    .sort((left, right) => left.hook.lastAdvancedChapter - right.hook.lastAdvancedChapter || right.score - left.score)
    .slice(0, 2);

  return [...primary, ...stale].map((entry: { hook: StoredHook; score: number; matched: boolean }) => entry.hook);
}

function selectRelevantFacts(
  facts: ReadonlyArray<Fact>,
  queryTerms: ReadonlyArray<string>,
): Fact[] {
  const prioritizedPredicates = [
    /^(当前冲突|current conflict)$/i,
    /^(当前目标|current goal)$/i,
    /^(主角状态|protagonist state)$/i,
    /^(当前限制|current constraint)$/i,
    /^(当前位置|current location)$/i,
    /^(当前敌我|current alliances|current relationships)$/i,
  ];

  return facts
    .map((fact) => {
      const text = [fact.subject, fact.predicate, fact.object].join(" ");
      const priority = prioritizedPredicates.findIndex((pattern) => pattern.test(fact.predicate));
      const baseScore = priority === -1 ? 5 : 20 - priority * 2;
      const termScore = queryTerms.reduce(
        (score, term) => score + (includesTerm(text, term) ? Math.max(8, term.length * 2) : 0),
        0,
      );

      return {
        fact,
        score: baseScore + termScore,
        matched: matchesAny(text, queryTerms),
      };
    })
    .filter((entry) => entry.matched || entry.score >= 14)
    .sort((left, right) => right.score - left.score)
    .slice(0, 4)
    .map((entry) => entry.fact);
}

function selectRelevantVolumeSummaries(
  summaries: ReadonlyArray<VolumeSummarySelection>,
  queryTerms: ReadonlyArray<string>,
): VolumeSummarySelection[] {
  if (summaries.length === 0) return [];

  const ranked = summaries
    .map((summary, index) => {
      const text = `${summary.heading} ${summary.content}`;
      const termScore = queryTerms.reduce(
        (score, term) => score + (includesTerm(text, term) ? Math.max(8, term.length * 2) : 0),
        0,
      );

      return {
        index,
        summary,
        score: termScore + index,
        matched: matchesAny(text, queryTerms),
      };
    })
    .filter((entry, index, all) => entry.matched || index === all.length - 1)
    .sort((left, right) => right.score - left.score)
    .slice(0, 2)
    .sort((left, right) => left.index - right.index)
    .map((entry) => entry.summary);

  return ranked;
}

function scoreSummary(summary: StoredSummary, chapterNumber: number, queryTerms: ReadonlyArray<string>): number {
  const text = [
    summary.title,
    summary.characters,
    summary.events,
    summary.stateChanges,
    summary.hookActivity,
    summary.chapterType,
  ].join(" ");
  const age = Math.max(0, chapterNumber - summary.chapter);
  const recencyScore = Math.max(0, 12 - age);
  const termScore = queryTerms.reduce((score, term) => score + (includesTerm(text, term) ? Math.max(8, term.length * 2) : 0), 0);
  return recencyScore + termScore;
}

function scoreHook(
  hook: StoredHook,
  queryTerms: ReadonlyArray<string>,
  _chapterNumber: number,
): number {
  const text = [hook.hookId, hook.type, hook.expectedPayoff, hook.payoffTiming ?? "", hook.notes].join(" ");
  const freshness = Math.max(0, hook.lastAdvancedChapter);
  const termScore = queryTerms.reduce((score, term) => score + (includesTerm(text, term) ? Math.max(8, term.length * 2) : 0), 0);
  return termScore + freshness;
}

function matchesAny(text: string, queryTerms: ReadonlyArray<string>): boolean {
  return queryTerms.some((term) => includesTerm(text, term));
}

function includesTerm(text: string, term: string): boolean {
  return text.toLowerCase().includes(term.toLowerCase());
}

function slugifyAnchor(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff]+/g, "-")
    .replace(/^-+|-+$/g, "")
    || "volume-summary";
}
