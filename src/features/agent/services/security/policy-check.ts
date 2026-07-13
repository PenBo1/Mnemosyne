import type {
  Operation,
  RiskLevel,
  PolicyDecision,
  FsScope,
  FsOperation,
} from "@/services/ipc/security";

export function calculateRiskLevel(operation: Operation): RiskLevel {
  switch (operation.type) {
    case "filesystem":
      return calculateFsRisk(
        operation.scope as FsScope,
        operation.operation as FsOperation
      );
    case "shell":
      return calculateShellRisk(operation.scope);
    case "network":
      return calculateNetworkRisk(operation.method ?? "GET");
    default:
      return "medium";
  }
}

function calculateFsRisk(scope: FsScope, operation: FsOperation): RiskLevel {
  switch (operation) {
    case "read":
    case "list":
      return "low";
    case "write":
    case "create_dir":
      return isWritableScope(scope) ? "medium" : "critical";
    case "delete":
      return scope === "workspace" ? "high" : "critical";
    default:
      return "medium";
  }
}

function isWritableScope(scope: FsScope): boolean {
  const writableScopes: FsScope[] = [
    "workspace",
    "app_data",
    "cache",
    "temp",
    "plugins",
    "logs",
  ];
  return writableScopes.includes(scope);
}

function calculateShellRisk(scope: unknown): RiskLevel {
  const shellScope = scope as { type: string; operations?: string[] };
  if (shellScope.type === "git") {
    const hasWrite = (shellScope.operations ?? []).some((op) =>
      isGitWriteOperation(op)
    );
    return hasWrite ? "high" : "low";
  }
  if (shellScope.type === "cargo") {
    const hasPublish = (shellScope.operations ?? []).some(
      (op) => op === "publish"
    );
    return hasPublish ? "critical" : "medium";
  }
  return "medium";
}

function isGitWriteOperation(op: string): boolean {
  const writeOps = [
    "commit",
    "push",
    "reset",
    "merge",
    "rebase",
    "stash",
    "tag",
    "clone",
  ];
  return writeOps.includes(op);
}

function calculateNetworkRisk(method: string): RiskLevel {
  const safeMethods = ["GET", "HEAD", "OPTIONS"];
  const mediumMethods = ["POST", "PUT", "PATCH"];
  const upperMethod = method.toUpperCase();

  if (safeMethods.includes(upperMethod)) return "low";
  if (mediumMethods.includes(upperMethod)) return "medium";
  if (upperMethod === "DELETE") return "high";
  return "medium";
}

export function checkPolicyDecision(operation: Operation): PolicyDecision {
  const riskLevel = calculateRiskLevel(operation);
  return riskLevelToDecision(riskLevel);
}

export function riskLevelToDecision(risk: RiskLevel): PolicyDecision {
  switch (risk) {
    case "low":
      return "allow";
    case "medium":
    case "high":
      return "require_approval";
    case "critical":
      return "deny";
    default:
      return "require_approval";
  }
}

export function needsApproval(operation: Operation): boolean {
  const decision = checkPolicyDecision(operation);
  return decision === "require_approval";
}

export function isDenied(operation: Operation): boolean {
  const decision = checkPolicyDecision(operation);
  return decision === "deny";
}

export function isAllowed(operation: Operation): boolean {
  const decision = checkPolicyDecision(operation);
  return decision === "allow";
}

export function riskLevelToColor(risk: RiskLevel): string {
  switch (risk) {
    case "low":
      return "green";
    case "medium":
      return "yellow";
    case "high":
      return "red";
    case "critical":
      return "red";
    default:
      return "yellow";
  }
}

export function riskLevelToLabel(risk: RiskLevel): string {
  switch (risk) {
    case "low":
      return "低风险";
    case "medium":
      return "中风险";
    case "high":
      return "高风险";
    case "critical":
      return "禁止操作";
    default:
      return "未知风险";
  }
}

export function formatOperationDescription(operation: Operation): string {
  switch (operation.type) {
    case "filesystem":
      const fsScope = operation.scope as FsScope;
      const fsOp = operation.operation as FsOperation;
      const path = operation.path ?? "未知路径";
      return `文件操作: ${fsOp} (${fsScope}) - ${path}`;
    case "shell":
      const shellScope = operation.scope as { type: string };
      const command = operation.command ?? "未知命令";
      return `Shell: ${shellScope.type} - ${command}`;
    case "network":
      const endpoint = operation.endpoint ?? "未知端点";
      const method = operation.method ?? "GET";
      return `网络请求: ${method} ${endpoint}`;
    default:
      return "未知操作";
  }
}

export function formatOperationDetails(operation: Operation): string {
  const lines: string[] = [];

  lines.push(`类型: ${operation.type}`);

  if (operation.type === "filesystem") {
    lines.push(`范围: ${operation.scope}`);
    lines.push(`操作: ${operation.operation}`);
    lines.push(`路径: ${operation.path ?? "N/A"}`);
  } else if (operation.type === "shell") {
    const shellScope = operation.scope as { type: string; operations?: string[]; scripts?: string[] };
    lines.push(`Shell 类型: ${shellScope.type}`);
    if (shellScope.operations) {
      lines.push(`操作: ${shellScope.operations.join(", ")}`);
    }
    if (shellScope.scripts) {
      lines.push(`脚本: ${shellScope.scripts.join(", ")}`);
    }
    lines.push(`命令: ${operation.command ?? "N/A"}`);
    if (operation.args) {
      lines.push(`参数: ${operation.args.join(" ")}`);
    }
  } else if (operation.type === "network") {
    const netScope = operation.scope as { type: string; endpoint: { host: string; port: number | null } };
    lines.push(`范围: ${netScope.type}`);
    lines.push(`端点: ${netScope.endpoint.host}${netScope.endpoint.port ? `:${netScope.endpoint.port}` : ""}`);
    lines.push(`方法: ${operation.method ?? "GET"}`);
  }

  return lines.join("\n");
}