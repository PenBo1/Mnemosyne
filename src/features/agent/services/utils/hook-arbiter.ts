// Hook 仲裁器。
//
// 迁移要点（import 路径调整）：
// - `../models/runtime-state.js`（RuntimeStateDeltaSchema + 类型）→ `@/shared/types/runtime-state`
// - `./story-markdown.js`（normalizeHookId）→ `./story-markdown`（已迁移到同目录）
// - `./hook-governance.js`（evaluateHookAdmission）→ `./hook-governance`（已迁移到同目录）
//
// timing 推导已恢复（P2.7）：mergeCandidateIntoExistingHook / createCanonicalHook
// 调用 resolveHookPayoffTiming(...) 按 5 级优先级推导 timing enum
// （显式 payoffTiming → expectedPayoff 关键词 → notes 关键词 → hook.type 默认 → 章节跨度）。
//
// 业务逻辑零改动（arbitrateRuntimeStateDeltaHooks：处理 upsert/mention/resolve/defer/
// newHookCandidates，admission 检测重复 family，创建规范 hook id 全部保留）。

import {
  RuntimeStateDeltaSchema,
  type HookRecord,
  type NewHookCandidate,
  type RuntimeStateDelta,
} from "@/types/runtime-state";
import { normalizeHookId } from "./story-markdown";
import { evaluateHookAdmission } from "./hook-governance";
import { resolveHookPayoffTiming } from "./hook-lifecycle";

export interface HookArbiterDecision {
  readonly action: "created" | "mapped" | "mentioned" | "rejected";
  readonly reason: string;
  readonly hookId?: string;
  readonly candidate: NewHookCandidate;
}

interface PendingHookCandidate extends NewHookCandidate {
  readonly preferredHookId?: string;
}

export function arbitrateRuntimeStateDeltaHooks(params: {
  readonly hooks: ReadonlyArray<HookRecord>;
  readonly delta: RuntimeStateDelta;
}): {
  readonly resolvedDelta: RuntimeStateDelta;
  readonly decisions: ReadonlyArray<HookArbiterDecision>;
} {
  const delta = RuntimeStateDeltaSchema.parse(params.delta);
  const workingHooks = params.hooks.map((hook) => ({ ...hook }));
  const knownHookIds = new Set(workingHooks.map((hook) => hook.hookId));
  const upsertsById = new Map<string, HookRecord>();
  const mentions = new Set(delta.hookOps.mention);
  const resolves = uniqueStrings(delta.hookOps.resolve);
  const defers = uniqueStrings(delta.hookOps.defer);
  const fallbackCandidates: PendingHookCandidate[] = [];
  const decisions: HookArbiterDecision[] = [];

  for (const hook of delta.hookOps.upsert) {
    if (knownHookIds.has(hook.hookId)) {
      const normalized = { ...hook };
      upsertsById.set(normalized.hookId, normalized);
      replaceWorkingHook(workingHooks, normalized);
      continue;
    }

    fallbackCandidates.push({
      type: hook.type,
      expectedPayoff: hook.expectedPayoff,
      notes: hook.notes,
      preferredHookId: hook.hookId,
    });
  }

  for (const candidate of [...fallbackCandidates, ...delta.newHookCandidates]) {
    const activeHooks = workingHooks.filter((hook) => hook.status !== "resolved");
    const admission = evaluateHookAdmission({
      candidate,
      activeHooks,
    });

    if (!admission.admit) {
      if (admission.reason === "duplicate_family" && admission.matchedHookId) {
        const matched = workingHooks.find((hook) => hook.hookId === admission.matchedHookId);
        if (!matched) {
          decisions.push({
            action: "rejected",
            reason: "duplicate_family_without_match",
            candidate,
          });
          continue;
        }

        if (isPureRestatement(candidate, matched)) {
          if (!upsertsById.has(matched.hookId) && !resolves.includes(matched.hookId) && !defers.includes(matched.hookId)) {
            mentions.add(matched.hookId);
          }
          decisions.push({
            action: "mentioned",
            reason: "restated_existing_family",
            hookId: matched.hookId,
            candidate,
          });
          continue;
        }

        const base = upsertsById.get(matched.hookId) ?? matched;
        const mapped = mergeCandidateIntoExistingHook(base, candidate, delta.chapter);
        upsertsById.set(mapped.hookId, mapped);
        mentions.delete(mapped.hookId);
        replaceWorkingHook(workingHooks, mapped);
        decisions.push({
          action: "mapped",
          reason: "duplicate_family_with_novelty",
          hookId: matched.hookId,
          candidate,
        });
        continue;
      }

      decisions.push({
        action: "rejected",
        reason: admission.reason,
        candidate,
      });
      continue;
    }

    const created = createCanonicalHook({
      candidate,
      chapter: delta.chapter,
      existingIds: new Set([
        ...workingHooks.map((hook) => hook.hookId),
        ...upsertsById.keys(),
      ]),
    });
    upsertsById.set(created.hookId, created);
    workingHooks.push(created);
    decisions.push({
      action: "created",
      reason: "admit",
      hookId: created.hookId,
      candidate,
    });
  }

  const resolvedDelta = RuntimeStateDeltaSchema.parse({
    ...delta,
    hookOps: {
      upsert: [...upsertsById.values()].sort(sortHooks),
      mention: [...mentions]
        .filter((hookId) => !upsertsById.has(hookId))
        .filter((hookId) => !resolves.includes(hookId))
        .filter((hookId) => !defers.includes(hookId))
        .sort(),
      resolve: resolves,
      defer: defers,
    },
    newHookCandidates: [],
  });

  return {
    resolvedDelta,
    decisions,
  };
}

function mergeCandidateIntoExistingHook(
  existing: HookRecord,
  candidate: NewHookCandidate,
  chapter: number,
): HookRecord {
  const type = preferRicherText(existing.type, candidate.type);
  const expectedPayoff = preferRicherText(existing.expectedPayoff, candidate.expectedPayoff);
  const notes = preferRicherText(existing.notes, candidate.notes);
  return {
    ...existing,
    type,
    status: existing.status === "resolved" ? "resolved" : "progressing",
    lastAdvancedChapter: Math.max(existing.lastAdvancedChapter, chapter),
    expectedPayoff,
    payoffTiming: resolveHookPayoffTiming({
      payoffTiming: candidate.payoffTiming ?? existing.payoffTiming,
      expectedPayoff,
      notes,
      type,
      startChapter: existing.startChapter,
      currentChapter: chapter,
    }),
    notes,
  };
}

function createCanonicalHook(params: {
  readonly candidate: PendingHookCandidate;
  readonly chapter: number;
  readonly existingIds: ReadonlySet<string>;
}): HookRecord {
  const type = params.candidate.type.trim();
  const expectedPayoff = params.candidate.expectedPayoff.trim();
  const notes = params.candidate.notes.trim();
  return {
    hookId: buildCanonicalHookId(params.candidate, params.existingIds),
    startChapter: params.chapter,
    type,
    status: "open",
    lastAdvancedChapter: params.chapter,
    expectedPayoff,
    payoffTiming: resolveHookPayoffTiming({
      payoffTiming: params.candidate.payoffTiming,
      expectedPayoff,
      notes,
      type,
      startChapter: params.chapter,
      currentChapter: params.chapter,
    }),
    notes,
  };
}

function buildCanonicalHookId(
  candidate: PendingHookCandidate,
  existingIds: ReadonlySet<string>,
): string {
  const preferred = normalizeHookId(candidate.preferredHookId);
  if (preferred && !existingIds.has(preferred)) {
    return preferred;
  }

  const base = slugifyHookStem([
    candidate.type,
    candidate.expectedPayoff,
    candidate.notes,
  ].join(" "));
  let next = base;
  let suffix = 2;

  while (existingIds.has(next)) {
    next = `${base}-${suffix}`;
    suffix += 1;
  }

  return next;
}

function slugifyHookStem(value: string): string {
  const normalized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  const englishTerms = (normalized.match(/[a-z0-9]{3,}/g) ?? [])
    .filter((term) => !STOP_WORDS.has(term))
    .slice(0, 5);
  const chineseTerms = (normalized.match(/[\u4e00-\u9fff]{2,6}/g) ?? []).slice(0, 3);
  const stem = [...englishTerms, ...chineseTerms].join("-").slice(0, 64).replace(/-+$/g, "");
  return stem || "hook";
}

function isPureRestatement(candidate: NewHookCandidate, existing: HookRecord): boolean {
  const candidateText = normalizeText([
    candidate.type,
    candidate.expectedPayoff,
    candidate.notes,
  ].join(" "));
  const existingText = normalizeText([
    existing.type,
    existing.expectedPayoff,
    existing.notes,
  ].join(" "));

  if (!candidateText) return true;
  if (candidateText === existingText) return true;

  const candidateTerms = extractTerms(candidateText);
  const existingTerms = extractTerms(existingText);
  const novelTerms = [...candidateTerms].filter((term) => !existingTerms.has(term));

  const candidateChinese = extractChineseBigrams(candidateText);
  const existingChinese = extractChineseBigrams(existingText);
  const novelChinese = [...candidateChinese].filter((term) => !existingChinese.has(term));

  return novelTerms.length === 0 && novelChinese.length < 2;
}

function replaceWorkingHook(workingHooks: HookRecord[], hook: HookRecord): void {
  const index = workingHooks.findIndex((candidate) => candidate.hookId === hook.hookId);
  if (index >= 0) {
    workingHooks[index] = hook;
    return;
  }

  workingHooks.push(hook);
}

function sortHooks(left: HookRecord, right: HookRecord): number {
  return left.startChapter - right.startChapter
    || left.lastAdvancedChapter - right.lastAdvancedChapter
    || left.hookId.localeCompare(right.hookId);
}

function uniqueStrings(values: ReadonlyArray<string>): string[] {
  return [...new Set(values.map((value) => value.trim()).filter(Boolean))];
}

function preferRicherText(primary: string, fallback: string): string {
  const left = primary.trim();
  const right = fallback.trim();

  if (!left) return right;
  if (!right) return left;
  if (left === right) return left;
  return right.length > left.length ? right : left;
}

function normalizeText(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fff]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function extractTerms(value: string): Set<string> {
  const english = value
    .split(" ")
    .map((term) => term.trim())
    .filter((term) => term.length >= 4)
    .filter((term) => !STOP_WORDS.has(term));
  const chinese = value.match(/[\u4e00-\u9fff]{2,6}/g) ?? [];
  return new Set([...english, ...chinese]);
}

function extractChineseBigrams(value: string): Set<string> {
  const segments = value.match(/[\u4e00-\u9fff]+/g) ?? [];
  const terms = new Set<string>();

  for (const segment of segments) {
    if (segment.length < 2) {
      continue;
    }

    for (let index = 0; index <= segment.length - 2; index += 1) {
      terms.add(segment.slice(index, index + 2));
    }
  }

  return terms;
}

const STOP_WORDS = new Set([
  "that",
  "this",
  "with",
  "from",
  "into",
  "still",
  "just",
  "have",
  "will",
  "reveal",
  "about",
  "already",
  "question",
  "chapter",
]);
