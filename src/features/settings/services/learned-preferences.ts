// 学习偏好服务 —— 暴露 learned_preferences 表的查询能力给前端。
//
// 后端命令:
// - learned_preferences_list: 列出所有偏好(按 confidence 降序)
// - learned_preferences_list_by_key: 按 key 列出
// - learned_preferences_list_high_confidence: 列出高置信度偏好
// - learned_preferences_delete: 删除指定偏好
// - learned_preferences_analyze: 主动分析 session 的用户偏好
// - learned_preferences_decay_stale: 衰减长期未观察的偏好(定时任务用)

import { ipc, ipcVoid } from "@/services/ipc";
import type { LearnedPreferenceRow } from "@/features/settings/types/learned-preferences";

/** 列出所有学习到的偏好(按 confidence 降序) */
export async function listLearnedPreferences(): Promise<LearnedPreferenceRow[]> {
  return ipc<LearnedPreferenceRow[]>("learned_preferences_list", {});
}

/** 按 preference_key 列出偏好(同一 key 可能有多个 value) */
export async function listLearnedPreferencesByKey(key: string): Promise<LearnedPreferenceRow[]> {
  return ipc<LearnedPreferenceRow[]>("learned_preferences_list_by_key", { key });
}

/** 列出高置信度偏好(>= threshold,默认 0.7) */
export async function listHighConfidencePreferences(
  threshold = 0.7,
): Promise<LearnedPreferenceRow[]> {
  return ipc<LearnedPreferenceRow[]>("learned_preferences_list_high_confidence", { threshold });
}

/** 删除指定偏好(用户主动否认) */
export async function deleteLearnedPreference(id: string): Promise<boolean> {
  return ipcVoid("learned_preferences_delete", { id }).then(() => true);
}

/** 主动分析 session 的用户偏好(用户点击"分析我的偏好"时触发) */
export async function analyzeLearnedPreferences(sessionId: string): Promise<number> {
  return ipc<number>("learned_preferences_analyze", { sessionId });
}

/** 衰减长期未观察的偏好(定时任务调用) */
export async function decayStalePreferences(cutoffDays = 30): Promise<number> {
  return ipc<number>("learned_preferences_decay_stale", { cutoffDays });
}
