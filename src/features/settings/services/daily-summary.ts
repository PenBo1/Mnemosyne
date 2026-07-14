// 每日摘要任务服务 —— 暴露 DailySummaryTask 的控制能力给前端。
//
// 后端命令:
// - daily_summary_trigger: 手动触发一次摘要生成
// - daily_summary_get_config: 读取当前配置
// - daily_summary_update_config: 更新配置(运行时生效)
// - daily_summary_start: 启动定时任务
// - daily_summary_stop: 停止定时任务
// - daily_summary_is_running: 查询任务是否在运行

import { ipc, ipcVoid } from "@/services/ipc";
import type {
  DailySummaryConfig,
  DailySummaryReport,
} from "@/features/settings/types/daily-summary";

/** 手动触发一次摘要生成(立即执行,不影响定时调度) */
export async function triggerDailySummary(): Promise<DailySummaryReport> {
  return ipc<DailySummaryReport>("daily_summary_trigger", {});
}

/** 读取当前配置 */
export async function getDailySummaryConfig(): Promise<DailySummaryConfig> {
  return ipc<DailySummaryConfig>("daily_summary_get_config", {});
}

/** 更新配置(运行时生效,无需重启) */
export async function updateDailySummaryConfig(
  config: DailySummaryConfig,
): Promise<DailySummaryConfig> {
  return ipc<DailySummaryConfig>("daily_summary_update_config", { config });
}

/** 启动定时任务 */
export async function startDailySummary(): Promise<boolean> {
  return ipcVoid("daily_summary_start", {}).then(() => true);
}

/** 停止定时任务 */
export async function stopDailySummary(): Promise<boolean> {
  return ipcVoid("daily_summary_stop", {}).then(() => true);
}

/** 查询任务是否在运行 */
export async function isDailySummaryRunning(): Promise<boolean> {
  return ipc<boolean>("daily_summary_is_running", {});
}
