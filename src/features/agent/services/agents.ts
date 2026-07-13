// Custom Agents 类型 + 持久化。
//
// Agent 是 Persona 层：切换 system prompt + 指令风格。
// P1 阶段 3 Mnemosyne 适配：
// - 8-agent pipeline 的工具子集需求 → Agent 加 toolWhitelist? 字段
//   （8-agent 需要不同工具集，这是接入前提）
// - 多窗口同步：用 Tauri emit/listen 广播（暂缓，P2+ 再加）
// - 持久化：LazyStore mnemosyne-agents.json

import { LazyStore } from "@tauri-apps/plugin-store";

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
  /** 工具白名单。undefined = 全工具集；string[] = 仅这些工具。
   *  为 8-agent 不同工具集做准备。 */
  toolWhitelist?: string[];
};

export const BUILTIN_AGENTS: readonly Agent[] = [
  {
    id: "builtin:coder",
    name: "Coder",
    description: "通用编程助手",
    instructions: "你是编程助手，帮助用户写代码、调试、重构。",
    icon: "coder",
    builtIn: true,
  },
  {
    id: "builtin:architect",
    name: "Architect",
    description: "设计与架构权衡",
    instructions: "你关注系统设计、架构权衡、技术选型。",
    icon: "architect",
    builtIn: true,
  },
  {
    id: "builtin:reviewer",
    name: "Code Reviewer",
    description: "代码评审",
    instructions: "你检查代码质量、潜在 bug、最佳实践。",
    icon: "reviewer",
    builtIn: true,
  },
  {
    id: "builtin:security",
    name: "Security",
    description: "威胁建模与安全审计",
    instructions: "你扫描安全漏洞、注入、认证绕过、密钥泄露。",
    icon: "security",
    builtIn: true,
  },
  {
    id: "builtin:designer",
    name: "Designer",
    description: "UI/UX 设计",
    instructions: "你关注用户体验、交互设计、视觉一致性。",
    icon: "designer",
    builtIn: true,
  },
] as const;

const STORE_PATH = "mnemosyne-agents.json";
const KEY_CUSTOM = "customAgents";
const KEY_ACTIVE = "activeAgentId";
const store = new LazyStore(STORE_PATH, { defaults: {}, autoSave: 200 });

export type LoadedAgents = { custom: Agent[]; activeId: string };

/** 一次 IPC 读取 custom + activeId（避免两次 get）。 */
export async function loadAgents(): Promise<LoadedAgents> {
  const entries = await store.entries();
  let custom: Agent[] | undefined;
  let activeId: string | undefined;
  for (const [k, v] of entries) {
    if (k === KEY_CUSTOM) custom = v as Agent[];
    else if (k === KEY_ACTIVE) activeId = v as string;
  }
  return { custom: custom ?? [], activeId: activeId ?? BUILTIN_AGENTS[0].id };
}

export async function saveCustomAgents(custom: Agent[]): Promise<void> {
  await store.set(KEY_CUSTOM, custom);
  await store.save();
}

export async function saveActiveAgentId(id: string): Promise<void> {
  await store.set(KEY_ACTIVE, id);
  await store.save();
}

export function newAgentId(): string {
  return `a-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

/** 查找 agent（builtin + custom）。找不到回退到第一个 builtin。 */
export function findAgent(agents: Agent[], id: string): Agent {
  return agents.find((a) => a.id === id) ?? BUILTIN_AGENTS[0];
}
