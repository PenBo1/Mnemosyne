// Context Assembly —— 纯函数。
//
// RuleStack + ChapterTrace 构造（治理层产物，writer/continuity/reviser prompt 共用）：
//   - buildGovernedRuleStack: 基于 plan.intent.mustAvoid/styleEmphasis 构造 4 层规则栈
//   - buildGovernedTrace: 基于 ContextPackage + composer 输入构造章节追踪（含 token 预算）
//   - isProtectedContextSource: 判定上下文源是否受保护（不计入压缩）
//
// 迁移要点：
// 1. 仅改 import 路径，去掉 .js 后缀。
// 2. 原依赖 `estimateTextTokens`（来自 ../llm/provider.js），Mnemosyne 未迁移该模块。
//    本文件内自写改进版（中文 1 字≈1.5 token / 英文 1 词≈1.3 token / 代码字符数÷3.5）。
//    不引入 tiktoken 外部依赖（保持包体积小）。
// 3. 原依赖 `PlanChapterOutput` 类型（来自 ../agents/planner.js，Mnemosyne 未迁移 planner）。
//    按迁移规则，本文件内定义局部最小接口 `PlanOutputLike`，只包含本文件实际用到的字段
//    （intent.mustAvoid / intent.styleEmphasis / plannerInputs）。后续迁移 planner.ts 时可
//    删除此局部接口并改回 import type。
// 4. 业务逻辑零改动（包括 overrideEdges、layer 顺序、schema.parse 调用均保持原样）。

import type {
  ActiveOverride,
  ChapterIntent,
  ChapterTrace,
  ContextPackage,
  RuleStack,
} from "@/features/agent/types/input-governance";
import {
  ChapterTraceSchema,
  RuleStackSchema,
} from "@/features/agent/types/input-governance";

/**
 * 局部最小 PlanChapterOutput 形状 —— 替代未迁移的 planner.ts 中的同名类型。
 * 仅声明本文件实际用到的字段。后续迁移 planner.ts 时可删除此接口并改回
 * `import type { PlanChapterOutput } from "../agents/planner"`。
 */
interface PlanOutputLike {
  readonly intent: Pick<ChapterIntent, "mustAvoid" | "styleEmphasis">;
  readonly plannerInputs: ReadonlyArray<string>;
}

// ── Token 估算（P2.1 改进版）─────────────────────────────────
//
// 估算依据（不引入 tiktoken，保持包体积小）：
// - 中文（CJK 统一表意文字 + 扩展 A）：1 字 ≈ 1.5 token。
//   依据：GPT-4o 的 cl100k_base 对中文常用字多为 1-2 token，平均约 1.5。
// - 英文：按空格分词，1 词 ≈ 1.3 token。
//   依据：英文常用词在 BPE 中多为 1 token，少见词 2-3 token，平均约 1.3。
// - 代码（非 CJK、非英文单词的符号序列）：字符数 ÷ 3.5。
//   依据：代码中大量符号在 BPE 中平均 2-4 字符/token，取 3.5 折中。
// - 混合文本按三类分别统计后求和，比旧的 length/3 粗估更贴近实际。

const CJK_RANGE = /[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]/g;
const ENGLISH_WORD = /[a-zA-Z]+/g;

/**
 * 改进版 token 估算（P2.1）。
 * 中文 1 字 ≈ 1.5 token / 英文 1 词 ≈ 1.3 token / 代码字符数 ÷ 3.5。
 * 不引入 tiktoken 外部依赖（保持包体积小）。
 */
export function estimateTextTokens(text: string): number {
  if (!text) return 0;

  let cjkChars = 0;
  let englishWords = 0;

  // 统计 CJK 字符数
  const cjkMatches = text.match(CJK_RANGE);
  if (cjkMatches) cjkChars = cjkMatches.length;

  // 统计英文单词数（在移除 CJK 后的文本上分词，避免中文被误匹配）
  const nonCjkText = text.replace(CJK_RANGE, " ");
  const wordMatches = nonCjkText.match(ENGLISH_WORD);
  if (wordMatches) englishWords = wordMatches.length;

  // 代码/符号字符数 = 总字符数 - CJK 字符数 - 英文单词字符数（含连字符）
  const englishChars = englishWords > 0
    ? wordMatches!.reduce((sum, w) => sum + w.length, 0)
    : 0;
  const codeChars = Math.max(0, text.length - cjkChars - englishChars);

  const cjkTokens = cjkChars * 1.5;
  const englishTokens = englishWords * 1.3;
  const codeTokens = codeChars / 3.5;

  return Math.ceil(cjkTokens + englishTokens + codeTokens);
}

const MAX_OVERRIDE_REASON_CHARS = 80;

function truncateForOverrideReason(value: string): string {
  const collapsed = value.replace(/\s+/g, " ").trim();
  return collapsed.length > MAX_OVERRIDE_REASON_CHARS
    ? `${collapsed.slice(0, MAX_OVERRIDE_REASON_CHARS - 1)}…`
    : collapsed;
}

/**
 * Compose the per-chapter rule stack used by writer / continuity / reviser
 * prompts. Source names follow the Phase 5 layout (story_frame, volume_map,
 * roles/) and activeOverrides are derived from the planner's intent so the
 * "Governed Control Stack" block surfaces the actual gating in effect for
 * the current chapter — it used to be a static stub that ignored both
 * `plan` and `chapterNumber`.
 *
 * Phase hotfix 6 (Option A): make this honestly dynamic instead of deleting
 * it, because writer.ts (~L820/L900), continuity.ts (~L590), and
 * reviser.ts (~L600) all render ruleStack.sections / activeOverrides into
 * the model prompt. Removing the function would require a much larger
 * prompt refactor; making it real fixes the lie at the source.
 */
export function buildGovernedRuleStack(plan: PlanOutputLike, chapterNumber: number): RuleStack {
  const activeOverrides: ActiveOverride[] = [];

  // L4 → L3: per-chapter prohibitions narrow the planning layer for this
  // chapter only. mustAvoid items come from rules-reader prohibitions +
  // current_focus avoid section (planner.collectMustAvoid).
  for (const item of plan.intent.mustAvoid) {
    activeOverrides.push({
      from: "L4",
      to: "L3",
      target: `chapter:${chapterNumber}/mustAvoid`,
      reason: truncateForOverrideReason(item),
    });
  }

  // L4 → L3: planner-issued style emphasis is also a per-chapter override
  // on the planning layer. Style emphasis surfaces things like POV tightness
  // or character-conflict focus that the writer must honor this chapter.
  for (const item of plan.intent.styleEmphasis) {
    activeOverrides.push({
      from: "L4",
      to: "L3",
      target: `chapter:${chapterNumber}/styleEmphasis`,
      reason: truncateForOverrideReason(item),
    });
  }

  return RuleStackSchema.parse({
    layers: [
      { id: "L1", name: "hard_facts", precedence: 100, scope: "global" },
      { id: "L2", name: "author_intent", precedence: 80, scope: "book" },
      { id: "L3", name: "planning", precedence: 60, scope: "arc" },
      { id: "L4", name: "current_task", precedence: 70, scope: "local" },
    ],
    sections: {
      // Phase 5 authoritative source names (was: story_bible, volume_outline).
      hard: ["story_frame", "current_state", "book_rules", "roles"],
      soft: ["author_intent", "current_focus", "volume_map"],
      diagnostic: ["anti_ai_checks", "continuity_audit", "style_regression_checks"],
    },
    overrideEdges: [
      { from: "L4", to: "L3", allowed: true, scope: "current_chapter" },
      { from: "L4", to: "L2", allowed: false, scope: "current_chapter" },
      { from: "L4", to: "L1", allowed: false, scope: "current_chapter" },
    ],
    activeOverrides,
  });
}

export function buildGovernedTrace(params: {
  readonly chapterNumber: number;
  readonly plan: PlanOutputLike;
  readonly contextPackage: ContextPackage;
  readonly composerInputs: ReadonlyArray<string>;
  readonly notes?: ReadonlyArray<string>;
}): ChapterTrace {
  const protectedEntries = params.contextPackage.selectedContext.filter((entry) =>
    isProtectedContextSource(entry.source),
  );
  const compressibleEntries = params.contextPackage.selectedContext.filter((entry) =>
    !isProtectedContextSource(entry.source),
  );
  const protectedTokens = sumContextTokens(protectedEntries);
  const compressibleTokens = sumContextTokens(compressibleEntries);

  return ChapterTraceSchema.parse({
    chapter: params.chapterNumber,
    plannerInputs: params.plan.plannerInputs,
    composerInputs: params.composerInputs,
    selectedSources: params.contextPackage.selectedContext.map((entry) => entry.source),
    contextTiers: {
      protectedSources: protectedEntries.map((entry) => entry.source),
      compressibleSources: compressibleEntries.map((entry) => entry.source),
    },
    tokenBudget: {
      protectedTokens,
      compressibleTokens,
      totalSelectedTokens: protectedTokens + compressibleTokens,
    },
    notes: params.notes ?? [],
  });
}

export function isProtectedContextSource(source: string): boolean {
  return source === "runtime/chapter_memo"
    || source === "story/current_focus.md"
    || source === "story/author_intent.md"
    || source === "story/audit_drift.md"
    || source === "story/outline/story_frame.md"
    || source.startsWith("story/outline/story_frame.md#")
    || source === "story/story_bible.md"
    || source === "story/outline/volume_map.md"
    || source.startsWith("story/outline/volume_map.md#")
    || source === "story/volume_outline.md"
    || source === "story/parent_canon.md"
    || source === "story/fanfic_canon.md"
    || source.startsWith("story/current_state.md")
    || source.startsWith("story/pending_hooks.md#")
    || source.startsWith("runtime/hook_debt#");
}

function sumContextTokens(entries: ReadonlyArray<ContextPackage["selectedContext"][number]>): number {
  return entries.reduce((total, entry) => total + estimateContextSourceTokens(entry), 0);
}

function estimateContextSourceTokens(entry: ContextPackage["selectedContext"][number]): number {
  return estimateTextTokens([entry.source, entry.reason, entry.excerpt].filter(Boolean).join("\n"));
}
