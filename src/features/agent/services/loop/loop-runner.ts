// Loop-Engineering 运行器 —— 管理 Loop 生命周期。

import { ipc, ipcVoid } from "@/services/ipc";
import { useLoopStateStore } from "./loop-state";
import type { LoopState, LoopConfig } from "./loop-patterns";

export interface ScheduleWakeupParams {
  stateId: string;
  intervalMs: number;
  maxIterations?: number;
}

export interface ArmMonitorParams {
  stateId: string;
  eventType: "file_change" | "git_commit" | "chapter_complete";
  filter?: Record<string, unknown>;
}

export interface TaskStopParams {
  stateId: string;
  reason?: string;
}

export interface LoopRunnerResult {
  success: boolean;
  state?: LoopState;
  nextTickAt?: number;
  error?: string;
}

// Loop 动态间隔的 token-usage 阈值与倍数（依据当日 token 用量自适应调速）。
const TOKEN_USAGE_CRITICAL_RATIO = 0.9;   // 危险阈值：超过则大幅减速
const INTERVAL_SLOWDOWN_CRITICAL_MULT = 4; // 危险区间减速倍数
const TOKEN_USAGE_HIGH_RATIO = 0.7;        // 高位阈值：超过则适度减速
const INTERVAL_SLOWDOWN_HIGH_MULT = 2;     // 高位区间减速倍数
const TOKEN_USAGE_LOW_RATIO = 0.3;         // 低位阈值：低于则考虑加速
const MIN_ITERATIONS_BEFORE_SPEEDUP = 10;  // 加速前需达到的最小迭代数
const MIN_SPEEDUP_INTERVAL_MS = 10_000;    // 加速时最小间隔（10 秒）
const INTERVAL_SPEEDUP_DIVISOR = 2;        // 加速时基础间隔除数

const ACTIVE_RUNNERS = new Map<string, LoopRunner>();

export class LoopRunner {
  private stateId: string;
  private intervalId: ReturnType<typeof setInterval> | null = null;
  private iterationCount: number = 0;
  private maxIterations: number = 1000;
  private stopped: boolean = false;
  private nextTickAt: number | null = null;

  private constructor(stateId: string, _patternId: string) {
    this.stateId = stateId;
  }

  static async create(
    novelId: string,
    patternId: string,
    readinessLevel?: string,
    config?: LoopConfig,
    tokenCapDaily?: number
  ): Promise<LoopRunner> {
    const existing = Array.from(ACTIVE_RUNNERS.values()).find((r) => r.stateId === patternId);
    if (existing) return existing;

    const state = await ipc<LoopState>("loop_create_state", {
      novelId,
      patternId,
      readinessLevel,
      config,
      tokenCapDaily,
    });

    const runner = new LoopRunner(state.id, patternId);
    ACTIVE_RUNNERS.set(state.id, runner);
    useLoopStateStore.getState().setActiveLoop(state.id, {
      stateId: state.id,
      patternId,
      startedAt: Date.now(),
      iterationCount: 0,
    });
    return runner;
  }

  static get(stateId: string): LoopRunner | undefined {
    return ACTIVE_RUNNERS.get(stateId);
  }

  static async stop(stateId: string, reason?: string): Promise<void> {
    const runner = ACTIVE_RUNNERS.get(stateId);
    if (runner) {
      await runner.stopInternal(reason);
      ACTIVE_RUNNERS.delete(stateId);
      useLoopStateStore.getState().setActiveLoop(stateId, undefined);
    }
  }

  static stopAll(): void {
    for (const runner of ACTIVE_RUNNERS.values()) {
      runner.stopInternal().catch((err) => { console.error("[loop-runner] stopInternal failed:", err); });
    }
    ACTIVE_RUNNERS.clear();
  }

  async start(params: ScheduleWakeupParams): Promise<LoopRunnerResult> {
    if (this.intervalId) return { success: false, error: "Loop already running" };

    this.maxIterations = params.maxIterations ?? 1000;
    this.stopped = false;
    this.iterationCount = 0;

    const state = useLoopStateStore.getState().getState(this.stateId);
    if (!state) return { success: false, error: "Loop state not found" };

    await ipcVoid("loop_resume", { stateId: this.stateId });
    useLoopStateStore.getState().updateStateStatus(this.stateId, "running");
    this.scheduleNextTick(params.intervalMs);
    return { success: true, state: useLoopStateStore.getState().getState(this.stateId), nextTickAt: this.nextTickAt ?? undefined };
  }

  private scheduleNextTick(intervalMs: number): void {
    if (this.stopped) return;
    const dynamicInterval = this.calculateDynamicInterval(intervalMs);
    this.nextTickAt = Date.now() + dynamicInterval;
    this.intervalId = setInterval(() => this.tick().catch(console.error), dynamicInterval);
    useLoopStateStore.getState().updateActiveLoopNextTick(this.stateId, this.nextTickAt);
  }

  private calculateDynamicInterval(baseIntervalMs: number): number {
    const state = useLoopStateStore.getState().getState(this.stateId);
    if (!state) return baseIntervalMs;
    const usageRatio = state.tokenUsageToday / state.tokenCapDaily;
    if (usageRatio > TOKEN_USAGE_CRITICAL_RATIO) return baseIntervalMs * INTERVAL_SLOWDOWN_CRITICAL_MULT;
    if (usageRatio > TOKEN_USAGE_HIGH_RATIO) return baseIntervalMs * INTERVAL_SLOWDOWN_HIGH_MULT;
    if (usageRatio < TOKEN_USAGE_LOW_RATIO && this.iterationCount > MIN_ITERATIONS_BEFORE_SPEEDUP) {
      return Math.max(baseIntervalMs / INTERVAL_SPEEDUP_DIVISOR, MIN_SPEEDUP_INTERVAL_MS);
    }
    return baseIntervalMs;
  }

  private async tick(): Promise<void> {
    if (this.stopped) return;
    this.iterationCount++;
    useLoopStateStore.getState().incrementActiveLoopIteration(this.stateId);
    if (this.iterationCount >= this.maxIterations) {
      await LoopRunner.stop(this.stateId, "Max iterations reached");
      return;
    }
    try {
      await ipcVoid("loop_resume", { stateId: this.stateId });
    } catch (err) {
      useLoopStateStore.getState().updateStateStatus(this.stateId, "error");
      console.error(`Loop ${this.stateId} tick failed:`, err);
    }
  }

  async armMonitor(_params: ArmMonitorParams): Promise<LoopRunnerResult> {
    if (this.intervalId) return { success: false, error: "Loop already running with timer" };
    this.stopped = false;
    const state = useLoopStateStore.getState().getState(this.stateId);
    if (!state) return { success: false, error: "Loop state not found" };
    await ipcVoid("loop_resume", { stateId: this.stateId });
    useLoopStateStore.getState().updateStateStatus(this.stateId, "running");
    return { success: true, state: useLoopStateStore.getState().getState(this.stateId) };
  }

  private async stopInternal(_reason?: string): Promise<void> {
    this.stopped = true;
    if (this.intervalId) { clearInterval(this.intervalId); this.intervalId = null; }
    this.nextTickAt = null;
    try {
      await ipcVoid("loop_pause", { stateId: this.stateId });
      useLoopStateStore.getState().updateStateStatus(this.stateId, "paused");
    } catch (err) { console.error(`Failed to stop loop ${this.stateId}:`, err); }
  }

  getIterationCount(): number { return this.iterationCount; }
  getNextTickAt(): number | null { return this.nextTickAt; }
  isRunning(): boolean { return !this.stopped && this.intervalId !== null; }
}

export async function scheduleWakeup(params: ScheduleWakeupParams): Promise<LoopRunnerResult> {
  const runner = LoopRunner.get(params.stateId);
  if (!runner) return { success: false, error: "Loop runner not found" };
  return runner.start(params);
}

export async function armMonitor(params: ArmMonitorParams): Promise<LoopRunnerResult> {
  const runner = LoopRunner.get(params.stateId);
  if (!runner) return { success: false, error: "Loop runner not found" };
  return runner.armMonitor(params);
}

export async function taskStop(params: TaskStopParams): Promise<void> {
  await LoopRunner.stop(params.stateId, params.reason);
}

export function stopAllLoopRunners(): void { LoopRunner.stopAll(); }