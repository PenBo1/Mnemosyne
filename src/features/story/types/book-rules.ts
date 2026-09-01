// BookRules —— 书级规则解析。
//
// 去除 js-yaml 依赖：
// - 只保留普通 Markdown 解析路径（parseMarkdownBookRules）
// - 删除 YAML frontmatter fallback（Mnemosyne 是新项目，无 Phase 5 之前的
//   legacy frontmatter 数据；architect 输出的就是普通 Markdown book_rules）
// - parseBookRules 仍识别 shim（兼容指针）并返回 null，让 caller 走 fallback
//
// 解析规则：从 Markdown 章节里提取主角 / 题材锁 / 数值规则 / 年代限制 / 禁忌 /
// 同人模式等结构化字段，原文作为 body 保留。

import { z } from "zod";

const ProtagonistSchema = z.object({
  name: z.string(),
  personalityLock: z.array(z.string()).default([]),
  behavioralConstraints: z.array(z.string()).default([]),
}).optional();

const GenreLockSchema = z.object({
  primary: z.string(),
  forbidden: z.array(z.string()).default([]),
}).optional();

const NumericalOverridesSchema = z.object({
  hardCap: z.union([z.number(), z.string()]).optional(),
  resourceTypes: z.array(z.string()).default([]),
}).optional();

const EraConstraintsSchema = z.object({
  enabled: z.boolean().default(false),
  period: z.string().optional(),
  region: z.string().optional(),
}).optional();

export const BookRulesSchema = z.object({
  version: z.string().default("1.0"),
  protagonist: ProtagonistSchema,
  genreLock: GenreLockSchema,
  narrativePerson: z.enum(["first", "third"]).optional().catch(undefined),
  numericalSystemOverrides: NumericalOverridesSchema,
  eraConstraints: EraConstraintsSchema,
  prohibitions: z.array(z.string()).default([]),
  chapterTypesOverride: z.array(z.string()).default([]),
  fatigueWordsOverride: z.array(z.string()).default([]),
  additionalAuditDimensions: z.array(z.union([z.number(), z.string()])).default([]),
  enableFullCastTracking: z.boolean().default(false),
  fanficMode: z.enum(["canon", "au", "ooc", "cp"]).optional(),
  allowedDeviations: z.array(z.string()).default([]),
});

export type BookRules = z.infer<typeof BookRulesSchema>;

export interface ParsedBookRules {
  readonly rules: BookRules;
  readonly body: string;
}

export function isBookRulesShim(raw: string): boolean {
  return (
    /本书规则（兼容指针——已废弃）/.test(raw)
    || /Book Rules \(compat pointer — deprecated\)/.test(raw)
    || /本文件仅为外部读取保留/.test(raw)
    || /This file is kept for external readers only/.test(raw)
  );
}

export function parseBookRules(raw: string): ParsedBookRules | null {
  const stripped = raw.replace(/^```(?:md|markdown|yaml)?\s*\n/, "").replace(/\n```\s*$/, "");

  if (isBookRulesShim(stripped)) {
    return null;
  }

  const rules = parseMarkdownBookRules(stripped);
  return { rules, body: stripped.trim() };
}

function parseMarkdownBookRules(raw: string): BookRules {
  const protagonistSection = extractMarkdownSection(raw, ["主角", "Protagonist"]);
  const protagonistName =
    readLabeledValue(protagonistSection, ["名字", "姓名", "name", "protagonist"])
    ?? readLabeledValue(raw, ["主角", "protagonist"]);
  const personalityLock = readLabeledList(protagonistSection, [
    "性格锁",
    "性格关键词",
    "personalityLock",
    "personality lock",
    "core tags",
  ]);
  const behavioralConstraints = readLabeledList(protagonistSection, [
    "行为约束",
    "behavioralConstraints",
    "behavioral constraints",
  ]);

  const genreSection = extractMarkdownSection(raw, ["题材锁", "Genre Lock", "Genre"]);
  const primary = readLabeledValue(genreSection, ["主类型", "题材", "primary", "genre"]);
  const forbidden = [
    ...readLabeledList(genreSection, ["禁止混入", "禁混", "forbidden"]),
    ...readMarkdownList(extractMarkdownSection(raw, ["禁止混入", "Forbidden Style Intrusions", "Forbidden"])),
  ];

  const prohibitions = readMarkdownList(extractMarkdownSection(raw, [
    "禁止事项",
    "禁忌",
    "本书禁忌",
    "Prohibitions",
    "Do Not",
  ]));
  const fanficSection = extractMarkdownSection(raw, ["同人模式", "Fanfic Mode", "Fanfic"]);
  const fanficMode = normalizeFanficMode(readLabeledValue(fanficSection, [
    "模式",
    "同人模式",
    "fanficMode",
    "fanfic mode",
    "mode",
  ]));
  const allowedDeviations = readLabeledList(fanficSection, [
    "允许偏离",
    "允许的偏离",
    "allowedDeviations",
    "allowed deviations",
  ]);

  const numericalSection = extractMarkdownSection(raw, [
    "数值/资源规则",
    "数值规则",
    "资源规则",
    "Numerical / Resource Rules",
    "Numerical Rules",
    "Resource Rules",
  ]);
  const resourceTypes = readLabeledList(numericalSection, [
    "核心资源",
    "资源类型",
    "resourceTypes",
    "core resources",
    "resources",
  ]);
  const hardCap = readLabeledValue(numericalSection, ["硬上限", "hardCap", "hard cap"]);

  const eraSection = extractMarkdownSection(raw, ["年代限制", "时代限制", "Era Constraints"]);
  const period = readLabeledValue(eraSection, ["时期", "年代", "period", "era"]);
  const region = readLabeledValue(eraSection, ["地域", "地区", "region"]);

  return BookRulesSchema.parse({
    protagonist: protagonistName
      ? {
          name: protagonistName,
          personalityLock,
          behavioralConstraints,
        }
      : undefined,
    genreLock: primary || forbidden.length > 0
      ? {
          primary: primary ?? "",
          forbidden,
        }
      : undefined,
    narrativePerson: detectNarrativePerson(raw),
    numericalSystemOverrides: hardCap || resourceTypes.length > 0
      ? {
          hardCap,
          resourceTypes,
        }
      : undefined,
    eraConstraints: eraSection
      ? {
          enabled: true,
          period,
          region,
        }
      : undefined,
    prohibitions,
    fanficMode,
    allowedDeviations,
  });
}

function extractMarkdownSection(raw: string, headings: ReadonlyArray<string>): string {
  const wanted = new Set(headings.map(normalizeHeading));
  const lines = raw.split(/\r?\n/);
  let collecting = false;
  const out: string[] = [];

  for (const line of lines) {
    const heading = line.match(/^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$/)?.[1];
    if (heading) {
      if (collecting) break;
      collecting = wanted.has(normalizeHeading(heading));
      continue;
    }
    if (collecting) out.push(line);
  }

  return out.join("\n").trim();
}

function readLabeledValue(raw: string, labels: ReadonlyArray<string>): string | undefined {
  if (!raw.trim()) return undefined;
  const labelPattern = labels.map(escapeRegExp).join("|");
  const match = raw.match(new RegExp(`^\\s*(?:[-*]\\s*)?(?:${labelPattern})\\s*[:：]\\s*(.+?)\\s*$`, "im"));
  const value = cleanScalar(match?.[1] ?? "");
  return value || undefined;
}

function readLabeledList(raw: string, labels: ReadonlyArray<string>): string[] {
  const value = readLabeledValue(raw, labels);
  return value ? splitList(value) : [];
}

function readMarkdownList(raw: string): string[] {
  if (!raw.trim()) return [];
  return raw
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => /^[-*]\s+/.test(line))
    .map((line) => cleanScalar(line.replace(/^[-*]\s+/, "")))
    .filter((value) => value.length > 0);
}

function splitList(value: string): string[] {
  const stripped = cleanScalar(value).replace(/^[\[(（【]\s*/, "").replace(/\s*[\])）】]$/, "");
  return stripped
    .split(/[、,，;；|]/)
    .map(cleanScalar)
    .filter((item) => item.length > 0);
}

function detectNarrativePerson(raw: string): "first" | "third" | undefined {
  if (/第一人称|first[-\s]?person|\bfirst\b/i.test(raw)) return "first";
  if (/第三人称|third[-\s]?person|\bthird\b/i.test(raw)) return "third";
  return undefined;
}

function normalizeFanficMode(value: string | undefined): "canon" | "au" | "ooc" | "cp" | undefined {
  if (!value) return undefined;
  const normalized = value.trim().toLowerCase();
  if (normalized === "canon" || /正典|原作空白|原作视角/.test(value)) return "canon";
  if (normalized === "au" || /平行|分歧|if线/i.test(value)) return "au";
  if (normalized === "ooc" || /性格偏离/i.test(value)) return "ooc";
  if (normalized === "cp" || /配对|感情线/i.test(value)) return "cp";
  return undefined;
}

function cleanScalar(value: string): string {
  const trimmed = value
    .trim()
    .replace(/^["'`"''']+|["'`"''']+$/g, "")
    .trim();
  return /^(?:无|none|n\/a|na|\(none\)|（无）|-|—)$/i.test(trimmed) ? "" : trimmed;
}

function normalizeHeading(value: string): string {
  return value.replace(/[：:]\s*$/, "").trim().toLowerCase();
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}