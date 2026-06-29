// ── Agent ──────────────────────────────────────────────────
//
// 注意：Session / Message / AgentEvent 已定义在 ./session（barrel 导出）。
// 此处仅保留 Agent 与 SendMessageParams。

export interface Agent {
  id: string;
  name: string;
  description: string;
  model: string;
  systemPrompt: string;
  temperature: number;
  maxTokens: number;
  status: "active" | "inactive";
  created_at: string;
}

// ── Agent IPC params ───────────────────────────────────────

export interface SendMessageParams {
  session_id: string;
  content: string;
  [key: string]: unknown;
}
