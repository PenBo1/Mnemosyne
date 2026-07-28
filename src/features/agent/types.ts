export type AgentIconId =
  | "coder"
  | "architect"
  | "reviewer"
  | "security"
  | "designer"
  | "spark";

export type Agent = {
  id: string;
  name: string;
  description: string;
  instructions: string;
  icon: AgentIconId;
  builtIn: boolean;
  /** 工具白名单。undefined = 全工具集；readonly string[] = 仅这些工具。
   *  按 agent 角色限定可用工具集（P2.8 启用）。 */
  toolWhitelist?: readonly string[];
};

export interface AgentState {
  id: string;
  status: "idle" | "running" | "paused" | "error";
  lastActivity?: string;
}

export interface AgentConfig {
  model?: string;
  temperature?: number;
  maxTokens?: number;
}