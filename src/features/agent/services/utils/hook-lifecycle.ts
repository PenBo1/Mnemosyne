// Hook 生命周期建模。
//
// 迁移要点（import 路径调整）：
// - `../models/runtime-state.js`（类型 HookPayoffTiming）→ `@/types/runtime-state`
// - `../state/memory-db.js`（仅类型 StoredHook）→ `@/types/hook`
// - `./hook-policy.js` → `./hook-policy`（同目录去 .js 后缀）
//
// 本文件实现完整 timing 推导链路（5 级优先级，P2.7 恢复）：
//   - LABELS / TIMING_ALIASES / SIGNAL_PATTERNS / TYPE_TIMING_DEFAULTS 常量齐全
//   - normalizeHookPayoffTiming / inferHookPayoffTiming / localizeHookPayoffTiming 三件套
//   - matchTimingKeyword / inferTimingFromType / inferTimingFromChapterSpan 三个推导辅助
//   - resolveHookPayoffTiming 返回 HookPayoffTiming | undefined，按 5 级优先级推导：
//       1. 显式 payoffTiming（已由 zod 校验为 enum）
//       2. expectedPayoff 关键词匹配
//       3. notes 关键词匹配
//       4. hook.type 默认 timing 映射（foreshadowing→mid-arc / mystery→slow-burn …）
//       5. 章节跨度推断（currentChapter - startChapter）
//     无法推断时返回 undefined，由 describeHookLifecycle 兜底为 "mid-arc"
//   - describeHookLifecycle 使用 HOOK_TIMING_PROFILES[timing] 查表，恢复 cadenceReady 与
//     profile.resolveBias * resolveBiasMultiplier 计算
//   - 移除 halfLifeChapters 粗略估算参数（由 timing profile 取代）

import type { HookPayoffTiming } from "@/types/runtime-state";
import type { StoredHook } from "@/types/hook";
import {
  HOOK_ACTIVITY_THRESHOLDS,
  HOOK_PHASE_THRESHOLDS,
  HOOK_PHASE_WEIGHT,
  HOOK_PRESSURE_WEIGHTS,
  HOOK_TIMING_PROFILES,
  type HookPhase,
} from "./hook-policy";

export const DEFAULT_HOOK_LOOKAHEAD_CHAPTERS = 3;

export function normalizeStoredHookStatus(status: string): "resolved" | "deferred" | "progressing" | "open" {
  if (/^(resolved|closed|done|已回收|已解决)$/i.test(status.trim())) return "resolved";
  if (/^(deferred|paused|hold|dormant|sleeping|延后|延期|搁置|暂缓|未开启|待开启|未启动|待启动|待推进)$/i.test(status.trim())) return "deferred";
  if (/^(progressing|advanced|重大推进|持续推进)$/i.test(status.trim())) return "progressing";
  return "open";
}

export function filterActiveHooks(hooks: ReadonlyArray<StoredHook>): StoredHook[] {
  return hooks.filter((hook) => {
    const status = normalizeStoredHookStatus(hook.status);
    if (status === "resolved" || status === "deferred") return false;
    // promoted=false means this is still an architect seed, not live hook debt.
    // Legacy rows without the promoted column keep the old behavior.
    return hook.promoted !== false;
  });
}

export function isFuturePlannedHook(
  hook: StoredHook,
  chapterNumber: number,
  lookahead: number = DEFAULT_HOOK_LOOKAHEAD_CHAPTERS,
): boolean {
  return hook.lastAdvancedChapter <= 0 && hook.startChapter > chapterNumber + lookahead;
}

export function isHookWithinChapterWindow(
  hook: StoredHook,
  chapterNumber: number,
  recentWindow: number = 5,
  lookahead: number = DEFAULT_HOOK_LOOKAHEAD_CHAPTERS,
): boolean {
  const recentCutoff = Math.max(0, chapterNumber - recentWindow);

  if (hook.lastAdvancedChapter > 0 && hook.lastAdvancedChapter >= recentCutoff) {
    return true;
  }

  if (hook.lastAdvancedChapter > 0) {
    return false;
  }

  if (hook.startChapter <= 0) {
    return true;
  }

  if (hook.startChapter >= recentCutoff && hook.startChapter <= chapterNumber) {
    return true;
  }

  return hook.startChapter > chapterNumber && hook.startChapter <= chapterNumber + lookahead;
}

const LABELS: Record<"zh" | "en", Record<HookPayoffTiming, string>> = {
  en: {
    immediate: "immediate",
    "near-term": "near-term",
    "mid-arc": "mid-arc",
    "slow-burn": "slow-burn",
    endgame: "endgame",
  },
  zh: {
    immediate: "立即",
    "near-term": "近期",
    "mid-arc": "中程",
    "slow-burn": "慢烧",
    endgame: "终局",
  },
};

const TIMING_ALIASES: ReadonlyArray<[HookPayoffTiming, RegExp]> = [
  ["immediate", /^(?:立即|马上|当章|本章|下一章|immediate|instant|next(?:\s+chapter|\s+beat)?|right\s+away)$/i],
  ["near-term", /^(?:近期|近几章|短线|soon|short(?:\s+run)?|near(?:\s*-\s*|\s+)term|current\s+sequence)$/i],
  ["mid-arc", /^(?:中程|中期|卷中|mid(?:\s*-\s*|\s+)arc|mid(?:\s*-\s*|\s+)book|middle)$/i],
  ["slow-burn", /^(?:慢烧|长线|后续|later|late(?:r)?|long(?:\s*-\s*|\s+)arc|slow(?:\s*-\s*|\s+)burn)$/i],
  ["endgame", /^(?:终局|终章|大结局|最终|climax|finale|endgame|late\s+book)$/i],
];

const SIGNAL_PATTERNS: ReadonlyArray<[HookPayoffTiming, RegExp]> = [
  ["endgame", /(终局|终章|大结局|最终揭晓|最终摊牌|climax|finale|endgame|final reveal|last act)/i],
  ["immediate", /(当章|本章|下一章|马上|立刻|即刻|immediate|next chapter|right away|at once)/i],
  ["near-term", /(近期|近几章|很快|短线|soon|near-term|short run|current sequence)/i],
  ["mid-arc", /(中期|卷中|本卷中段|mid-book|mid arc|middle of the arc)/i],
  ["slow-burn", /(长线|慢烧|后续发酵|慢慢揭开|later|slow burn|long arc|long tail)/i],
];

// 优先级 4：hook.type → 默认 timing 映射表。
// 按 hook 类别推断常见 timing 节奏（伏笔中期回收、悬念长期发酵等）。
// 匹配方式：case-insensitive contains（hook.type 通常是 "foreshadowing" / "mystery" 等短标签）。
const TYPE_TIMING_DEFAULTS: ReadonlyArray<[HookPayoffTiming, RegExp]> = [
  // callback / 回调 / 呼应 — 通常当章或下一章触发
  ["immediate", /(?:callback|回调|呼应)/i],
  // promise / 承诺 / 约定 — 通常近期兑现
  ["near-term", /(?:promise|承诺|约定)/i],
  // foreshadowing / 伏笔 / 铺垫 — 通常中期回收
  ["mid-arc", /(?:foreshadow|伏笔|铺垫)/i],
  // setup / 设置 / 布置 — 通常中期展开
  ["mid-arc", /(?:setup|设置|布置)/i],
  // mystery / 悬念 / 悬疑 — 通常长期发酵
  ["slow-burn", /(?:mystery|悬念|悬疑)/i],
];

export function normalizeHookPayoffTiming(value: string | undefined | null): HookPayoffTiming | undefined {
  const normalized = value?.trim();
  if (!normalized) return undefined;

  for (const [timing, pattern] of TIMING_ALIASES) {
    if (pattern.test(normalized)) {
      return timing;
    }
  }

  return undefined;
}

/**
 * 对单个文本字段做 SIGNAL_PATTERNS 关键词匹配（优先级 2/3 共用）。
 * 返回首个命中的 timing，无匹配返回 undefined。
 */
function matchTimingKeyword(text: string | undefined | null): HookPayoffTiming | undefined {
  const trimmed = text?.trim();
  if (!trimmed) return undefined;

  for (const [timing, pattern] of SIGNAL_PATTERNS) {
    if (pattern.test(trimmed)) {
      return timing;
    }
  }

  return undefined;
}

/**
 * 优先级 4：从 hook.type 推断默认 timing。
 * foreshadowing → mid-arc、mystery → slow-burn、promise → near-term、setup → mid-arc、callback → immediate。
 * 无匹配返回 undefined。
 */
function inferTimingFromType(type: string | undefined): HookPayoffTiming | undefined {
  const trimmed = type?.trim();
  if (!trimmed) return undefined;

  for (const [timing, pattern] of TYPE_TIMING_DEFAULTS) {
    if (pattern.test(trimmed)) {
      return timing;
    }
  }

  return undefined;
}

/**
 * 优先级 5：从章节跨度推断 timing。
 * 跨度 = currentChapter - startChapter（已存在章节数）。
 *   < 3 章 → immediate（短线 hook，应尽快回收）
 *   3–9 章 → near-term
 *   10–29 章 → mid-arc
 *   ≥ 30 章 → slow-burn（长期埋线）
 * startChapter 或 currentChapter 缺失/无效时返回 undefined。
 */
function inferTimingFromChapterSpan(
  startChapter: number | undefined,
  currentChapter: number | undefined,
): HookPayoffTiming | undefined {
  if (startChapter === undefined || currentChapter === undefined) return undefined;
  if (startChapter < 0 || currentChapter < 0) return undefined;

  const span = currentChapter - startChapter;
  if (span < 0) return undefined;

  if (span < 3) return "immediate";
  if (span < 10) return "near-term";
  if (span < 30) return "mid-arc";
  return "slow-burn";
}

export function inferHookPayoffTiming(params: {
  readonly expectedPayoff?: string;
  readonly notes?: string;
}): HookPayoffTiming {
  // 优先级 2：expectedPayoff 关键词
  // 优先级 3：notes 关键词
  // 兜底：mid-arc（保守中性值，describeHookLifecycle 需要 non-undefined timing 查 profile 表）
  return matchTimingKeyword(params.expectedPayoff)
    ?? matchTimingKeyword(params.notes)
    ?? "mid-arc";
}

/**
 * 完整 timing 推导（5 级优先级，P2.7 恢复）。
 *
 * 优先级从高到低：
 *   1. 显式 payoffTiming（已由 zod 校验为 HookPayoffTiming enum 或原始字符串，经
 *      normalizeHookPayoffTiming 规范化）
 *   2. expectedPayoff 关键词匹配（SIGNAL_PATTERNS，中英双语）
 *   3. notes 关键词匹配（同上）
 *   4. hook.type 默认 timing 映射（TYPE_TIMING_DEFAULTS：
 *      foreshadowing→mid-arc / mystery→slow-burn / promise→near-term /
 *      setup→mid-arc / callback→immediate）
 *   5. 章节跨度推断（currentChapter - startChapter：
 *      <3→immediate / <10→near-term / <30→mid-arc / ≥30→slow-burn）
 *
 * 全部未命中时返回 undefined。调用方（describeHookLifecycle）兜底为 "mid-arc"
 * 以保证 HOOK_TIMING_PROFILES[timing] 查表安全。
 */
export function resolveHookPayoffTiming(params: {
  readonly payoffTiming?: string | null;
  readonly expectedPayoff?: string;
  readonly notes?: string;
  readonly type?: string;
  readonly startChapter?: number;
  readonly currentChapter?: number;
}): HookPayoffTiming | undefined {
  // 优先级 1：显式 payoffTiming
  const explicit = normalizeHookPayoffTiming(params.payoffTiming);
  if (explicit) return explicit;

  // 优先级 2：expectedPayoff 关键词
  const fromExpected = matchTimingKeyword(params.expectedPayoff);
  if (fromExpected) return fromExpected;

  // 优先级 3：notes 关键词
  const fromNotes = matchTimingKeyword(params.notes);
  if (fromNotes) return fromNotes;

  // 优先级 4：hook.type 默认 timing
  const fromType = inferTimingFromType(params.type);
  if (fromType) return fromType;

  // 优先级 5：章节跨度
  const fromSpan = inferTimingFromChapterSpan(params.startChapter, params.currentChapter);
  if (fromSpan) return fromSpan;

  // 无法推断
  return undefined;
}

export function localizeHookPayoffTiming(
  timing: HookPayoffTiming,
  language: "zh" | "en",
): string {
  return LABELS[language][timing];
}

export function describeHookLifecycle(params: {
  readonly payoffTiming?: string | null;
  readonly expectedPayoff?: string;
  readonly notes?: string;
  readonly type?: string;
  readonly startChapter: number;
  readonly lastAdvancedChapter: number;
  readonly status: string;
  readonly chapterNumber: number;
  readonly targetChapters?: number;
}): {
  readonly timing: HookPayoffTiming;
  readonly phase: HookPhase;
  readonly age: number;
  readonly dormancy: number;
  readonly readyToResolve: boolean;
  readonly stale: boolean;
  readonly overdue: boolean;
  readonly advancePressure: number;
  readonly resolvePressure: number;
} {
  const timing = resolveHookPayoffTiming(params) ?? "mid-arc";
  const profile = HOOK_TIMING_PROFILES[timing];
  const phase = resolveHookPhase(params.chapterNumber, params.targetChapters);
  const age = Math.max(0, params.chapterNumber - Math.max(1, params.startChapter));
  const lastTouchChapter = Math.max(params.startChapter, params.lastAdvancedChapter);
  const dormancy = Math.max(0, params.chapterNumber - Math.max(1, lastTouchChapter));
  const explicitProgressing = /^(progressing|advanced|重大推进|持续推进)$/i.test(params.status.trim());
  const phaseReady = HOOK_PHASE_WEIGHT[phase] >= HOOK_PHASE_WEIGHT[profile.minimumPhase];
  const recentlyTouched = dormancy <= HOOK_ACTIVITY_THRESHOLDS.recentlyTouchedDormancy;
  const overdue = phaseReady && age >= profile.overdueAge;
  // cadenceReady:slow-burn 需 late 阶段或 overdue;endgame 需 late 阶段;其余 timing 默认 true。
  const cadenceReady = timing === "slow-burn"
    ? phase === "late" || overdue
    : timing === "endgame"
      ? phase === "late"
      : true;
  const momentum = explicitProgressing || recentlyTouched;
  const stale = phaseReady && (
    dormancy >= profile.staleDormancy
    || (overdue && !momentum)
  );
  const readyToResolve = phaseReady
    && cadenceReady
    && age >= profile.earliestResolveAge
    && (momentum || (overdue && explicitProgressing));

  return {
    timing,
    phase,
    age,
    dormancy,
    readyToResolve,
    stale,
    overdue,
    advancePressure: age
      + dormancy
      + (stale ? HOOK_PRESSURE_WEIGHTS.staleAdvanceBonus : 0)
      + (overdue ? HOOK_PRESSURE_WEIGHTS.overdueAdvanceBonus : 0),
    resolvePressure: readyToResolve
      ? profile.resolveBias * HOOK_PRESSURE_WEIGHTS.resolveBiasMultiplier
        + (explicitProgressing ? HOOK_PRESSURE_WEIGHTS.progressingResolveBonus : 0)
        + Math.min(
          HOOK_PRESSURE_WEIGHTS.maxDormancyResolveBonus,
          dormancy * HOOK_PRESSURE_WEIGHTS.dormancyResolveMultiplier,
        )
        + (overdue ? HOOK_PRESSURE_WEIGHTS.overdueResolveBonus : 0)
      : 0,
  };
}

function resolveHookPhase(chapterNumber: number, targetChapters?: number): HookPhase {
  if (targetChapters && targetChapters > 0) {
    const progress = chapterNumber / targetChapters;
    if (progress >= HOOK_PHASE_THRESHOLDS.lateProgress) return "late";
    if (progress >= HOOK_PHASE_THRESHOLDS.middleProgress) return "middle";
    return "opening";
  }

  if (chapterNumber >= HOOK_PHASE_THRESHOLDS.lateChapter) return "late";
  if (chapterNumber >= HOOK_PHASE_THRESHOLDS.middleChapter) return "middle";
  return "opening";
}
