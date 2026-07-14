// 学习偏好的前端类型 —— 对应 Rust 的 LearnedPreferenceRow
//
// 字段说明:
// - preferenceKey: 偏好键,如 "code_style" / "work_hours" / "favorite_tools"
// - preferenceValue: 偏好值,如 "concise" / "09-18" / "git/cargo"
// - confidence: 0.0-1.0,越高越可信(0.2=探索,0.5=确认,0.8=稳定)
// - occurrenceCount: 出现次数,每次相同偏好被识别 +1
// - learnedFrom: 来源标识,如 "session:abc123"
// - lastSeenAt / createdAt: ISO8601 时间戳

export interface LearnedPreferenceRow {
  id: string;
  preferenceKey: string;
  preferenceValue: string;
  /** 0.0-1.0,越高越可信 */
  confidence: number;
  occurrenceCount: number;
  learnedFrom: string | null;
  lastSeenAt: string;
  createdAt: string;
}

/** 置信度等级标签 */
export function confidenceLabel(confidence: number): string {
  if (confidence >= 0.8) return "稳定";
  if (confidence >= 0.5) return "确认";
  if (confidence >= 0.3) return "探索";
  return "微弱";
}

/** 置信度对应的颜色(用于 UI 徽章) */
export function confidenceColor(confidence: number): string {
  if (confidence >= 0.8) return "text-emerald-600";
  if (confidence >= 0.5) return "text-blue-600";
  if (confidence >= 0.3) return "text-amber-600";
  return "text-muted-foreground";
}
