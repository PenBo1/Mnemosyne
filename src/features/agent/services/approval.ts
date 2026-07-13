// Approval 协调器 — 通过 Rust 后端 IPC 处理工具确认。
//
// 新流程：
// 1. Rust Agent 调用需要审批的工具 → 发送 ToolApprovalRequired 事件
// 2. 前端显示 ApprovalCard
// 3. 用户点击 Approve/Deny → 调用 respondApproval
// 4. respondApproval 调用 chat_tool_respond IPC → Rust 继续执行

import { invoke } from "@tauri-apps/api/core";
import { useAgentStore } from "@/features/chat/store";

/**
 * 响应 approval — useChat 在 UI Approve/Deny 按钮回调中调用。
 * 通过 IPC 通知 Rust 后端用户决定。
 */
export async function respondApproval(
  sessionId: string,
  requestId: string,
  approved: boolean,
): Promise<void> {
  try {
    await invoke("chat_tool_respond", {
      request: { sessionId, requestId, approved },
    });
  } catch {
    // Ignore errors — tool may have already timed out
  }
  // Clear the pending confirmation in the store
  useAgentStore.getState().setPendingConfirmation(null);
  useAgentStore.getState().setSubmittingConfirmation(false);
}
