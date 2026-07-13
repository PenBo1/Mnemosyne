// 会话生命周期规则 —— 纯逻辑内核
// 会话状态机：active/paused/completed/archived + 压缩触发 + 乐观更新回滚

export type SessionLifecycle = "active" | "paused" | "completed" | "archived";

/**
 * 判断是否应触发会话压缩（基于消息数/token 估算）
 */
export function shouldCompact(messageCount: number, estimatedTokens: number): boolean {
  return messageCount > 50 || estimatedTokens > 80000;
}

/**
 * 乐观更新回滚规则
 */
export function rollbackRule<T>(_optimistic: T, confirmed: T): T {
  return confirmed;
}
