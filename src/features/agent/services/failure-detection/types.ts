import type { ToolCallStatus } from "../tool-protocol";

export enum FailurePattern {
  HallucinatedAction = "HallucinatedAction",
  ScopeCreep = "ScopeCreep",
  CascadingError = "CascadingError",
  ContextLoss = "ContextLoss",
  ToolMisuse = "ToolMisuse",
}

export type FailureSeverity = "error" | "warning" | "info";

export interface FailureReport {
  pattern: FailurePattern;
  severity: FailureSeverity;
  message: string;
  suggestion: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

export interface AgentStep {
  id: string;
  type: "tool_call" | "text" | "reasoning";
  toolName?: string;
  toolArgs?: Record<string, unknown>;
  toolResult?: unknown;
  status: ToolCallStatus;
  error?: string;
  timestamp: number;
}

export interface AgentTrace {
  steps: AgentStep[];
  originalRequest?: string;
  constraints?: string[];
}

export interface ToolRegistry {
  hasTool(name: string): boolean;
  getToolNames(): string[];
}

export interface FailureDetectorOptions {
  toolRegistry?: ToolRegistry;
  maxErrorPropagationDepth?: number;
  contextLossThreshold?: number;
}

export type FailureCallback = (report: FailureReport) => void;