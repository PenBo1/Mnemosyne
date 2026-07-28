import {
  FailurePattern,
  type FailureReport,
  type AgentTrace,
  type AgentStep,
  type FailureDetectorOptions,
  type FailureCallback,
  type ToolRegistry,
} from "./types";
import { HallucinationDetector } from "./hallucination-detector";
import { ScopeCreepDetector } from "./scope-creep-detector";
import { ContextLossDetector } from "./context-loss-detector";
import { CascadingErrorDetector } from "./cascading-error-detector";
import { ToolMisuseDetector } from "./tool-misuse-detector";

export class FailureDetector {
  private hallucinationDetector: HallucinationDetector;
  private scopeCreepDetector: ScopeCreepDetector;
  private contextLossDetector: ContextLossDetector;
  private cascadingErrorDetector: CascadingErrorDetector;
  private toolMisuseDetector: ToolMisuseDetector;

  private onFailureCallbacks: Set<FailureCallback> = new Set();
  private enabledPatterns: Set<FailurePattern>;
  private isRunning = false;
  private lastTrace: AgentTrace | null = null;

  constructor(options: FailureDetectorOptions = {}) {
    this.hallucinationDetector = new HallucinationDetector(options.toolRegistry);
    this.scopeCreepDetector = new ScopeCreepDetector();
    this.contextLossDetector = new ContextLossDetector(options.contextLossThreshold);
    this.cascadingErrorDetector = new CascadingErrorDetector(options.maxErrorPropagationDepth);
    this.toolMisuseDetector = new ToolMisuseDetector(options.toolRegistry);

    this.enabledPatterns = new Set([
      FailurePattern.HallucinatedAction,
      FailurePattern.ScopeCreep,
      FailurePattern.CascadingError,
      FailurePattern.ContextLoss,
      FailurePattern.ToolMisuse,
    ]);
  }

  setToolRegistry(registry: ToolRegistry): void {
    this.hallucinationDetector = new HallucinationDetector(registry);
    this.toolMisuseDetector = new ToolMisuseDetector(registry);
  }

  enablePattern(pattern: FailurePattern): void {
    this.enabledPatterns.add(pattern);
  }

  disablePattern(pattern: FailurePattern): void {
    this.enabledPatterns.delete(pattern);
  }

  setEnabledPatterns(patterns: FailurePattern[]): void {
    this.enabledPatterns = new Set(patterns);
  }

  setOriginalRequest(request: string): void {
    this.scopeCreepDetector.setOriginalRequest(request);
  }

  setConstraints(constraints: string[]): void {
    this.contextLossDetector.setConstraints(constraints);
  }

  onStepFinish(step: AgentStep): FailureReport[] {
    const reports: FailureReport[] = [];

    if (this.enabledPatterns.has(FailurePattern.HallucinatedAction)) {
      const report = this.hallucinationDetector.detect(step);
      if (report) reports.push(report);
    }

    if (this.enabledPatterns.has(FailurePattern.ToolMisuse)) {
      const report = this.toolMisuseDetector.detect(step);
      if (report) reports.push(report);
    }

    for (const report of reports) {
      this.emitFailure(report);
    }

    return reports;
  }

  analyzeTrace(trace: AgentTrace): FailureReport[] {
    const reports: FailureReport[] = [];

    this.lastTrace = trace;

    for (const step of trace.steps) {
      const stepReports = this.onStepFinish(step);
      reports.push(...stepReports);
    }

    if (this.enabledPatterns.has(FailurePattern.ScopeCreep)) {
      const report = this.scopeCreepDetector.detect(trace);
      if (report) reports.push(report);
    }

    if (this.enabledPatterns.has(FailurePattern.ContextLoss)) {
      const report = this.contextLossDetector.detect(trace);
      if (report) reports.push(report);
    }

    if (this.enabledPatterns.has(FailurePattern.CascadingError)) {
      const report = this.cascadingErrorDetector.detect(trace);
      if (report) reports.push(report);
    }

    const deduplicatedReports = this.deduplicateReports(reports);

    return deduplicatedReports;
  }

  report(trace: AgentTrace): FailureReport[] {
    return this.analyzeTrace(trace);
  }

  onFailure(callback: FailureCallback): () => void {
    this.onFailureCallbacks.add(callback);
    return () => {
      this.onFailureCallbacks.delete(callback);
    };
  }

  private emitFailure(report: FailureReport): void {
    for (const callback of this.onFailureCallbacks) {
      try {
        callback(report);
      } catch (error) {
        console.error("[FailureDetector] Callback error:", error);
      }
    }
  }

  private deduplicateReports(reports: FailureReport[]): FailureReport[] {
    const seen = new Map<string, FailureReport>();

    for (const report of reports) {
      const key = `${report.pattern}:${report.message}`;

      if (!seen.has(key)) {
        seen.set(key, report);
      } else {
        const existing = seen.get(key)!;
        if (report.severity === "error" && existing.severity !== "error") {
          seen.set(key, report);
        }
      }
    }

    return Array.from(seen.values());
  }

  registerTool(name: string): void {
    this.hallucinationDetector.registerTool(name);
  }

  registerTools(names: string[]): void {
    this.hallucinationDetector.registerTools(names);
  }

  registerToolSchema(schema: { name: string; requiredParams: string[]; optionalParams?: string[]; paramTypes?: Record<string, string> }): void {
    this.toolMisuseDetector.registerSchema({
      name: schema.name,
      requiredParams: schema.requiredParams,
      optionalParams: schema.optionalParams ?? [],
      paramTypes: schema.paramTypes ?? {},
    });
  }

  reset(): void {
    this.scopeCreepDetector.reset();
    this.contextLossDetector.reset();
    this.cascadingErrorDetector.reset();
    this.lastTrace = null;
  }

  getLastTrace(): AgentTrace | null {
    return this.lastTrace;
  }

  getIsRunning(): boolean {
    return this.isRunning;
  }
}

export function createFailureDetector(options?: FailureDetectorOptions): FailureDetector {
  return new FailureDetector(options);
}