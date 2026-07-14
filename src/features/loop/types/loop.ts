// ── Loop Engineering ───────────────────────────────────────
//
// LoopEngineState（含 actions 的 store 状态接口）也在此处定义。
// 风险等级类型名为 LoopRiskLevel（与 index.ts 中的 RiskLevel 对应）。
//
// 字段命名约定:camelCase,与后端 application/loop_engine/types.rs 的
// #[serde(rename_all = "camelCase")] DTO 对齐。

export type LoopStatus = "idle" | "running" | "paused" | "error";
export type ReadinessLevel = "L0" | "L1" | "L2" | "L3";
export type LoopRunStatus = "success" | "partial" | "failed" | "escalated";
export type LoopRiskLevel = "low" | "medium" | "high";

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
  lastRunAt: string | null;
  lastRunResult: LoopRunResult | null;
  createdAt: string;
  updatedAt: string;
}

export interface LoopConfig {
  cadence: string;
  denylist: string[];
  humanGates: string[];
  maxRetries: number;
}

export interface LoopRunResult {
  findings: string[];
  actions: string[];
  escalations: string[];
}

export interface CreateLoopStateRequest {
  patternId: string;
  readinessLevel?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  tokenCapDaily?: number;
}

export interface UpdateLoopStateRequest {
  status?: LoopStatus;
  readinessLevel?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  tokenCapDaily?: number;
}

export interface LoopRunLog {
  id: string;
  loopStateId: string;
  patternId: string;
  status: LoopRunStatus;
  phaseResults: PhaseResult[];
  tokensUsed: number;
  durationMs: number;
  findings: string[];
  actionsTaken: string[];
  escalations: string[];
  errorMessage: string | null;
  createdAt: string;
}

export interface PhaseResult {
  phase: string;
  status: string;
  output: string;
  durationMs: number;
}

export interface LoopPattern {
  id: string;
  name: string;
  description: string;
  goal: string;
  cadence: string;
  riskLevel: LoopRiskLevel;
  phases: PhaseDef[];
  humanGates: string[];
  costConfig: CostConfig;
  skillsRequired: string[];
  isActive: boolean;
  isBuiltin?: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface PhaseDef {
  name: string;
  description: string;
  type: "discover" | "deliver" | "verify" | "persist" | "schedule";
}

export interface CostConfig {
  tokensNoop: number;
  tokensReport: number;
  tokensAction: number;
  dailyCap: number;
  earlyExitRequired: boolean;
}

export interface UpsertLoopPatternRequest {
  name: string;
  description?: string;
  goal?: string;
  cadence?: string;
  riskLevel?: LoopRiskLevel;
  phases?: PhaseDef[];
  humanGates?: string[];
  costConfig?: Partial<CostConfig>;
  skillsRequired?: string[];
  stateSchema?: Record<string, unknown>;
  isActive?: boolean;
}

export interface LoopEngineState {
  states: LoopState[];
  patterns: LoopPattern[];
  runLogs: LoopRunLog[];
  loading: boolean;
  error: string | null;
  loadStates: (novelId: string) => Promise<void>;
  loadPatterns: () => Promise<void>;
  createState: (novelId: string, req: CreateLoopStateRequest) => Promise<LoopState>;
  deleteState: (stateId: string) => Promise<void>;
  runCycle: (stateId: string) => Promise<LoopRunLog>;
  pauseLoop: (stateId: string) => Promise<void>;
  resumeLoop: (stateId: string) => Promise<void>;
  loadRunLogs: (stateId: string) => Promise<void>;
}
