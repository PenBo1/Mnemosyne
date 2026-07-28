import { FailurePattern, type FailureReport, type AgentTrace } from "./types";

interface ConstraintState {
  constraint: string;
  mentioned: boolean;
  violated: boolean;
  mentionStepIndex: number;
  violationStepIndex?: number;
}

export class ContextLossDetector {
  private constraints: Map<string, ConstraintState> = new Map();
  private stepCount = 0;
  private contextLossThreshold: number;
  private violationHistory: string[] = [];

  constructor(contextLossThreshold = 10) {
    this.contextLossThreshold = contextLossThreshold;
  }

  setConstraints(constraints: string[]): void {
    this.constraints.clear();
    this.stepCount = 0;
    this.violationHistory = [];

    for (const constraint of constraints) {
      this.constraints.set(constraint, {
        constraint,
        mentioned: false,
        violated: false,
        mentionStepIndex: -1,
      });
    }
  }

  detect(trace: AgentTrace): FailureReport | null {
    if (trace.constraints && trace.constraints.length > 0) {
      this.setConstraints(trace.constraints);
    }

    for (let i = 0; i < trace.steps.length; i++) {
      const step = trace.steps[i];
      this.processStep(step, i);
    }

    const forgottenConstraints = this.detectForgottenConstraints();
    const violatedConstraints = this.detectViolatedConstraints();

    if (forgottenConstraints.length > 0) {
      return {
        pattern: FailurePattern.ContextLoss,
        severity: "warning",
        message: `Detected potentially forgotten constraints: ${forgottenConstraints.join(", ")}`,
        suggestion: `The following constraints may have been forgotten during execution:\n- ${forgottenConstraints.join("\n- ")}\n\nConsider revisiting these constraints.`,
        timestamp: Date.now(),
        metadata: {
          forgottenConstraints,
          stepCount: this.stepCount,
          i18nKey: "failureDetection.contextLoss",
          i18nParams: { constraints: forgottenConstraints.join(", ") },
        },
      };
    }

    if (violatedConstraints.length > 0) {
      return {
        pattern: FailurePattern.ContextLoss,
        severity: "error",
        message: `Detected constraint violation: ${violatedConstraints.join(", ")}`,
        suggestion: "Current operation violates established constraints. Consider stopping and re-evaluating.",
        timestamp: Date.now(),
        metadata: {
          violatedConstraints,
          violationHistory: this.violationHistory,
          i18nKey: "failureDetection.constraintViolation",
          i18nParams: { constraints: violatedConstraints.join(", ") },
        },
      };
    }

    return null;
  }

  private processStep(step: { type: string; toolName?: string; toolArgs?: Record<string, unknown> }, index: number): void {
    this.stepCount++;

    for (const [constraint, state] of this.constraints) {
      if (!state.mentioned) {
        state.mentioned = true;
        state.mentionStepIndex = index;
      }

      const violation = this.checkConstraintViolation(constraint, step);
      if (violation && !state.violated) {
        state.violated = true;
        state.violationStepIndex = index;
        this.violationHistory.push(`Step ${index}: ${violation}`);
      }
    }
  }

  private checkConstraintViolation(
    constraint: string,
    step: { type: string; toolName?: string; toolArgs?: Record<string, unknown> }
  ): string | null {
    if (step.type !== "tool_call" || !step.toolName) {
      return null;
    }

    const lowerConstraint = constraint.toLowerCase();
    const toolName = step.toolName.toLowerCase();

    if (lowerConstraint.includes("不要删除") || lowerConstraint.includes("don't delete")) {
      if (toolName.includes("delete") || toolName.includes("remove")) {
        return `Tool '${step.toolName}' may violate constraint: ${constraint}`;
      }
    }

    if (lowerConstraint.includes("只读") || lowerConstraint.includes("read-only")) {
      if (toolName.includes("write") || toolName.includes("delete") || toolName.includes("modify")) {
        return `Tool '${step.toolName}' may violate read-only constraint`;
      }
    }

    return null;
  }

  private detectForgottenConstraints(): string[] {
    const forgotten: string[] = [];
    const stepsSinceLastMention = this.stepCount;

    for (const [constraint, state] of this.constraints) {
      if (state.mentioned && !state.violated) {
        const stepsSinceMention = stepsSinceLastMention - state.mentionStepIndex;
        if (stepsSinceMention > this.contextLossThreshold) {
          forgotten.push(constraint);
        }
      }
    }

    return forgotten;
  }

  private detectViolatedConstraints(): string[] {
    const violated: string[] = [];
    for (const [constraint, state] of this.constraints) {
      if (state.violated) {
        violated.push(constraint);
      }
    }
    return violated;
  }

  reset(): void {
    this.constraints.clear();
    this.stepCount = 0;
    this.violationHistory = [];
  }
}