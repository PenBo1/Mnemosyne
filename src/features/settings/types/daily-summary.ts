// 每日摘要任务的前端类型 —— 对应 Rust 的 DailySummaryConfig / DailySummaryReport
//
// 字段说明:
// - intervalMs: 触发间隔(毫秒),默认 24 小时
// - staleCutoffDays: learned_preferences 衰减 cutoff(天),默认 30
// - lookbackDays: 短期记忆回看窗口(天),默认 1
// - enabled: 是否启用定时任务

export interface DailySummaryConfig {
  /** 触发间隔(毫秒),范围:60000(1分钟) - 604800000(7天) */
  intervalMs: number;
  /** 衰减 cutoff(天),范围:1-365 */
  staleCutoffDays: number;
  /** 短期记忆回看窗口(天),范围:1-30 */
  lookbackDays: number;
  /** 是否启用定时任务 */
  enabled: boolean;
}

/** 摘要任务执行结果 */
export interface DailySummaryReport {
  /** 审查的短期记忆条数 */
  reviewedShortTerm: number;
  /** 衰减的学习偏好数量 */
  decayedPreferences: number;
  /** 更新的 agent role 列表 */
  updatedRoles: string[];
  /** 执行耗时(毫秒) */
  durationMs: number;
  /** 执行过程中的错误(不阻塞任务) */
  errors: string[];
}

/** 默认配置(对齐 Rust Default) */
export const DEFAULT_DAILY_SUMMARY_CONFIG: DailySummaryConfig = {
  intervalMs: 24 * 60 * 60 * 1000, // 24h
  staleCutoffDays: 30,
  lookbackDays: 1,
  enabled: false, // 默认不启动,需用户主动开启
};

/** 预设间隔选项(毫秒 → 可读标签) */
export const INTERVAL_PRESETS: Array<{ value: number; label: string }> = [
  { value: 60 * 60 * 1000, label: "每小时" },
  { value: 6 * 60 * 60 * 1000, label: "每 6 小时" },
  { value: 12 * 60 * 60 * 1000, label: "每 12 小时" },
  { value: 24 * 60 * 60 * 1000, label: "每天" },
  { value: 7 * 24 * 60 * 60 * 1000, label: "每周" },
];

/** 格式化间隔为可读文本 */
export function formatInterval(ms: number): string {
  const hours = ms / (60 * 60 * 1000);
  if (hours < 1) return `${ms / 60000} 分钟`;
  if (hours < 24) return `${hours} 小时`;
  const days = hours / 24;
  return `${days} 天`;
}

/** 格式化耗时为可读文本 */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} 秒`;
  const minutes = Math.floor(seconds / 60);
  const remSeconds = Math.round(seconds % 60);
  return `${minutes}m ${remSeconds}s`;
}
