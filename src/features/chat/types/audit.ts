// ── Audit ──────────────────────────────────────────────────

export interface AuditResult {
  passed: boolean;
  score: number;
  issues: AuditIssue[];
  summary: string;
}

export interface AuditIssue {
  severity: "critical" | "warning" | "info";
  category: string;
  description: string;
  suggestion: string;
  /** 修复范围提示 (前端 pipeline 用, Rust 侧可选)。 */
  repairScope?: "local" | "structural" | "unknown";
}
