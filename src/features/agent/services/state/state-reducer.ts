// 运行时状态 Reducer。
//
// 迁移要点（import 路径调整）：
// - `../models/runtime-state.js`（多个 schema + 类型）→ `@/shared/types/runtime-state`
// - `../utils/hook-governance.js`（evaluateHookAdmission）→ `../utils/hook-governance`（已迁移）
// - `./state-validator.js`（validateRuntimeState）→ `./state-validator`（已迁移到同目录）
//
// 简化适配（hook-lifecycle.ts 是简化版）：
//   原版 mergeHookRecord 调用 resolveHookPayoffTiming({ payoffTiming, expectedPayoff, notes })
//   推导 timing enum。简化版 resolveHookPayoffTiming 仅 trim 原始字符串返回 string | undefined，
//   与 HookRecord.payoffTiming 字段类型（HookPayoffTiming | undefined）不兼容。
//   适配为直接使用原始 payoffTiming 值（incoming.payoffTiming ?? existing.payoffTiming）：
//     - 该值已由 zod schema 校验为 HookPayoffTiming enum，类型为 HookPayoffTiming | undefined，
//       与字段类型兼容；
//     - 简化版 resolveHookPayoffTiming 的 trim 对 enum 值是 no-op（enum 字面量无空白），等价；
//     - 缺失 payoffTiming 时原版会从 expectedPayoff/notes 推断，简化版退回 undefined（保守下界）。
//   移除 hook-lifecycle import（不再使用）。
//
// TODO: P2 阶段 7 三层记忆接入时恢复 resolveHookPayoffTiming timing 推导。
//
// 业务逻辑零改动（hookOps upsert/resolve/defer + 同族去重合并 + currentStatePatch
// 别名替换 + chapterSummary 增量，经 schema + state-validator 双重校验全部保留）。

import {
  ChapterSummariesStateSchema,
  CurrentStateStateSchema,
  HooksStateSchema,
  RuntimeStateDeltaSchema,
  StateManifestSchema,
  type HookRecord,
  type ChapterSummariesState,
  type CurrentStateState,
  type HooksState,
  type RuntimeStateDelta,
  type StateManifest,
} from "@/types/runtime-state";
import { evaluateHookAdmission } from "../utils/hook-governance";
import { validateRuntimeState } from "./state-validator";

export interface RuntimeStateSnapshot {
  readonly manifest: StateManifest;
  readonly currentState: CurrentStateState;
  readonly hooks: HooksState;
  readonly chapterSummaries: ChapterSummariesState;
}

export function applyRuntimeStateDelta(params: {
  readonly snapshot: RuntimeStateSnapshot;
  readonly delta: RuntimeStateDelta;
  readonly allowReapply?: boolean;
}): RuntimeStateSnapshot {
  const snapshot = {
    manifest: StateManifestSchema.parse(params.snapshot.manifest),
    currentState: CurrentStateStateSchema.parse(params.snapshot.currentState),
    hooks: HooksStateSchema.parse(params.snapshot.hooks),
    chapterSummaries: ChapterSummariesStateSchema.parse(params.snapshot.chapterSummaries),
  };
  const delta = RuntimeStateDeltaSchema.parse(params.delta);
  const allowReapply = params.allowReapply ?? false;

  if (allowReapply ? delta.chapter < snapshot.manifest.lastAppliedChapter : delta.chapter <= snapshot.manifest.lastAppliedChapter) {
    throw new Error(`delta chapter ${delta.chapter} goes backwards`);
  }

  if (delta.chapterSummary && delta.chapterSummary.chapter !== delta.chapter) {
    throw new Error(`chapter summary ${delta.chapterSummary.chapter} does not match delta chapter ${delta.chapter}`);
  }

  if (
    delta.chapterSummary
    && snapshot.chapterSummaries.rows.some((row) => row.chapter === delta.chapterSummary?.chapter)
    && !allowReapply
  ) {
    throw new Error(`duplicate summary row for chapter ${delta.chapterSummary.chapter}`);
  }

  const hooks = applyHookOps(snapshot.hooks, delta);
  const currentState = applyCurrentStatePatch(
    snapshot.currentState,
    snapshot.manifest.language,
    delta,
  );
  const chapterSummaries = applySummaryDelta(snapshot.chapterSummaries, delta, allowReapply);

  const next: RuntimeStateSnapshot = {
    manifest: {
      ...snapshot.manifest,
      lastAppliedChapter: delta.chapter,
    },
    currentState,
    hooks,
    chapterSummaries,
  };

  const issues = validateRuntimeState(next);
  if (issues.length > 0) {
    throw new Error(issues.map((issue) => `${issue.code}: ${issue.message}`).join("; "));
  }

  return next;
}

function applyHookOps(hooksState: HooksState, delta: RuntimeStateDelta): HooksState {
  const hooksById = new Map(hooksState.hooks.map((hook) => [hook.hookId, { ...hook }]));

  for (const hook of delta.hookOps.upsert) {
    const sameHook = hooksById.get(hook.hookId);
    if (sameHook) {
      hooksById.set(sameHook.hookId, mergeHookRecord(sameHook, hook));
      continue;
    }

    const admission = evaluateHookAdmission({
      candidate: {
        type: hook.type,
        expectedPayoff: hook.expectedPayoff,
        notes: hook.notes,
      },
      activeHooks: [...hooksById.values()].filter((candidate) => candidate.status !== "resolved"),
    });

    if (!admission.admit && admission.reason === "duplicate_family") {
      const matchedHookId = admission.matchedHookId;
      const existing = matchedHookId ? hooksById.get(matchedHookId) : undefined;
      if (!existing) {
        throw new Error(`duplicate active hook family: ${hook.hookId} overlaps ${admission.matchedHookId}`);
      }
      hooksById.set(existing.hookId, mergeDuplicateHookFamily(existing, hook));
      continue;
    }

    hooksById.set(hook.hookId, { ...hook });
  }

  for (const hookId of delta.hookOps.resolve) {
    const existing = hooksById.get(hookId);
    if (!existing) {
      // Hook may have been cleared by a previous settlement or not yet created — skip gracefully
      continue;
    }
    hooksById.set(hookId, {
      ...existing,
      status: "resolved",
      lastAdvancedChapter: Math.max(existing.lastAdvancedChapter, delta.chapter),
    });
  }

  for (const hookId of delta.hookOps.defer) {
    const existing = hooksById.get(hookId);
    if (!existing) {
      continue;
    }
    hooksById.set(hookId, {
      ...existing,
      status: "deferred",
      lastAdvancedChapter: Math.max(existing.lastAdvancedChapter, delta.chapter),
    });
  }

  return {
    hooks: [...hooksById.values()].sort((left, right) => (
      left.startChapter - right.startChapter
      || left.lastAdvancedChapter - right.lastAdvancedChapter
      || left.hookId.localeCompare(right.hookId)
    )),
  };
}

function mergeDuplicateHookFamily(existing: HookRecord, incoming: HookRecord): HookRecord {
  return mergeHookRecord(existing, incoming);
}

function mergeHookRecord(existing: HookRecord, incoming: HookRecord): HookRecord {
  const expectedPayoff = preferRicherText(existing.expectedPayoff, incoming.expectedPayoff);
  const notes = preferRicherText(existing.notes, incoming.notes);
  const advanced = Math.max(existing.lastAdvancedChapter, incoming.lastAdvancedChapter);
  const progressed = advanced > existing.lastAdvancedChapter;

  return {
    ...existing,
    startChapter: Math.min(existing.startChapter, incoming.startChapter),
    type: preferRicherText(existing.type, incoming.type),
    status: mergeHookStatus(existing.status, incoming.status, progressed),
    lastAdvancedChapter: advanced,
    expectedPayoff,
    // 简化适配：直接使用原始 payoffTiming 值，不调用 resolveHookPayoffTiming。
    // 详见文件头注释。
    payoffTiming: incoming.payoffTiming ?? existing.payoffTiming,
    notes,
  };
}

function mergeHookStatus(
  existing: HookRecord["status"],
  incoming: HookRecord["status"],
  progressed: boolean,
): HookRecord["status"] {
  if (existing === "resolved" || incoming === "resolved") return "resolved";
  if (progressed || existing === "progressing" || incoming === "progressing") return "progressing";
  return existing;
}

function preferRicherText(primary: string, fallback: string): string {
  const left = primary.trim();
  const right = fallback.trim();

  if (!left) return right;
  if (!right) return left;
  if (left === right) return left;
  return right.length > left.length ? right : left;
}

function applyCurrentStatePatch(
  currentState: CurrentStateState,
  language: "zh" | "en",
  delta: RuntimeStateDelta,
): CurrentStateState {
  if (!delta.currentStatePatch) {
    return {
      chapter: delta.chapter,
      facts: [...currentState.facts],
    };
  }

  const nextFacts = [...currentState.facts];
  const labels = language === "en"
    ? {
      currentLocation: ["Current Location", "当前位置"],
      protagonistState: ["Protagonist State", "主角状态"],
      currentGoal: ["Current Goal", "当前目标"],
      currentConstraint: ["Current Constraint", "当前限制"],
      currentAlliances: ["Current Alliances", "Current Relationships", "当前敌我"],
      currentConflict: ["Current Conflict", "当前冲突"],
    }
    : {
      currentLocation: ["当前位置", "Current Location"],
      protagonistState: ["主角状态", "Protagonist State"],
      currentGoal: ["当前目标", "Current Goal"],
      currentConstraint: ["当前限制", "Current Constraint"],
      currentAlliances: ["当前敌我", "Current Alliances", "Current Relationships"],
      currentConflict: ["当前冲突", "Current Conflict"],
    };

  for (const [patchKey, aliases] of Object.entries(labels) as Array<[
    keyof typeof labels,
    string[],
  ]>) {
    const value = delta.currentStatePatch[patchKey];
    if (value === undefined) continue;

    for (let index = nextFacts.length - 1; index >= 0; index -= 1) {
      const predicate = nextFacts[index]?.predicate ?? "";
      if (aliases.some((alias) => alias.toLowerCase() === predicate.toLowerCase())) {
        nextFacts.splice(index, 1);
      }
    }

    nextFacts.push({
      subject: "protagonist",
      predicate: aliases[0]!,
      object: value,
      validFromChapter: delta.chapter,
      validUntilChapter: null,
      sourceChapter: delta.chapter,
    });
  }

  return {
    chapter: delta.chapter,
    facts: nextFacts.sort((left, right) => (
      left.predicate.localeCompare(right.predicate)
      || left.object.localeCompare(right.object)
    )),
  };
}

function applySummaryDelta(
  state: ChapterSummariesState,
  delta: RuntimeStateDelta,
  allowReapply = false,
): ChapterSummariesState {
  if (!delta.chapterSummary) {
    return {
      rows: [...state.rows].sort((left, right) => left.chapter - right.chapter),
    };
  }

  return {
    rows: [
      ...(allowReapply ? state.rows.filter((row) => row.chapter !== delta.chapterSummary!.chapter) : state.rows),
      delta.chapterSummary,
    ].sort((left, right) => left.chapter - right.chapter),
  };
}
