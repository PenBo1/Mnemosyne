// Loop-Engineering 类型定义 —— 与 Rust application/loop_engine/types.rs 对齐。
//
// 类型映射:
// - LoopState ↔ LoopStateDto(Rust)
// - LoopPattern ↔ LoopPatternDto(Rust)
// - LoopRunLog ↔ LoopRunLogDto(Rust)
//
// 所有字段使用 camelCase，与 Rust 侧 serde(rename_all = "camelCase") 对齐。

export type LoopTriggerType = "timer" | "event" | "hybrid";

export type LoopStatus = "idle" | "running" | "paused" | "error";

export type ReadinessLevel = "L0" | "L1" | "L2" | "L3";

export type RiskLevel = "low" | "medium" | "high";

export type Cadence = "manual" | "hourly" | "daily" | "per-chapter";

export type LoopRunStatus = "success" | "partial" | "failed" | "escalated";

export interface StopCondition {
  type: "max_iterations" | "budget_exhausted" | "error_threshold" | "manual";
  value: number;
}

export interface LoopTrigger {
  type: LoopTriggerType;
  intervalMs?: number;
  eventType?: string;
  eventFilter?: Record<string, unknown>;
}

export interface PhaseDef {
  name: string;
  description: string;
  type: "discover" | "deliver" | "verify" | "persist" | "schedule";
}

export interface PhaseResult {
  phase: string;
  status: string;
  output: string;
  durationMs: number;
}

export interface CostConfig {
  tokensNoop: number;
  tokensReport: number;
  tokensAction: number;
  dailyCap: number;
  earlyExitRequired: boolean;
}

export interface LoopConfig {
  cadence: Cadence;
  denylist: string[];
  humanGates: string[];
  maxRetries: number;
}

export interface LoopRunResult {
  findings: string[];
  actions: string[];
  escalations: string[];
}

export interface LoopPattern {
  id: string;
  name: string;
  description?: string;
  goal?: string;
  cadence: Cadence;
  riskLevel: RiskLevel;
  phases: PhaseDef[];
  humanGates: string[];
  costConfig: CostConfig;
  skillsRequired: string[];
  isActive: boolean;
  isBuiltin: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface LoopState {
  id: string;
  novelId: string;
  patternId: string;
  status: LoopStatus;
  readinessLevel: ReadinessLevel;
  statePayload: Record<string, unknown>;
  config: LoopConfig;
  tokenUsageToday: number;
  tokenCapDaily: number;
  lastRunAt?: string;
  lastRunResult?: LoopRunResult;
  createdAt: string;
  updatedAt: string;
}

export interface LoopRunLog {
  id: string;
  loopStateId?: string;
  patternId: string;
  status: LoopRunStatus;
  phaseResults: PhaseResult[];
  tokensUsed: number;
  durationMs: number;
  findings: string[];
  actionsTaken: string[];
  escalations: string[];
  errorMessage?: string;
  createdAt: string;
}

export const DEFAULT_COST_CONFIG: CostConfig = {
  tokensNoop: 0,
  tokensReport: 5_000,
  tokensAction: 20_000,
  dailyCap: 100_000,
  earlyExitRequired: false,
};

export const DEFAULT_LOOP_CONFIG: LoopConfig = {
  cadence: "manual",
  denylist: [],
  humanGates: [],
  maxRetries: 3,
};

export const BUILTIN_PATTERN_IDS = [
  "chapter-write-loop",
  "audit-revise-loop",
  "observation-loop",
  "consolidation-loop",
] as const;

export type BuiltinPatternId = (typeof BUILTIN_PATTERN_IDS)[number];

export function isBuiltinPatternId(id: string): id is BuiltinPatternId {
  return BUILTIN_PATTERN_IDS.includes(id as BuiltinPatternId);
}