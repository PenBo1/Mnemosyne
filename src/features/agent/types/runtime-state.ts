// Runtime State zod schemas —— 纯 zod 定义。
//
// 这些 schema 定义了 pipeline 运行时状态的 4 个 JSON 文件结构：
// - current_state.json：当前状态事实（subject-predicate-object 三元组）
// - pending_hooks.json：待处理的伏笔/hook 记录
// - chapter_summaries.json：章节摘要表
// - manifest.json：状态清单（schemaVersion + lastAppliedChapter）

import { z } from "zod";

export const RuntimeStateLanguageSchema = z.enum(["zh", "en"]);
export type RuntimeStateLanguage = z.infer<typeof RuntimeStateLanguageSchema>;

export const StateManifestSchema = z.object({
  schemaVersion: z.literal(2),
  language: RuntimeStateLanguageSchema,
  lastAppliedChapter: z.number().int().min(0),
  projectionVersion: z.number().int().min(1),
  migrationWarnings: z.array(z.string()).default([]),
});

export type StateManifest = z.infer<typeof StateManifestSchema>;

export const HookStatusSchema = z.enum(["open", "progressing", "deferred", "resolved"]);
export type HookStatus = z.infer<typeof HookStatusSchema>;

export const HookPayoffTimingSchema = z.enum([
  "immediate",
  "near-term",
  "mid-arc",
  "slow-burn",
  "endgame",
]);
export type HookPayoffTiming = z.infer<typeof HookPayoffTimingSchema>;

export const HookRecordSchema = z.object({
  hookId: z.string().min(1),
  startChapter: z.number().int().min(0),
  type: z.string().min(1),
  status: HookStatusSchema,
  lastAdvancedChapter: z.number().int().min(0),
  expectedPayoff: z.string().default(""),
  payoffTiming: HookPayoffTimingSchema.optional(),
  notes: z.string().default(""),
  dependsOn: z.array(z.string().min(1)).optional(),
  paysOffInArc: z.string().optional(),
  coreHook: z.boolean().optional(),
  halfLifeChapters: z.number().int().positive().optional(),
  advancedCount: z.number().int().min(0).optional(),
  promoted: z.boolean().optional(),
});

export type HookRecord = z.infer<typeof HookRecordSchema>;

export const HooksStateSchema = z.object({
  hooks: z.array(HookRecordSchema).default([]),
});

export type HooksState = z.infer<typeof HooksStateSchema>;

export const ChapterSummaryRowSchema = z.object({
  chapter: z.number().int().min(1),
  title: z.string().min(1),
  characters: z.string().default(""),
  events: z.string().default(""),
  stateChanges: z.string().default(""),
  hookActivity: z.string().default(""),
  mood: z.string().default(""),
  chapterType: z.string().default(""),
});

export type ChapterSummaryRow = z.infer<typeof ChapterSummaryRowSchema>;

export const ChapterSummariesStateSchema = z.object({
  rows: z.array(ChapterSummaryRowSchema).default([]),
});

export type ChapterSummariesState = z.infer<typeof ChapterSummariesStateSchema>;

export const CurrentStateFactSchema = z.object({
  subject: z.string().min(1),
  predicate: z.string().min(1),
  object: z.string().min(1),
  validFromChapter: z.number().int().min(0),
  validUntilChapter: z.number().int().min(0).nullable(),
  sourceChapter: z.number().int().min(0),
});

export type CurrentStateFact = z.infer<typeof CurrentStateFactSchema>;

export const CurrentStateStateSchema = z.object({
  chapter: z.number().int().min(0),
  facts: z.array(CurrentStateFactSchema).default([]),
});

export type CurrentStateState = z.infer<typeof CurrentStateStateSchema>;

export const CurrentStatePatchSchema = z.object({
  currentLocation: z.string().optional(),
  protagonistState: z.string().optional(),
  currentGoal: z.string().optional(),
  currentConstraint: z.string().optional(),
  currentAlliances: z.string().optional(),
  currentConflict: z.string().optional(),
});

export type CurrentStatePatch = z.infer<typeof CurrentStatePatchSchema>;

export const HookOpsSchema = z.object({
  upsert: z.array(HookRecordSchema).default([]),
  mention: z.array(z.string().min(1)).default([]),
  resolve: z.array(z.string().min(1)).default([]),
  defer: z.array(z.string().min(1)).default([]),
});

export type HookOps = z.infer<typeof HookOpsSchema>;

export const NewHookCandidateSchema = z.object({
  type: z.string().min(1),
  expectedPayoff: z.string().default(""),
  payoffTiming: HookPayoffTimingSchema.optional(),
  notes: z.string().default(""),
});

export type NewHookCandidate = z.infer<typeof NewHookCandidateSchema>;

const LooseOpSchema = z.record(z.string(), z.unknown());

export const RuntimeStateDeltaSchema = z.object({
  chapter: z.number().int().min(1),
  currentStatePatch: CurrentStatePatchSchema.optional(),
  hookOps: HookOpsSchema.default({
    upsert: [],
    mention: [],
    resolve: [],
    defer: [],
  }),
  newHookCandidates: z.array(NewHookCandidateSchema).default([]),
  chapterSummary: ChapterSummaryRowSchema.optional(),
  subplotOps: z.array(LooseOpSchema).default([]),
  emotionalArcOps: z.array(LooseOpSchema).default([]),
  characterMatrixOps: z.array(LooseOpSchema).default([]),
  notes: z.array(z.string()).default([]),
});

export type RuntimeStateDelta = z.infer<typeof RuntimeStateDeltaSchema>;