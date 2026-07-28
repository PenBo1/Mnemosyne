// Custom Agents 类型 + 持久化。
//
// Agent 是 Persona 层：切换 system prompt + 指令风格。
// P1 阶段 3 Mnemosyne 适配：
// - 8-agent pipeline 的工具子集需求 → Agent 加 toolWhitelist? 字段
//   （8-agent 需要不同工具集，这是接入前提）
// - 多窗口同步：用 Tauri emit/listen 广播（暂缓，P2+ 再加）
// - 持久化：LazyStore mnemosyne-agents.json
// P2.8：toolWhitelist 已启用，BUILTIN_AGENTS 按角色配置工具子集，
//   chat-runtime.sendMessage 将白名单随 IPC request 传递给后端
//   （后端 SendMessageRequest 需新增 tool_whitelist 字段才能生效，当前 serde 忽略未知字段）。

import { LazyStore } from "@tauri-apps/plugin-store";
import type { Agent, AgentIconId } from "../types";

export type { Agent, AgentIconId };

export const BUILTIN_AGENTS: readonly Agent[] = [
  {
    id: "builtin:coder",
    name: "Coder",
    description: "通用编程助手",
    instructions: "你是编程助手，帮助用户写代码、调试、重构。",
    icon: "coder",
    builtIn: true,
    toolWhitelist: [
      "read_file",
      "list_directory",
      "write_file",
      "create_directory",
      "edit",
      "multi_edit",
      "todo_write",
    ],
  },
  {
    id: "builtin:architect",
    name: "Architect",
    description: "设计与架构权衡",
    instructions: "你关注系统设计、架构权衡、技术选型。",
    icon: "architect",
    builtIn: true,
    toolWhitelist: ["read_file", "list_directory", "todo_write"],
  },
  {
    id: "builtin:reviewer",
    name: "Code Reviewer",
    description: "代码评审",
    instructions: "你检查代码质量、潜在 bug、最佳实践。",
    icon: "reviewer",
    builtIn: true,
    toolWhitelist: ["read_file", "list_directory", "todo_write"],
  },
  {
    id: "builtin:security",
    name: "Security",
    description: "威胁建模与安全审计",
    instructions: "你扫描安全漏洞、注入、认证绕过、密钥泄露。",
    icon: "security",
    builtIn: true,
    toolWhitelist: ["read_file", "list_directory", "todo_write"],
  },
  {
    id: "builtin:designer",
    name: "Designer",
    description: "UI/UX 设计",
    instructions: "你关注用户体验、交互设计、视觉一致性。",
    icon: "designer",
    builtIn: true,
    toolWhitelist: [
      "read_file",
      "list_directory",
      "write_file",
      "create_directory",
      "todo_write",
    ],
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

/**
 * 获取指定 agent 的工具白名单（P2.8）。
 *
 * 查找顺序：builtin agents → custom agents。
 * - 命中 builtin：返回该 builtin 的 toolWhitelist（builtin 必有，coder/architect/...）
 * - 命中 custom：返回该 custom 的 toolWhitelist（custom 可选字段，可能为 undefined）
 * - 未命中：返回 undefined（语义上等同"无限制 / 全工具集"）
 *
 * 返回 undefined 即表示该 agent 不做工具过滤，应使用全工具集。
 */
export function getToolWhitelist(
  agentId: string,
  customAgents: readonly Agent[] = [],
): readonly string[] | undefined {
  const builtin = BUILTIN_AGENTS.find((a) => a.id === agentId);
  if (builtin) return builtin.toolWhitelist;
  const custom = customAgents.find((a) => a.id === agentId);
  if (custom) return custom.toolWhitelist;
  return undefined;
}

/**
 * 按白名单过滤工具列表（P2.8）。
 *
 * - whitelist === undefined：返回原列表（无限制，使用全工具集）
 * - whitelist 为空数组：返回空数组（禁用所有工具）
 * - whitelist 非空：仅保留同时存在于白名单中的工具（顺序与原列表一致）
 *
 * 注意：此函数为纯前端过滤。实际工具调用由 Rust agent engine 执行，
 * 后端需读取 IPC 请求中的 tool_whitelist 字段才能让限制真正生效
 * （当前后端 SendMessageRequest 未声明该字段，serde 忽略未知字段）。
 */
export function filterToolsByWhitelist(
  tools: readonly string[],
  whitelist: readonly string[] | undefined,
): readonly string[] {
  if (whitelist === undefined) return tools;
  const allowed = new Set(whitelist);
  return tools.filter((tool) => allowed.has(tool));
}
