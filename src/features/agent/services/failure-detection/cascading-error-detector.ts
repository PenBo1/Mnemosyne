import { FailurePattern, type FailureReport, type AgentTrace, type AgentStep } from "./types";

interface ErrorNode {
  stepIndex: number;
  errorMessage: string;
  toolName?: string;
  propagatedTo: number[];
}

export class CascadingErrorDetector {
  private maxErrorPropagationDepth: number;
  private errorNodes: ErrorNode[] = [];
  private errorCount = 0;

  constructor(maxErrorPropagationDepth = 3) {
    this.maxErrorPropagationDepth = maxErrorPropagationDepth;
  }

  detect(trace: AgentTrace): FailureReport | null {
    this.errorNodes = [];
    this.errorCount = 0;

    const failedSteps = this.extractFailedSteps(trace.steps);

    if (failedSteps.length === 0) {
      return null;
    }

    for (let i = 0; i < failedSteps.length; i++) {
      const step = failedSteps[i];
      this.errorNodes.push({
        stepIndex: step.index,
        errorMessage: step.error ?? "Unknown error",
        toolName: step.toolName,
        propagatedTo: [],
      });
    }

    this.buildPropagationGraph(trace.steps);

    const cascadingChains = this.findCascadingChains();

    if (cascadingChains.length > 0) {
      const longestChain = cascadingChains.reduce((a, b) => (a.length > b.length ? a : b));

      return {
        pattern: FailurePattern.CascadingError,
        severity: "error",
        message: `Detected cascading errors: ${cascadingChains.length} error chains, max depth ${longestChain.length}`,
        suggestion: `Root cause: ${this.formatRootCause(longestChain)}. Consider fixing this error first to stop error propagation.`,
        timestamp: Date.now(),
        metadata: {
          cascadingChains: cascadingChains.map((chain) => this.formatChain(chain)),
          errorCount: this.errorCount,
          maxDepth: longestChain.length,
          i18nKey: "failureDetection.cascadingError",
          i18nParams: { count: cascadingChains.length, depth: longestChain.length },
        },
      };
    }

    return null;
  }

  private extractFailedSteps(steps: AgentStep[]): Array<AgentStep & { index: number }> {
    const failed: Array<AgentStep & { index: number }> = [];

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      if (step.status === "failed" || step.error) {
        failed.push({ ...step, index: i });
        this.errorCount++;
      }
    }

    return failed;
  }

  private buildPropagationGraph(steps: AgentStep[]): void {
    for (let i = 0; i < this.errorNodes.length; i++) {
      const errorNode = this.errorNodes[i];

      for (let j = i + 1; j < this.errorNodes.length; j++) {
        const laterError = this.errorNodes[j];

        if (this.isPropagation(steps, errorNode.stepIndex, laterError.stepIndex)) {
          errorNode.propagatedTo.push(j);
        }
      }
    }
  }

  private isPropagation(steps: AgentStep[], fromIndex: number, toIndex: number): boolean {
    if (toIndex <= fromIndex) {
      return false;
    }

    const fromStep = steps[fromIndex];
    const toStep = steps[toIndex];

    if (fromStep.toolName && toStep.toolName) {
      const relatedTools = this.areToolsRelated(fromStep.toolName, toStep.toolName);
      if (relatedTools) {
        return true;
      }
    }

    const distance = toIndex - fromIndex;
    if (distance <= 3 && fromStep.type === "tool_call" && toStep.type === "tool_call") {
      return true;
    }

    return false;
  }

  private areToolsRelated(tool1: string, tool2: string): boolean {
    const writeReadPairs: [string, string][] = [
      ["fs_write_file", "fs_read_file"],
      ["write_to_file", "read_file"],
      ["fs_create_dir", "fs_list_dir"],
    ];

    for (const [write, read] of writeReadPairs) {
      if (
        (tool1.includes(write.split("_")[0]) && tool2.includes(read.split("_")[0])) ||
        (tool1.includes(read.split("_")[0]) && tool2.includes(write.split("_")[0]))
      ) {
        return true;
      }
    }

    return false;
  }

  private findCascadingChains(): number[][] {
    const chains: number[][] = [];

    for (let i = 0; i < this.errorNodes.length; i++) {
      const chain = this.buildChain(i, new Set<number>());
      if (chain.length >= this.maxErrorPropagationDepth) {
        chains.push(chain);
      }
    }

    return chains;
  }

  private buildChain(errorIndex: number, visited: Set<number>): number[] {
    if (visited.has(errorIndex)) {
      return [];
    }

    visited.add(errorIndex);

    const errorNode = this.errorNodes[errorIndex];
    if (!errorNode) {
      return [errorIndex];
    }

    let maxChain: number[] = [errorIndex];

    for (const propagatedIndex of errorNode.propagatedTo) {
      const subChain = this.buildChain(propagatedIndex, visited);
      if (subChain.length + 1 > maxChain.length) {
        maxChain = [errorIndex, ...subChain];
      }
    }

    return maxChain;
  }

  private formatRootCause(chain: number[]): string {
    if (chain.length === 0) return "Unknown";

    const rootError = this.errorNodes[chain[0]];
    if (!rootError) return "Unknown";

    return rootError.toolName
      ? `${rootError.toolName}: ${rootError.errorMessage}`
      : rootError.errorMessage;
  }

  private formatChain(chain: number[]): string[] {
    return chain.map((idx) => {
      const node = this.errorNodes[idx];
      return node ? `${node.stepIndex}: ${node.errorMessage}` : "Unknown";
    });
  }

  reset(): void {
    this.errorNodes = [];
    this.errorCount = 0;
  }
}