/**
 * Process Monitor 类型定义
 */

/** 进程类型 */
export type ProcessType = "main" | "renderer" | "gpu" | "utility" | "unknown";

/** 进程信息 */
export interface ProcessInfo {
  /** 进程名称 */
  name: string;
  /** 进程 ID */
  pid: number;
  /** CPU 使用率 (0.0 - 100.0) */
  cpuUsage: number;
  /** 内存使用量 (MB) */
  memoryMb: number;
  /** 进程类型分类 */
  processType: ProcessType;
}

/** 系统资源摘要 */
export interface SystemResourceSummary {
  /** 总 CPU 使用率 */
  totalCpuUsage: number;
  /** 总内存使用量 (MB) */
  totalMemoryMb: number;
  /** 可用内存 (MB) */
  totalMemoryAvailableMb: number;
  /** 进程数量 */
  processCount: number;
}