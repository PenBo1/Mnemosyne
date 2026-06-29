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
}
