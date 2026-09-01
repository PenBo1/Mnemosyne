/**
 * ═══════════════════════════════════════════════════════════════════════════
 * AgentStatusIndicator - Agent 执行状态指示器组件
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { memo } from "react";
import { Loader2, CheckCircle2, Clock, AlertCircle, PauseCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { useI18n } from "@/locales/i18n";

// ── 类型定义 ────────────────────────────────────────────────────────────────

/** Agent 工作状态 */
export type AgentStatus =
  | "working"    // 正在执行任务
  | "waiting"    // 等待用户输入/审批
  | "idle"       // 空闲，无活动任务
  | "stale"      // 有待处理的工具调用
  | "error"      // 执行出错
  | "completed"; // 任务完成

/** 状态指示器 Props */
interface AgentStatusIndicatorProps {
  status: AgentStatus;
  message?: string;
  size?: "sm" | "md" | "lg";
  showLabel?: boolean;
  className?: string;
}

// ── 状态配置 ────────────────────────────────────────────────────────────────

/** 状态配置映射 */
const STATUS_CONFIG: Record<AgentStatus, {
  icon: typeof Loader2;
  colorClass: string;
  bgClass: string;
  animate?: boolean;
}> = {
  working: {
    icon: Loader2,
    colorClass: "text-blue-500",
    bgClass: "bg-blue-500/10",
    animate: true,
  },
  waiting: {
    icon: Clock,
    colorClass: "text-amber-500",
    bgClass: "bg-amber-500/10",
  },
  idle: {
    icon: PauseCircle,
    colorClass: "text-muted-foreground",
    bgClass: "bg-muted/50",
  },
  stale: {
    icon: AlertCircle,
    colorClass: "text-rose-500",
    bgClass: "bg-rose-500/10",
  },
  error: {
    icon: AlertCircle,
    colorClass: "text-destructive",
    bgClass: "bg-destructive/10",
  },
  completed: {
    icon: CheckCircle2,
    colorClass: "text-emerald-500",
    bgClass: "bg-emerald-500/10",
  },
};

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/**
 * 根据活动时间和状态计算 Agent 当前工作状态
 * @param lastActivityAt - 最后活动时间戳（毫秒）
 * @param streaming - 是否正在流式输出
 * @param pendingToolCall - 是否有待处理的工具调用
 * @param hasError - 是否有错误
 * @returns Agent 工作状态
 */
export function computeAgentStatus(params: {
  lastActivityAt: number | null;
  streaming: boolean;
  pendingToolCall: boolean;
  hasError: boolean;
}): AgentStatus {
  const { lastActivityAt, streaming, pendingToolCall, hasError } = params;

  // 有错误
  if (hasError) return "error";

  // 正在流式输出
  if (streaming) return "working";

  // 有待处理的工具调用（超过 60 秒未响应）
  if (pendingToolCall) {
    if (lastActivityAt) {
      const ageMs = Date.now() - lastActivityAt;
      const STALE_THRESHOLD_MS = 60 * 1000; // 60 秒
      if (ageMs > STALE_THRESHOLD_MS) return "stale";
    }
    return "waiting";
  }

  // 无活动记录
  if (!lastActivityAt) return "idle";

  // 根据最后活动时间判断
  const ageMs = Date.now() - lastActivityAt;
  const FRESH_THRESHOLD_MS = 30 * 1000; // 30 秒内视为刚完成
  const IDLE_THRESHOLD_MS = 5 * 60 * 1000; // 5 分钟内视为空闲

  if (ageMs < FRESH_THRESHOLD_MS) return "completed";
  if (ageMs < IDLE_THRESHOLD_MS) return "idle";

  return "idle";
}

// ── 主组件 ──────────────────────────────────────────────────────────────────

/**
 * Agent 状态指示器，显示当前工作状态
 */
export const AgentStatusIndicator = memo(function AgentStatusIndicator({
  status,
  message,
  size = "md",
  showLabel = true,
  className,
}: AgentStatusIndicatorProps) {
  const { t } = useI18n();
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;

  const sizeClasses = {
    sm: "size-3",
    md: "size-4",
    lg: "size-5",
  };

  const labelMap: Record<AgentStatus, string> = {
    working: t.agentChat.statusWorking || "Working",
    waiting: t.agentChat.statusWaiting || "Waiting",
    idle: t.agentChat.statusIdle || "Idle",
    stale: t.agentChat.statusStale || "Stale",
    error: t.agentChat.statusError || "Error",
    completed: t.agentChat.statusCompleted || "Completed",
  };

  return (
    <div className={cn("flex items-center gap-1.5", className)}>
      <Badge
        variant="outline"
        className={cn("gap-1 px-1.5 py-0.5 text-xs", config.bgClass)}
      >
        <Icon
          className={cn(
            sizeClasses[size],
            config.colorClass,
            config.animate && "animate-spin"
          )}
        />
        {showLabel && (
          <span className={config.colorClass}>{labelMap[status]}</span>
        )}
      </Badge>
      {message && (
        <span className="text-xs text-muted-foreground truncate max-w-[200px]">
          {message}
        </span>
      )}
    </div>
  );
});

// ── 简化版本 ────────────────────────────────────────────────────────────────

/**
 * 简单的状态徽章，仅显示状态图标和颜色
 */
export const AgentStatusBadge = memo(function AgentStatusBadge({
  status,
  className,
}: {
  status: AgentStatus;
  className?: string;
}) {
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;

  return (
    <Badge
      variant="outline"
      className={cn("p-1", config.bgClass, className)}
    >
      <Icon
        className={cn(
          "size-3",
          config.colorClass,
          config.animate && "animate-spin"
        )}
      />
    </Badge>
  );
});