// GenreProfile —— 题材档案类型与解析。
//
// js-yaml 在 Mnemosyne 未安装且用户禁用 npm，改为自写最小 frontmatter 解析器。
//
// 支持的 YAML 子集（覆盖全部题材档案文件的实际语法）：
//   key: value          → string
//   key: "value"        → string（去引号）
//   key: true | false   → boolean
//   key: 123            → number
//   key: [a, b, c]      → string[]（逗号分隔，自动去引号）
//   key: [1, 2, 3]      → number[]
//
// 不支持：嵌套对象、多行数组、锚点/引用、字符串续行。
// 解析失败 fail-loud（throw），不静默 fallback —— 题材档案是建书流程的硬依赖，
// 静默降级会让 architect 拿到错误的 genre 配置而难以排查。

import { z } from "zod";

export const GenreProfileSchema = z.object({
  name: z.string(),
  id: z.string(),
  language: z.enum(["zh", "en"]).default("zh"),
  chapterTypes: z.array(z.string()),
  fatigueWords: z.array(z.string()),
  numericalSystem: z.boolean().default(false),
  powerScaling: z.boolean().default(false),
  eraResearch: z.boolean().default(false),
  pacingRule: z.string().default(""),
  satisfactionTypes: z.array(z.string()).default([]),
  auditDimensions: z.array(z.number()).default([]),
});

export type GenreProfile = z.infer<typeof GenreProfileSchema>;

export interface ParsedGenreProfile {
  readonly profile: GenreProfile;
  readonly body: string;
}

export function parseGenreProfile(raw: string): ParsedGenreProfile {
  const fmMatch = raw.match(/^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/);
  if (!fmMatch) {
    throw new Error("Genre profile missing YAML frontmatter (--- ... ---)");
  }

  const frontmatter = parseMinimalYamlFrontmatter(fmMatch[1]!);
  const profile = GenreProfileSchema.parse(frontmatter);
  const body = fmMatch[2]!.trim();

  return { profile, body };
}

function parseMinimalYamlFrontmatter(raw: string): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  const lines = raw.split(/\r?\n/);

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const colonIdx = trimmed.indexOf(":");
    if (colonIdx < 0) {
      throw new Error(`Invalid YAML line (no key:value): ${line}`);
    }

    const key = trimmed.slice(0, colonIdx).trim();
    const rawValue = trimmed.slice(colonIdx + 1).trim();
    if (!key) {
      throw new Error(`Invalid YAML line (empty key): ${line}`);
    }

    result[key] = parseScalarValue(rawValue, line);
  }

  return result;
}

function parseScalarValue(rawValue: string, line: string): unknown {
  if (rawValue.startsWith("[") && rawValue.endsWith("]")) {
    const inner = rawValue.slice(1, -1).trim();
    if (!inner) return [];
    return inner.split(",").map((item) => {
      const s = item.trim();
      return parseScalarToken(s);
    });
  }

  return parseScalarToken(rawValue, line);
}

function parseScalarToken(token: string, line?: string): unknown {
  const trimmed = token.trim();
  if (trimmed === "") return "";

  if (trimmed === "true") return true;
  if (trimmed === "false") return false;

  if (/^-?\d+$/.test(trimmed)) return parseInt(trimmed, 10);
  if (/^-?\d+\.\d+$/.test(trimmed)) return parseFloat(trimmed);

  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }

  if ((trimmed.startsWith('"') && !trimmed.endsWith('"')) || (trimmed.startsWith("'") && !trimmed.endsWith("'"))) {
    throw new Error(`Unclosed quote in YAML line: ${line ?? trimmed}`);
  }

  return trimmed;
}