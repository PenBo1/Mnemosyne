// ── Loop Engineering ───────────────────────────────────────
//
// LoopEngineState（含 actions 的 store 状态接口）也在此处定义。
// 风险等级类型名为 LoopRiskLevel（与 index.ts 中的 RiskLevel 对应）。

export type LoopStatus = "idle" | "running" | "paused" | "error";
export type ReadinessLevel = "L0" | "L1" | "L2" | "L3";
export type LoopRunStatus = "success" | "partial" | "failed" | "escalated";
export type LoopRiskLevel = "low" | "medium" | "high";

export interface LoopState {
  id: string;
  novel_id: string;
  pattern_id: string;
  status: LoopStatus;
  readiness_level: ReadinessLevel;
  state_payload: Record<string, unknown>;
  config: LoopConfig;
  token_usage_today: number;
  token_cap_daily: number;
  last_run_at: string | null;
  last_run_result: LoopRunResult | null;
  created_at: string;
  updated_at: string;
}

export interface LoopConfig {
  cadence: string;
  denylist: string[];
  human_gates: string[];
  max_retries: number;
}

export interface LoopRunResult {
  findings: string[];
  actions: string[];
  escalations: string[];
}

export interface CreateLoopStateRequest {
  pattern_id: string;
  readiness_level?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  token_cap_daily?: number;
}

export interface UpdateLoopStateRequest {
  status?: LoopStatus;
  readiness_level?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  token_cap_daily?: number;
}

export interface LoopRunLog {
  id: string;
  loop_state_id: string;
  pattern_id: string;
  status: LoopRunStatus;
  phase_results: PhaseResult[];
  tokens_used: number;
  duration_ms: number;
  findings: string[];
  actions_taken: string[];
  escalations: string[];
  error_message: string | null;
  created_at: string;
}

export interface PhaseResult {
  phase: string;
  status: string;
  output: string;
  duration_ms: number;
}

export interface LoopPattern {
  id: string;
  name: string;
  description: string;
  goal: string;
  cadence: string;
  risk_level: LoopRiskLevel;
  phases: PhaseDef[];
  human_gates: string[];
  cost_config: CostConfig;
  skills_required: string[];
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PhaseDef {
  name: string;
  description: string;
  type: "discover" | "deliver" | "verify" | "persist" | "schedule";
}

export interface CostConfig {
  tokens_noop: number;
  tokens_report: number;
  tokens_action: number;
  daily_cap: number;
  early_exit_required: boolean;
}

export interface UpsertLoopPatternRequest {
  name: string;
  description?: string;
  goal?: string;
  cadence?: string;
  risk_level?: LoopRiskLevel;
  phases?: PhaseDef[];
  human_gates?: string[];
  cost_config?: Partial<CostConfig>;
  skills_required?: string[];
  state_schema?: Record<string, unknown>;
  is_active?: boolean;
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
