import { ipc } from "@/infrastructure/api";
import type {
  LoopState,
  LoopPattern,
  LoopRunLog,
  CreateLoopStateRequest,
  UpdateLoopStateRequest,
  UpsertLoopPatternRequest,
} from "@/shared/types";

export async function createLoopState(
  novelId: string,
  req: CreateLoopStateRequest
): Promise<LoopState> {
  return ipc<LoopState>("loop_create_state", {
    novelId,
    patternId: req.pattern_id,
    readinessLevel: req.readiness_level,
    config: req.config,
    tokenCapDaily: req.token_cap_daily,
  });
}

export async function getLoopStates(novelId: string): Promise<LoopState[]> {
  return ipc<LoopState[]>("loop_get_states", { novelId });
}

export async function updateLoopState(
  stateId: string,
  req: UpdateLoopStateRequest
): Promise<LoopState> {
  return ipc<LoopState>("loop_update_state", {
    stateId,
    status: req.status,
    readinessLevel: req.readiness_level,
    config: req.config,
    tokenCapDaily: req.token_cap_daily,
  });
}

export async function deleteLoopState(stateId: string): Promise<void> {
  await ipc<unknown>("loop_delete_state", { stateId });
}

export async function runLoopCycle(stateId: string): Promise<LoopRunLog> {
  return ipc<LoopRunLog>("loop_run_cycle", { stateId });
}

export async function getRunLogs(
  stateId: string,
  limit?: number
): Promise<LoopRunLog[]> {
  return ipc<LoopRunLog[]>("loop_get_run_logs", { stateId, limit });
}

export async function getPatterns(): Promise<LoopPattern[]> {
  return ipc<LoopPattern[]>("loop_get_patterns");
}

export async function upsertPattern(
  req: UpsertLoopPatternRequest,
  id?: string
): Promise<LoopPattern> {
  return ipc<LoopPattern>("loop_upsert_pattern", {
    id,
    name: req.name,
    description: req.description,
    goal: req.goal,
    cadence: req.cadence,
    riskLevel: req.risk_level,
    phases: req.phases,
    humanGates: req.human_gates,
    costConfig: req.cost_config,
    skillsRequired: req.skills_required,
    stateSchema: req.state_schema ?? undefined,
    isActive: req.is_active,
  });
}

export async function pauseLoop(stateId: string): Promise<void> {
  await ipc<unknown>("loop_pause", { stateId });
}

export async function resumeLoop(stateId: string): Promise<void> {
  await ipc<unknown>("loop_resume", { stateId });
}

export async function getBudgetStatus(
  stateId: string
): Promise<{ used: number; cap: number; remaining: number; usage_percent: number; exceeded: boolean }> {
  return ipc("loop_get_budget_status", { stateId });
}
