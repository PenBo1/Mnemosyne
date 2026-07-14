// Skill 进化系统的前端类型 —— 对齐 Rust 端 SkillUsageStatsRow / SkillCandidateRow。
//
// 设计:JSON DTO 直接映射,保持字段名一致(camelCase 由 Tauri 自动转换)。

/** Skill 成熟度等级 */
export type SkillMaturity = "emerging" | "developing" | "mature" | "deprecated";

/** Skill 使用统计行 */
export interface SkillUsageStats {
  skillName: string;
  usedCount: number;
  successCount: number;
  failureCount: number;
  userFeedbackScore: number;
  firstUsedAt: string;
  lastUsedAt: string;
  lastSessionId: string | null;
}

/** Skill 候选状态 */
export type SkillCandidateStatus = "pending" | "approved" | "rejected" | "superseded";

/** Skill 候选行 */
export interface SkillCandidate {
  id: string;
  candidateName: string;
  candidateDescription: string;
  sourceSessionId: string;
  sourceSummary: string;
  candidateContent: string;
  status: SkillCandidateStatus;
  reviewerNotes: string | null;
  reviewedAt: string | null;
  createdAt: string;
}

/** 计算成熟度(前端版本,与 Rust 端规则保持一致) */
export function calculateMaturity(stats: Pick<SkillUsageStats, "usedCount" | "lastUsedAt">): SkillMaturity {
  // 1. 先判断是否过期(90 天)
  try {
    const last = new Date(stats.lastUsedAt);
    const now = new Date();
    const daysDiff = (now.getTime() - last.getTime()) / (1000 * 60 * 60 * 24);
    if (daysDiff > 90) return "deprecated";
  } catch {
    // 解析失败,忽略
  }
  // 2. 按 used_count
  if (stats.usedCount <= 2) return "emerging";
  if (stats.usedCount <= 9) return "developing";
  return "mature";
}

/** 成功率(0-1) */
export function successRate(stats: Pick<SkillUsageStats, "usedCount" | "successCount">): number {
  if (stats.usedCount === 0) return 0;
  return stats.successCount / stats.usedCount;
}

/** 成熟度的中文标签 */
export function maturityLabel(maturity: SkillMaturity): string {
  switch (maturity) {
    case "emerging":
      return "新生";
    case "developing":
      return "开发中";
    case "mature":
      return "成熟";
    case "deprecated":
      return "已弃用";
  }
}

/** 成熟度对应的颜色类 */
export function maturityColor(maturity: SkillMaturity): string {
  switch (maturity) {
    case "emerging":
      return "text-blue-500";
    case "developing":
      return "text-amber-500";
    case "mature":
      return "text-emerald-500";
    case "deprecated":
      return "text-muted-foreground";
  }
}

/** 候选状态的中文标签 */
export function candidateStatusLabel(status: SkillCandidateStatus): string {
  switch (status) {
    case "pending":
      return "待审核";
    case "approved":
      return "已批准";
    case "rejected":
      return "已拒绝";
    case "superseded":
      return "已取代";
  }
}

/** 候选状态对应的颜色类 */
export function candidateStatusColor(status: SkillCandidateStatus): string {
  switch (status) {
    case "pending":
      return "text-amber-500";
    case "approved":
      return "text-emerald-500";
    case "rejected":
      return "text-red-500";
    case "superseded":
      return "text-muted-foreground";
  }
}
