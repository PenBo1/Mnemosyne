import { FailurePattern, type FailureReport, type AgentStep, type ToolRegistry } from "./types";

interface ToolSchema {
  name: string;
  requiredParams: string[];
  optionalParams: string[];
  paramTypes: Record<string, string>;
}

const TOOL_SCHEMAS: Record<string, ToolSchema> = {
  fs_read_file: {
    name: "fs_read_file",
    requiredParams: ["path"],
    optionalParams: ["encoding"],
    paramTypes: { path: "string", encoding: "string" },
  },
  fs_write_file: {
    name: "fs_write_file",
    requiredParams: ["path", "content"],
    optionalParams: ["encoding"],
    paramTypes: { path: "string", content: "string", encoding: "string" },
  },
  fs_list_dir: {
    name: "fs_list_dir",
    requiredParams: ["path"],
    optionalParams: [],
    paramTypes: { path: "string" },
  },
  shell_exec: {
    name: "shell_exec",
    requiredParams: ["command"],
    optionalParams: ["args", "cwd", "timeout"],
    paramTypes: { command: "string", args: "array", cwd: "string", timeout: "number" },
  },
  http_request: {
    name: "http_request",
    requiredParams: ["url"],
    optionalParams: ["method", "headers", "body", "timeout"],
    paramTypes: { url: "string", method: "string", headers: "object", body: "string", timeout: "number" },
  },
};

const TOOL_ALIASES: Record<string, string> = {
  read_file: "fs_read_file",
  write_to_file: "fs_write_file",
  list_files: "fs_list_dir",
  execute_command: "shell_exec",
  browser_search: "http_request",
};

const TOOL_ALTERNATIVES: Record<string, string[]> = {
  fs_read_file: ["fs_list_dir + path inference", "search_files with pattern"],
  fs_write_file: ["fs_read_file + modify", "fs_copy + modify"],
  shell_exec: ["fs_read_file + fs_write_file (if editing files)"],
  http_request: ["browser_search (if searching)"],
};

export class ToolMisuseDetector {
  private toolSchemas: Map<string, ToolSchema>;

  constructor(_toolRegistry?: ToolRegistry) {
    this.toolSchemas = new Map(Object.entries(TOOL_SCHEMAS));
  }

  registerSchema(schema: ToolSchema): void {
    this.toolSchemas.set(schema.name, schema);
  }

  detect(step: AgentStep): FailureReport | null {
    if (step.type !== "tool_call" || !step.toolName) {
      return null;
    }

    const paramValidation = this.validateParameters(step.toolName, step.toolArgs);
    if (paramValidation) {
      return paramValidation;
    }

    const semanticValidation = this.validateSemantics(step);
    if (semanticValidation) {
      return semanticValidation;
    }

    return null;
  }

  private validateParameters(
    toolName: string,
    args?: Record<string, unknown>
  ): FailureReport | null {
    const canonicalName = TOOL_ALIASES[toolName] ?? toolName;
    const schema = this.toolSchemas.get(canonicalName);

    if (!schema) {
      return null;
    }

    const missingParams: string[] = [];
    const typeErrors: string[] = [];

    for (const required of schema.requiredParams) {
      if (!args || !(required in args) || args[required] === undefined) {
        missingParams.push(required);
      }
    }

    if (args) {
      for (const [key, value] of Object.entries(args)) {
        const expectedType = schema.paramTypes[key];
        if (expectedType && !this.validateType(value, expectedType)) {
          typeErrors.push(`${key}: expected ${expectedType}, got ${typeof value}`);
        }
      }
    }

    if (missingParams.length > 0) {
      return {
        pattern: FailurePattern.ToolMisuse,
        severity: "error",
        message: `Tool '${toolName}' missing required parameters: ${missingParams.join(", ")}`,
        suggestion: "Please provide all required parameters and retry.",
        timestamp: Date.now(),
        metadata: {
          toolName,
          missingParams,
          schema: schema.name,
          i18nKey: "failureDetection.toolMisuseMissingParams",
          i18nParams: { tool: toolName, params: missingParams.join(", ") },
        },
      };
    }

    if (typeErrors.length > 0) {
      return {
        pattern: FailurePattern.ToolMisuse,
        severity: "error",
        message: `Tool '${toolName}' parameter type error: ${typeErrors.join("; ")}`,
        suggestion: "Please check parameter types and correct before retrying.",
        timestamp: Date.now(),
        metadata: {
          toolName,
          typeErrors,
          i18nKey: "failureDetection.toolMisuseTypeError",
          i18nParams: { tool: toolName, errors: typeErrors.join("; ") },
        },
      };
    }

    return null;
  }

  private validateSemantics(step: AgentStep): FailureReport | null {
    if (!step.toolName || !step.toolArgs) {
      return null;
    }

    if (step.toolName.includes("write") || step.toolName.includes("delete")) {
      const path = this.extractPath(step.toolArgs);
      if (path && this.isCriticalPath(path)) {
        return {
          pattern: FailurePattern.ToolMisuse,
          severity: "warning",
          message: `Tool '${step.toolName}' is modifying critical path: ${path}`,
          suggestion: "Modifying this file may affect system stability. Please confirm the operation.",
          timestamp: Date.now(),
          metadata: {
            toolName: step.toolName,
            path,
            i18nKey: "failureDetection.toolMisuseCriticalPath",
            i18nParams: { tool: step.toolName, path },
          },
        };
      }
    }

    if (step.toolName.includes("shell") && step.toolArgs) {
      const command = step.toolArgs.command as string | undefined;
      if (command && this.isDangerousCommand(command)) {
        return {
          pattern: FailurePattern.ToolMisuse,
          severity: "warning",
          message: `Detected dangerous command: ${command}`,
          suggestion: "This command has potential risks. Consider testing in a sandbox environment first.",
          timestamp: Date.now(),
          metadata: {
            toolName: step.toolName,
            command,
            i18nKey: "failureDetection.toolMisuseDangerousCommand",
            i18nParams: { command },
          },
        };
      }
    }

    return null;
  }

  private validateType(value: unknown, expectedType: string): boolean {
    switch (expectedType) {
      case "string":
        return typeof value === "string";
      case "number":
        return typeof value === "number";
      case "boolean":
        return typeof value === "boolean";
      case "array":
        return Array.isArray(value);
      case "object":
        return typeof value === "object" && value !== null && !Array.isArray(value);
      default:
        return true;
    }
  }

  private extractPath(args: Record<string, unknown>): string | null {
    if (typeof args.path === "string") return args.path;
    if (typeof args.file_path === "string") return args.file_path;
    if (typeof args.file === "string") return args.file;
    return null;
  }

  private isCriticalPath(path: string): boolean {
    const criticalPatterns = [
      /package\.json$/,
      /tsconfig\.json$/,
      /\.env$/,
      /\.git\//,
      /node_modules\//,
      /src-tauri\/Cargo\.toml$/,
    ];

    return criticalPatterns.some((pattern) => pattern.test(path));
  }

  private isDangerousCommand(command: string): boolean {
    const dangerousPatterns = [
      /\brm\s+-rf\b/,
      /\bdd\s+if=/,
      />\s*\/dev\//,
      /\bsudo\s+/,
      /\bchmod\s+777\b/,
      /\bcurl\s+.*\|\s*bash\b/,
      /\bwget\s+.*\|\s*bash\b/,
    ];

    return dangerousPatterns.some((pattern) => pattern.test(command));
  }

  getAlternatives(toolName: string): string[] {
    return TOOL_ALTERNATIVES[toolName] ?? [];
  }
}