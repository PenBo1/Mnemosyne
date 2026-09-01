// Hook 策略常量表。
//
// 迁移要点：
// - import 路径：`../models/runtime-state.js`（仅类型 HookPayoffTiming）→ `@/shared/types/runtime-state`。
// - 业务逻辑零改动。
// - 简化决策（按迁移规则）：
//   - 保留全部常量：HOOK_TIMING_PROFILES / HOOK_PHASE_WEIGHT / HOOK_PHASE_THRESHOLDS /
//     HOOK_PRESSURE_WEIGHTS / HOOK_ACTIVITY_THRESHOLDS / HOOK_VISIBILITY_WINDOWS /
//     HOOK_RELEVANT_SELECTION_DEFAULTS / HOOK_HEALTH_DEFAULTS。
//   - HOOK_TIMING_PROFILES / HOOK_VISIBILITY_WINDOWS 等 Record<HookPayoffTiming, ...> 常量保留：
//     hook-governance / hook-arbiter 后续可能用到 timing profile 做债务计算。
//   - resolveHookVisibilityWindow 包装函数保留。
//   - 本文件无 localize 映射表（LABELS 在 hook-lifecycle.ts，已在迁移中跳过），故无跳过项。

import type { HookPayoffTiming } from "@/features/agent/types/runtime-state";

export type HookPhase = "opening" | "middle" | "late";

export interface HookLifecycleProfile {
  readonly earliestResolveAge: number;
  readonly staleDormancy: number;
  readonly overdueAge: number;
  readonly minimumPhase: HookPhase;
  readonly resolveBias: number;
}

export const HOOK_TIMING_PROFILES: Record<HookPayoffTiming, HookLifecycleProfile> = {
  immediate: {
    earliestResolveAge: 1,
    staleDormancy: 1,
    overdueAge: 3,
    minimumPhase: "opening",
    resolveBias: 5,
  },
  "near-term": {
    earliestResolveAge: 1,
    staleDormancy: 2,
    overdueAge: 5,
    minimumPhase: "opening",
    resolveBias: 4,
  },
  "mid-arc": {
    earliestResolveAge: 2,
    staleDormancy: 4,
    overdueAge: 8,
    minimumPhase: "opening",
    resolveBias: 3,
  },
  "slow-burn": {
    earliestResolveAge: 4,
    staleDormancy: 5,
    overdueAge: 12,
    minimumPhase: "middle",
    resolveBias: 2,
  },
  endgame: {
    earliestResolveAge: 6,
    staleDormancy: 6,
    overdueAge: 16,
    minimumPhase: "late",
    resolveBias: 1,
  },
};

export const HOOK_PHASE_WEIGHT: Record<HookPhase, number> = {
  opening: 0,
  middle: 1,
  late: 2,
};

export const HOOK_PHASE_THRESHOLDS = {
  middleProgress: 0.33,
  lateProgress: 0.72,
  middleChapter: 8,
  lateChapter: 24,
} as const;

export const HOOK_PRESSURE_WEIGHTS = {
  staleAdvanceBonus: 8,
  overdueAdvanceBonus: 6,
  resolveBiasMultiplier: 10,
  progressingResolveBonus: 5,
  dormancyResolveMultiplier: 2,
  maxDormancyResolveBonus: 12,
  overdueResolveBonus: 10,
  mustAdvancePressureFloor: 8,
  criticalResolvePressure: 40,
} as const;

export const HOOK_ACTIVITY_THRESHOLDS = {
  recentlyTouchedDormancy: 1,
  longArcQuietHoldMaxAge: 2,
  longArcQuietHoldMaxDormancy: 1,
  refreshDormancy: 2,
  freshPromiseAge: 1,
} as const;

export const HOOK_VISIBILITY_WINDOWS: Record<HookPayoffTiming, number> = {
  immediate: 5,
  "near-term": 5,
  "mid-arc": 6,
  "slow-burn": 8,
  endgame: 10,
};

export const HOOK_RELEVANT_SELECTION_DEFAULTS = {
  primary: {
    baseLimit: 3,
    pressuredExpansionLimit: 4,
    pressuredThreshold: 4,
  },
  stale: {
    defaultLimit: 1,
    expandedLimit: 2,
    overdueThreshold: 2,
    familySpreadThreshold: 2,
  },
} as const;

export const HOOK_HEALTH_DEFAULTS = {
  maxActiveHooks: 12,
  staleAfterChapters: 10,
  noAdvanceWindow: 5,
  newHookBurstThreshold: 2,
} as const;

export function resolveHookVisibilityWindow(timing: HookPayoffTiming): number {
  return HOOK_VISIBILITY_WINDOWS[timing];
}
