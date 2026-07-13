// Hook promotion —— Phase 7 伏笔升级规则。
//
// architect 产出的初始伏笔池里
// 不是所有 seed 都应进入 live ledger。升级条件（满足任一即升级）：
//   1. cross_volume —— 跨卷伏笔（在更晚卷回收 / 依赖更晚卷的 hook）
//   2. advanced_count >= 2 —— 历史推进 ≥2 章（已证明读者追踪）
//   3. depends_on 非空 —— 有上游因果依赖
//   4. core_hook === true —— 主线承重伏笔
//
// architect 阶段：根据 cross_volume / depends_on / core_hook 做结构性预升级；
// consolidator 阶段：根据 advanced_count 做运行时升级（rerunPromotionPass）。
//
// 纯函数，无 I/O，无外部依赖（除 StoredHook 类型）。

import type { StoredHook } from "@/types/hook";

export interface VolumeBoundary {
  readonly name: string;
  readonly startCh: number;
  readonly endCh: number;
}

export interface PromotionContext {
  /** 从 outline/volume_map.md 解析的卷边界，按 startCh 排序。 */
  readonly volumeBoundaries: ReadonlyArray<VolumeBoundary>;
  /** 当前章号（建书时为 0）。 */
  readonly currentChapter: number;
  /** hook id → 历史推进次数。 */
  readonly advancedCounts: ReadonlyMap<string, number>;
  /** hook id → 起始章号（含未升级的 seed）。用于判断 depends_on 是否在更晚卷声明。 */
  readonly allSeedStartChapters: ReadonlyMap<string, number>;
}

export interface PromotionDecision {
  readonly promote: boolean;
  readonly reasons: ReadonlyArray<PromotionReason>;
}

export type PromotionReason =
  | "cross_volume"
  | "advanced_count"
  | "depends_on"
  | "core_hook";

export function shouldPromoteHook(
  hook: StoredHook,
  context: PromotionContext,
): PromotionDecision {
  const reasons: PromotionReason[] = [];

  if (hook.coreHook === true) {
    reasons.push("core_hook");
  }

  if ((hook.dependsOn?.length ?? 0) > 0) {
    reasons.push("depends_on");
  }

  const advancedCount = hook.advancedCount
    ?? context.advancedCounts.get(hook.hookId)
    ?? 0;
  if (advancedCount >= 2) {
    reasons.push("advanced_count");
  }

  if (isCrossVolume(hook, context)) {
    reasons.push("cross_volume");
  }

  return {
    promote: reasons.length > 0,
    reasons,
  };
}

function isCrossVolume(hook: StoredHook, context: PromotionContext): boolean {
  const { volumeBoundaries, allSeedStartChapters } = context;
  if (volumeBoundaries.length < 2) return false;

  const seedVolumeIndex = findVolumeIndex(volumeBoundaries, hook.startChapter);
  if (seedVolumeIndex < 0) return false;

  // Case A: 上游 hook 声明在更晚的卷。
  for (const upstreamId of hook.dependsOn ?? []) {
    const upstreamStart = allSeedStartChapters.get(upstreamId);
    if (upstreamStart === undefined) continue;
    const upstreamVolumeIndex = findVolumeIndex(volumeBoundaries, upstreamStart);
    if (upstreamVolumeIndex > seedVolumeIndex) return true;
  }

  // Case B: pays_off_in_arc 提到不同卷（只识别明显的"第 N 卷" / "volume N"）。
  const arcVolumeIndex = extractVolumeIndexFromArc(hook.paysOffInArc ?? "");
  if (arcVolumeIndex !== null && arcVolumeIndex !== seedVolumeIndex) {
    return true;
  }

  // Case C: payoff timing 为 endgame / slow-burn 且 seed 在早期卷 —— 跨卷。
  if (
    (hook.payoffTiming === "endgame" || hook.payoffTiming === "slow-burn")
    && seedVolumeIndex < volumeBoundaries.length - 1
  ) {
    return true;
  }

  return false;
}

function findVolumeIndex(
  boundaries: ReadonlyArray<VolumeBoundary>,
  chapter: number,
): number {
  for (let i = 0; i < boundaries.length; i++) {
    const vol = boundaries[i]!;
    if (chapter >= vol.startCh && chapter <= vol.endCh) return i;
  }
  // 第 0 章（seed 阶段）算作卷 0（卷从 1 开始时）。
  if (chapter <= 0 && boundaries.length > 0) return 0;
  return -1;
}

const VOLUME_PATTERNS: ReadonlyArray<RegExp> = [
  /第\s*([一二三四五六七八九十百千\d]+)\s*卷/u,
  /volume\s+(\d+)/i,
  /vol\.?\s*(\d+)/i,
];

function extractVolumeIndexFromArc(arc: string): number | null {
  const trimmed = arc.trim();
  if (!trimmed) return null;

  for (const pattern of VOLUME_PATTERNS) {
    const match = trimmed.match(pattern);
    if (!match) continue;
    const token = match[1] ?? "";
    const n = parseVolumeNumber(token);
    if (n !== null) return n - 1; // prose 中 1-indexed，这里 0-indexed
  }
  return null;
}

const CHINESE_NUMERALS: Readonly<Record<string, number>> = {
  一: 1, 二: 2, 三: 3, 四: 4, 五: 5,
  六: 6, 七: 7, 八: 8, 九: 9, 十: 10,
};

function parseVolumeNumber(token: string): number | null {
  if (/^\d+$/.test(token)) return parseInt(token, 10);
  if (token.length === 1 && CHINESE_NUMERALS[token]) return CHINESE_NUMERALS[token]!;
  if (token === "十") return 10;
  return null;
}

/** 默认半衰期（按 payoff timing 推导）。 */
export function defaultHalfLifeChapters(
  payoffTiming: StoredHook["payoffTiming"] | undefined,
): number {
  switch (payoffTiming) {
    case "immediate":
    case "near-term":
      return 10;
    case "slow-burn":
    case "endgame":
      return 80;
    case "mid-arc":
    default:
      return 30;
  }
}

export function resolveHalfLifeChapters(hook: StoredHook): number {
  return hook.halfLifeChapters ?? defaultHalfLifeChapters(hook.payoffTiming);
}

/** 转义 regex 特殊字符。 */
export function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export interface PromotionPassResult {
  readonly updated: boolean;
  readonly hooks: ReadonlyArray<StoredHook>;
  readonly flippedCount: number;
}

/**
 * 从 chapter_summaries.md 内容推导 hook 推进次数。
 * 统计每个 hook id 在 hookActivity 列（默认 index 5）出现的次数。
 */
export function deriveAdvancedCountsFromSummaries(
  summariesRaw: string,
  hookIds: ReadonlyArray<string>,
): Map<string, number> {
  const counts = new Map<string, number>();
  if (!summariesRaw.trim() || hookIds.length === 0) return counts;

  const lines = summariesRaw.split("\n");
  const hookActivityIndex = detectHookActivityColumnIndex(lines);

  for (const hookId of hookIds) {
    const escaped = escapeRegex(hookId);
    const pattern = new RegExp(`\\b${escaped}\\b`, "i");
    let count = 0;
    for (const line of lines) {
      if (!line.startsWith("|")) continue;
      // 跳过 header / 分隔行。
      if (line.includes("---") || /\|\s*(章节|Chapter)\s*\|/i.test(line)) continue;
      const cell = extractColumn(line, hookActivityIndex);
      if (cell !== null && pattern.test(cell)) count += 1;
    }
    if (count > 0) counts.set(hookId, count);
  }
  return counts;
}

/** 检测 hookActivity 列索引，fallback 到 5（schema 标准位置）。 */
function detectHookActivityColumnIndex(lines: ReadonlyArray<string>): number {
  const DEFAULT_INDEX = 5;
  for (const line of lines) {
    if (!line.startsWith("|")) continue;
    if (/\|\s*(章节|Chapter)\s*\|/i.test(line)) {
      const cols = line.split("|").map((c) => c.trim());
      const idx = cols.findIndex((c) => /^(伏笔动态|hookActivity)$/i.test(c));
      return idx >= 0 ? idx : DEFAULT_INDEX;
    }
  }
  return DEFAULT_INDEX;
}

function extractColumn(row: string, index: number): string | null {
  const cols = row.split("|");
  if (index >= 0 && index < cols.length) {
    return cols[index]!.trim();
  }
  return null;
}

/**
 * 轻量推进升级 pass：读 hooks + chapter_summaries，检查 advancedCount >= 2，
 * 翻转 promoted 标志。无 LLM 调用。
 */
export function rerunPromotionPass(
  hooks: ReadonlyArray<StoredHook>,
  summariesRaw: string,
): PromotionPassResult {
  if (hooks.length === 0) {
    return { updated: false, hooks, flippedCount: 0 };
  }

  const derivedCounts = deriveAdvancedCountsFromSummaries(
    summariesRaw,
    hooks.map((h) => h.hookId),
  );

  let flipped = 0;
  const nextHooks: StoredHook[] = hooks.map((hook) => {
    if (hook.promoted === true) return hook;
    const advanced = hook.advancedCount ?? derivedCounts.get(hook.hookId) ?? 0;
    if (advanced >= 2) {
      flipped += 1;
      return { ...hook, promoted: true };
    }
    return hook;
  });

  return {
    updated: flipped > 0,
    hooks: nextHooks,
    flippedCount: flipped,
  };
}
