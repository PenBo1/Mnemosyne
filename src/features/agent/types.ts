export interface Agent {
  id: string;
  name: string;
  description: string;
  builtIn?: boolean;
}

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