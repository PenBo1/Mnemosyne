// Hook 生命周期建模。
//
// 迁移要点（import 路径调整）：
// - `../models/runtime-state.js`（类型 HookPayoffTiming）→ `@/shared/types/runtime-state`
// - `../state/memory-db.js`（仅类型 StoredHook）→ `@/shared/types/hook`
// - `./hook-policy.js` → `./hook-policy`（同目录去 .js 后缀）
//
// 简化决策（Mnemosyne 跳过 payoffTiming enum 推导链路，story-markdown.ts 直接展示原始字符串）：
//
// 保留并迁移（timing 无关的纯函数，业务逻辑零改动）：
//   - normalizeStoredHookStatus / filterActiveHooks / isFuturePlannedHook /
//     isHookWithinChapterWindow / resolveHookPhase（原文件私有 helper，timing 无关）
//   - DEFAULT_HOOK_LOOKAHEAD_CHAPTERS 常量
//   注：原任务描述提及的 `describeHookPhase` 在源文件中不存在，实际为 `resolveHookPhase`。
//
// 跳过（仅服务于 enum 推导或本地化展示）：
//   - normalizeHookPayoffTiming / inferHookPayoffTiming / localizeHookPayoffTiming
//     （story-markdown.ts 已内联简化版 normalizeHookPayoffTiming，保留原始字符串）
//   - LABELS / TIMING_ALIASES / SIGNAL_PATTERNS 常量
//
// 简化：
//   - resolveHookPayoffTiming：直接返回 payoffTiming 原始字符串，不做 enum 推导。
//     返回类型由 HookPayoffTiming 改为 string | undefined。
//   - describeHookLifecycle：保留函数签名（新增可选 halfLifeChapters 参数）。
//     依赖 HOOK_TIMING_PROFILES 的部分改用 halfLifeChapters 粗略判断：
//       earliestResolveAge = max(1, floor(halfLife / 2))
//       staleDormancyThreshold = halfLife
//       overdueAgeThreshold = halfLife * 2
//     halfLifeChapters 缺失时退回 HOOK_HEALTH_DEFAULTS.staleAfterChapters 保守阈值。
//     cadenceReady（原基于 timing enum 的 slow-burn/endgame 分支）不再适用，移除。
//     phaseReady 改为 phase >= middle（保守：opening 阶段不判定 stale/overdue/ready）。
//     resolvePressure 中 profile.resolveBias * resolveBiasMultiplier 改为固定 resolveBiasMultiplier
//     （等价 resolveBias=1，保守下界）。
//
// TODO: P2 阶段 7 三层记忆接入时补 timing 推导，恢复 HOOK_TIMING_PROFILES 查表逻辑。
//
// 注意：简化后不再使用 HookPayoffTiming 类型与 HOOK_TIMING_PROFILES 常量，
//       故 hook-lifecycle.ts 不 import runtime-state / HOOK_TIMING_PROFILES（避免 noUnusedLocals）。

import type { StoredHook } from "@/types/hook";
import {
  HOOK_ACTIVITY_THRESHOLDS,
  HOOK_HEALTH_DEFAULTS,
  HOOK_PHASE_THRESHOLDS,
  HOOK_PHASE_WEIGHT,
  HOOK_PRESSURE_WEIGHTS,
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

/**
 * 简化版：直接返回 payoffTiming 原始字符串，不做 enum 推导。
 *
 * TODO: P2 阶段 7 三层记忆接入时补 timing 推导（恢复 TIMING_ALIASES / SIGNAL_PATTERNS 匹配）。
 */
export function resolveHookPayoffTiming(params: {
  readonly payoffTiming?: string | null;
  readonly expectedPayoff?: string;
  readonly notes?: string;
}): string | undefined {
  const trimmed = params.payoffTiming?.trim();
  return trimmed ? trimmed : undefined;
}

export function describeHookLifecycle(params: {
  readonly payoffTiming?: string | null;
  readonly expectedPayoff?: string;
  readonly notes?: string;
  readonly startChapter: number;
  readonly lastAdvancedChapter: number;
  readonly status: string;
  readonly chapterNumber: number;
  readonly targetChapters?: number;
  readonly halfLifeChapters?: number;
}): {
  readonly timing: string | undefined;
  readonly phase: HookPhase;
  readonly age: number;
  readonly dormancy: number;
  readonly readyToResolve: boolean;
  readonly stale: boolean;
  readonly overdue: boolean;
  readonly advancePressure: number;
  readonly resolvePressure: number;
} {
  const timing = resolveHookPayoffTiming(params);
  const phase = resolveHookPhase(params.chapterNumber, params.targetChapters);
  const age = Math.max(0, params.chapterNumber - Math.max(1, params.startChapter));
  const lastTouchChapter = Math.max(params.startChapter, params.lastAdvancedChapter);
  const dormancy = Math.max(0, params.chapterNumber - Math.max(1, lastTouchChapter));
  const explicitProgressing = /^(progressing|advanced|重大推进|持续推进)$/i.test(params.status.trim());
  // 简化：原 phaseReady = HOOK_PHASE_WEIGHT[phase] >= HOOK_PHASE_WEIGHT[profile.minimumPhase]，
  //       依赖 timing profile。改为保守判定 phase >= middle（opening 阶段不触发 stale/overdue/ready）。
  const phaseReady = HOOK_PHASE_WEIGHT[phase] >= HOOK_PHASE_WEIGHT.middle;
  const recentlyTouched = dormancy <= HOOK_ACTIVITY_THRESHOLDS.recentlyTouchedDormancy;

  // 简化：未推导 timing profile，使用 halfLifeChapters 做粗略判断；
  //       缺失时退回 HOOK_HEALTH_DEFAULTS.staleAfterChapters 保守阈值。
  // TODO: P2 阶段 7 三层记忆接入时补 timing 推导，恢复 HOOK_TIMING_PROFILES 查表逻辑。
  const halfLife = typeof params.halfLifeChapters === "number" && params.halfLifeChapters > 0
    ? params.halfLifeChapters
    : undefined;
  const staleDormancyThreshold = halfLife ?? HOOK_HEALTH_DEFAULTS.staleAfterChapters;
  const overdueAgeThreshold = halfLife ? halfLife * 2 : HOOK_HEALTH_DEFAULTS.staleAfterChapters * 2;
  const earliestResolveAge = halfLife ? Math.max(1, Math.floor(halfLife / 2)) : 1;

  const overdue = phaseReady && age >= overdueAgeThreshold;
  const momentum = explicitProgressing || recentlyTouched;
  const stale = phaseReady && (
    dormancy >= staleDormancyThreshold
    || (overdue && !momentum)
  );
  // 简化：移除原 cadenceReady 分支（依赖 timing enum 的 slow-burn/endgame 判定）。
  const readyToResolve = phaseReady
    && age >= earliestResolveAge
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
    // 简化：原 resolvePressure = profile.resolveBias * resolveBiasMultiplier + ...
    //       改为固定 resolveBiasMultiplier（等价 resolveBias=1，保守下界）。
    resolvePressure: readyToResolve
      ? HOOK_PRESSURE_WEIGHTS.resolveBiasMultiplier
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
