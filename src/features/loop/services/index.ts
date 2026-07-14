import { ipc } from "@/services/ipc";
import type { LoopState, LoopPattern, LoopRunLog, CreateLoopStateRequest, UpdateLoopStateRequest, UpsertLoopPatternRequest } from "@/features/loop/types";

export async function createLoopState(
  novelId: string,
  req: CreateLoopStateRequest
): Promise<LoopState> {
  return ipc<LoopState>("loop_create_state", {
    novelId,
    patternId: req.patternId,
    readinessLevel: req.readinessLevel,
    config: req.config,
    tokenCapDaily: req.tokenCapDaily,
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
    readinessLevel: req.readinessLevel,
    config: req.config,
    tokenCapDaily: req.tokenCapDaily,
  });
}

export async function deleteLoopState(stateId: string): Promise<void> {
  await ipc<unknown>("loop_delete_state", { stateId });
}

export async function runLoopCycle(_stateId: string): Promise<LoopRunLog> {
  // loop_run_cycle IPC 已移除，命令迁移至前端 AI 引擎。
  // 保留签名以避免 loop-engine store 的编译错误，调用时会抛错。
  throw new Error("该命令已迁移至前端 AI 引擎");
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
    riskLevel: req.riskLevel,
    phases: req.phases,
    humanGates: req.humanGates,
    costConfig: req.costConfig,
    skillsRequired: req.skillsRequired,
    stateSchema: req.stateSchema ?? undefined,
    isActive: req.isActive,
  });
}

export async function pauseLoop(stateId: string): Promise<void> {
  await ipc<unknown>("loop_pause", { stateId });
}

export async function resumeLoop(stateId: string): Promise<void> {
  await ipc<unknown>("loop_resume", { stateId });
}
