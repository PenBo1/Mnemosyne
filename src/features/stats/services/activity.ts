/**
 * ═══════════════════════════════════════════════════════════════════════════
 * Activity Service - 活跃度数据服务
 * ═══════════════════════════════════════════════════════════════════════════
 *
 * 封装 get_daily_activity IPC 调用，为组件提供清晰的 API。
 */

import { ipc } from "@/services/ipc";

// ── 类型定义 ────────────────────────────────────────────────────────────────

/**
 * 每日活跃度结果
 */
export interface DailyActivityResult {
  /** 日期（YYYY-MM-DD） */
  date: string;
  /** 消息数量 */
  messages: number;
  /** 会话数量 */
  sessions: number;
  /** Token数量 */
  tokens: number;
}

// ── 服务函数 ────────────────────────────────────────────────────────────────

/**
 * 获取每日活跃度数据
 *
 * 通过 IPC 调用 Rust 后端获取指定天数内的活跃度统计。
 *
 * @param days - 要获取的天数（默认365天）
 * @returns 每日活跃度数据数组
 * @throws 当获取失败时抛出错误
 *
 * @example
 * ```typescript
 * const activity = await getDailyActivity(30);
 * console.log(activity); // 最近30天的活跃度数据
 * ```
 */
export async function getDailyActivity(days: number = 365): Promise<DailyActivityResult[]> {
  return ipc<DailyActivityResult[]>("get_daily_activity", { days });
}