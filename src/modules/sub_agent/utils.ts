import type { SubAgentRole, SubAgentStatus } from "@/shared/types";

/**
 * 子 Agent 特性内部工具：相对时间格式化、角色/状态视觉映射。
 *
 * 不放到 `shared/utils` — 这些是 sub_agent 特性专属的呈现逻辑，
 * 非跨模块通用能力。
 */

/** 将 ISO 8601 时间字符串格式化为相对时间（如 "3m ago"）。 */
export function formatRelativeTime(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = Date.now();
    const diffMs = now - date.getTime();
    const seconds = Math.floor(diffMs / 1000);
    if (seconds < 0) return dateStr;
    if (seconds < 60) return "just now";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months}mo ago`;
    const years = Math.floor(months / 12);
    return `${years}y ago`;
  } catch {
    return dateStr;
  }
}

/** 将毫秒耗时格式化为 "1.2s" / "3m 21s" / "1h 5m"。 */
export function formatDuration(durationMs: number): string {
  if (durationMs < 1000) return `${durationMs}ms`;
  const totalSeconds = Math.floor(durationMs / 1000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
}

/** 角色 → 图标容器前景色 CSS variable。 */
export function roleIconClass(role: SubAgentRole): string {
  switch (role) {
    case "Researcher":
      return "text-[var(--status-primary-default)]";
    case "Outliner":
      return "text-[var(--status-success-default)]";
    case "Critic":
      return "text-[var(--status-warning-default)]";
    case "Default":
    default:
      return "text-[var(--text-tertiary)]";
  }
}

/** 角色 → i18n key 后缀（researcher/outliner/critic/default）。 */
export function roleI18nKey(role: SubAgentRole): string {
  return role.toLowerCase();
}

/** 状态 → Badge variant（对应 badgeVariants）。 */
export function statusBadgeVariant(
  status: SubAgentStatus
): "secondary" | "info" | "success" | "destructive" {
  switch (status) {
    case "Running":
      return "info";
    case "Completed":
      return "success";
    case "Errored":
      return "destructive";
    case "Pending":
    case "Cancelled":
    default:
      return "secondary";
  }
}

/** 状态 → i18n key 后缀（pending/running/completed/errored/cancelled）。 */
export function statusI18nKey(status: SubAgentStatus): string {
  return status.toLowerCase();
}
