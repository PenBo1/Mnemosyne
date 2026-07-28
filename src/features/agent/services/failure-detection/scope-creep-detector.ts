import { FailurePattern, type FailureReport, type AgentTrace } from "./types";

interface OperationScope {
  files: Set<string>;
  directories: Set<string>;
  operations: Set<string>;
}

const SCOPE_KEYWORDS = {
  read: ["read", "list", "search", "get", "fetch"],
  write: ["write", "create", "update", "modify", "delete", "remove"],
  exec: ["execute", "run", "shell", "command"],
};

export class ScopeCreepDetector {
  private originalScope: OperationScope | null = null;

  analyzeRequest(request: string): OperationScope {
    const scope: OperationScope = {
      files: new Set(),
      directories: new Set(),
      operations: new Set(),
    };

    const lowerRequest = request.toLowerCase();

    if (SCOPE_KEYWORDS.read.some((kw) => lowerRequest.includes(kw))) {
      scope.operations.add("read");
    }
    if (SCOPE_KEYWORDS.write.some((kw) => lowerRequest.includes(kw))) {
      scope.operations.add("write");
    }
    if (SCOPE_KEYWORDS.exec.some((kw) => lowerRequest.includes(kw))) {
      scope.operations.add("execute");
    }

    const filePathMatches = request.match(/(?:\/[\w.-]+)+\/[\w.-]+\.\w+/g) ?? [];
    for (const match of filePathMatches) {
      scope.files.add(match);
    }

    const dirPathMatches = request.match(/(?:\/[\w.-]+)+\/[\w.-]+/g) ?? [];
    for (const match of dirPathMatches) {
      if (!match.includes(".")) {
        scope.directories.add(match);
      }
    }

    return scope;
  }

  setOriginalRequest(request: string): void {
    this.originalScope = this.analyzeRequest(request);
  }

  detect(trace: AgentTrace): FailureReport | null {
    if (!trace.originalRequest) {
      return null;
    }

    if (!this.originalScope) {
      this.originalScope = this.analyzeRequest(trace.originalRequest);
    }

    const operationTypes = new Set<string>();
    const accessedFiles = new Set<string>();

    for (const step of trace.steps) {
      if (step.type !== "tool_call" || !step.toolName) {
        continue;
      }

      const toolName = step.toolName.toLowerCase();
      if (SCOPE_KEYWORDS.read.some((kw) => toolName.includes(kw))) {
        operationTypes.add("read");
      }
      if (SCOPE_KEYWORDS.write.some((kw) => toolName.includes(kw))) {
        operationTypes.add("write");
      }
      if (SCOPE_KEYWORDS.exec.some((kw) => toolName.includes(kw))) {
        operationTypes.add("execute");
      }

      if (step.toolArgs) {
        const path = this.extractPathFromArgs(step.toolArgs);
        if (path) {
          accessedFiles.add(path);
        }
      }
    }

    const scopeCreepOps = this.detectScopeCreepOperations(this.originalScope.operations, operationTypes);

    if (scopeCreepOps.length > 0) {
      return {
        pattern: FailurePattern.ScopeCreep,
        severity: "warning",
        message: `Detected operation scope beyond original request: ${scopeCreepOps.join(", ")}`,
        suggestion: "This operation may exceed the user's original request scope. Consider confirming before proceeding.",
        timestamp: Date.now(),
        metadata: {
          originalScope: Array.from(this.originalScope.operations),
          actualScope: Array.from(operationTypes),
          scopeCreepOps,
          i18nKey: "failureDetection.scopeCreep",
          i18nParams: { operations: scopeCreepOps.join(", ") },
        },
      };
    }

    return null;
  }

  private extractPathFromArgs(args: Record<string, unknown>): string | null {
    if (typeof args.path === "string") return args.path;
    if (typeof args.file_path === "string") return args.file_path;
    if (typeof args.file === "string") return args.file;
    if (typeof args.directory === "string") return args.directory;
    if (typeof args.dir === "string") return args.dir;
    return null;
  }

  private detectScopeCreepOperations(
    original: Set<string>,
    actual: Set<string>
  ): string[] {
    const creepOps: string[] = [];

    for (const op of actual) {
      if (!original.has(op)) {
        creepOps.push(op);
      }
    }

    return creepOps;
  }

  reset(): void {
    this.originalScope = null;
  }
}