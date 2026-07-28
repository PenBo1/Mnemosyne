export {
  FailurePattern,
  type FailureReport,
  type FailureSeverity,
  type AgentStep,
  type AgentTrace,
  type ToolRegistry,
  type FailureDetectorOptions,
  type FailureCallback,
} from "./types";

export { FailureDetector, createFailureDetector } from "./detector";

export { HallucinationDetector } from "./hallucination-detector";
export { ScopeCreepDetector } from "./scope-creep-detector";
export { ContextLossDetector } from "./context-loss-detector";
export { CascadingErrorDetector } from "./cascading-error-detector";
export { ToolMisuseDetector } from "./tool-misuse-detector";