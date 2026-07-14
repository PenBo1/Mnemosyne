// Skill 进化系统的前端 services —— 包装 skill_usage_* / skill_candidate_* IPC 命令。
//
// 调用约定:前端使用 camelCase 参数,Tauri 自动转换为 snake_case。

import { ipc, ipcVoid } from "@/services/ipc";
import type {
  SkillCandidate,
  SkillCandidateStatus,
  SkillUsageStats,
} from "@/features/skill/types/skill-evolution";

// ── usage 命令 ────────────────────────────────────────────────

/** 列出所有 skill 的使用统计(按 used_count 降序) */
export async function listSkillUsageStats(): Promise<SkillUsageStats[]> {
  return ipc<SkillUsageStats[]>("skill_usage_list", {});
}

/** 获取单个 skill 的使用统计 */
export async function getSkillUsageStats(
  skillName: string,
): Promise<SkillUsageStats | null> {
  return ipc<SkillUsageStats | null>("skill_usage_get", { skillName });
}

/** 手动记录一次 skill 使用(前端调试或外部集成用) */
export async function recordSkillUsage(
  skillName: string,
  success: boolean,
  sessionId?: string,
): Promise<void> {
  await ipcVoid("skill_usage_record", {
    skillName,
    success,
    sessionId: sessionId ?? null,
  });
}

/** 调整用户反馈分(±1.0) */
export async function adjustSkillFeedback(
  skillName: string,
  delta: number,
): Promise<boolean> {
  return ipc<boolean>("skill_usage_feedback", { skillName, delta });
}

// ── candidate 命令 ────────────────────────────────────────────

/** 列出 skill 候选(可按 status 过滤) */
export async function listSkillCandidates(
  status?: SkillCandidateStatus,
): Promise<SkillCandidate[]> {
  return ipc<SkillCandidate[]>("skill_candidate_list", {
    status: status ?? null,
  });
}

/** 批准一个 pending 候选 */
export async function approveSkillCandidate(
  candidateId: string,
  reviewerNotes?: string,
): Promise<boolean> {
  return ipc<boolean>("skill_candidate_approve", {
    candidateId,
    reviewerNotes: reviewerNotes ?? null,
  });
}

/** 拒绝一个 pending 候选 */
export async function rejectSkillCandidate(
  candidateId: string,
  reviewerNotes?: string,
): Promise<boolean> {
  return ipc<boolean>("skill_candidate_reject", {
    candidateId,
    reviewerNotes: reviewerNotes ?? null,
  });
}

/** 删除一个候选 */
export async function deleteSkillCandidate(candidateId: string): Promise<boolean> {
  return ipc<boolean>("skill_candidate_delete", { candidateId });
}
