// Skill 类型定义 —— 与 Rust application/skill/types.rs 对齐。
//
// 类型映射:
// - SkillMetadata ↔ SkillMetadataDto(Rust)
// - SkillPhase ↔ SkillPhaseDto(Rust)
// - SkillDefinition ↔ SkillDefinitionDto(Rust)
// - SkillExecutionResult ↔ SkillExecutionResultDto(Rust)
//
// 所有字段使用 camelCase，与 Rust 侧 serde(rename_all = "camelCase") 对齐。

export type SkillOutputFormat = "markdown" | "json" | "text";

export type EffortLevel = "low" | "medium" | "high" | "xhigh" | "max";

export interface SkillMetadata {
  name: string;
  description: string;
  whenToUse: string[];
  version?: string;
  author?: string;
  tags?: string[];
}

export interface SkillPhase {
  name: string;
  instructions: string;
  type?: "discover" | "deliver" | "verify" | "persist" | "schedule";
  timeoutMs?: number;
  retryCount?: number;
}

export interface SkillDefinition {
  metadata: SkillMetadata;
  phases: SkillPhase[];
  outputFormat: SkillOutputFormat;
  outputPath?: string;
  parameters?: Record<string, SkillParameterDef>;
}

export interface SkillParameterDef {
  type: "string" | "number" | "boolean" | "array" | "object";
  description: string;
  required: boolean;
  default?: unknown;
  enum?: string[];
  min?: number;
  max?: number;
}

export interface SkillExecutionContext {
  sessionId: string;
  workspaceId?: string;
  userId?: string;
  role?: string;
  parameters: Record<string, unknown>;
  effort?: EffortLevel;
}

export interface SkillExecutionResult {
  skillName: string;
  status: "success" | "partial" | "failed";
  output: string;
  phases: PhaseExecutionResult[];
  durationMs: number;
  tokensUsed?: number;
  errorMessage?: string;
}

export interface PhaseExecutionResult {
  phaseName: string;
  status: "success" | "skipped" | "failed";
  output: string;
  durationMs: number;
  tokensUsed?: number;
}

export interface SkillYamlFrontmatter {
  name: string;
  description: string;
  when_to_use: string[];
  output_format: SkillOutputFormat;
  version?: string;
  author?: string;
  tags?: string[];
  parameters?: Record<string, SkillParameterDef>;
}

export const DEFAULT_SKILL_METADATA: Partial<SkillMetadata> = {
  version: "1.0.0",
  tags: [],
};

export const DEFAULT_SKILL_PHASE: Partial<SkillPhase> = {
  type: "deliver",
  timeoutMs: 60000,
  retryCount: 0,
};

export const BUILTIN_SKILL_NAMES = [
  "loop",
  "code-review",
  "deep-research",
] as const;

export type BuiltinSkillName = (typeof BUILTIN_SKILL_NAMES)[number];

export function isBuiltinSkillName(name: string): name is BuiltinSkillName {
  return BUILTIN_SKILL_NAMES.includes(name as BuiltinSkillName);
}