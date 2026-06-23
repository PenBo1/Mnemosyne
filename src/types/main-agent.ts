// ── Main Agent Types ──────────────────────────────────────────

export type AgentStatus =
  | "Idle"
  | "Planning"
  | "Executing"
  | "WaitingForConfirmation"
  | "Paused"
  | "Completed"
  | "Failed";

export type StepStatus =
  | "Pending"
  | "InProgress"
  | "Completed"
  | "Failed"
  | "Skipped"
  | "AwaitingConfirmation";

export type RiskLevel = "Safe" | "Moderate" | "High";

export interface PlanStep {
  id: number;
  description: string;
  tool_name: string | null;
  tool_args: Record<string, unknown> | null;
  risk_level: RiskLevel;
  status: StepStatus;
  result: string | null;
}

export interface ConfirmationRequest {
  step_id: number;
  description: string;
  details: string;
  risk_level: RiskLevel;
}

export interface ProgressUpdate {
  status: AgentStatus;
  current_step: number | null;
  total_steps: number | null;
  message: string;
}

export interface MainAgentEvent {
  type: "Progress" | "ConfirmationRequired" | "Completed" | "Failed";
  session_id: string;
  // Progress fields
  status?: AgentStatus;
  current_step?: number | null;
  total_steps?: number | null;
  message?: string;
  // Confirmation fields
  step_id?: number;
  description?: string;
  details?: string;
  risk_level?: string;
  // Result fields
  result?: string;
  error?: string;
}

export interface MainAgentSession {
  id: string;
  goal: string;
  status: AgentStatus;
  plan: PlanStep[];
  currentStep: number | null;
  totalSteps: number | null;
  messages: MainAgentMessage[];
  confirmation: ConfirmationRequest | null;
  result: string | null;
  error: string | null;
  createdAt: string;
}

export interface MainAgentMessage {
  id: string;
  role: "user" | "agent" | "system";
  content: string;
  timestamp: string;
}
