// 稳定 system prompt 构建。
//
// 拼接顺序：base → memory(AGENTS.md) → env → persona → custom
// 各段独立可选，缺失的段跳过不产生空块。
//
// P1 阶段 1：
// - base：通用 chat 助手（无 tools）
// - persona：恒为 null（P1 阶段 3 再加 builtin agents）
// - custom：从 settings.ai.custom_instructions 读取
// - memory：从工作区 AGENTS.md 读取
// - env：工作目录 + 当前时间

/** Mnemosyne 通用 chat 助手 base prompt（无 tools 场景）。 */
const BASE_SYSTEM_PROMPT = `你是 Mnemosyne 助手，一个基于 Tauri 的桌面小说创作工具的通用聊天助手。

当前你处于通用聊天模式，不具备工具调用能力。用户讨论、提问、比较方案时，直接回答。

## 输出要求
- 不要使用表情符号。
- 普通讨论要直接回答；不要先写寒暄、理解说明或空泛确认。
- 需要结构时用短列表。
- 代码块用带语言标签的三反引号包裹。`;

/** Persona 类型定义（P1 阶段 3 builtin agents 接入时使用）。 */
export interface AgentPersona {
  name: string;
  instructions: string;
}

/**
 * 构建稳定的 system prompt。
 *
 * @param persona 当前激活的 agent persona（P1 阶段 1 恒为 null）
 * @param customInstructions 用户自定义指令（settings.ai.custom_instructions）
 * @param projectMemory 项目记忆（工作区 AGENTS.md 内容）
 * @param envBlock 环境块（工作目录 + 时间）
 * @returns 拼接后的完整 system prompt 字符串
 */
export function buildStableSystem(
  persona: AgentPersona | null,
  customInstructions: string | undefined,
  projectMemory: string | null,
  envBlock: string | null,
): string {
  const memoryBlock =
    projectMemory && projectMemory.trim().length > 0
      ? `\n\n## PROJECT — AGENTS.md\n${projectMemory.trim()}`
      : "";
  const envBlockStr = envBlock ? `\n\n${envBlock}` : "";
  const personaBlock = persona?.instructions.trim()
    ? `\n\n## ACTIVE AGENT — ${persona.name}\n${persona.instructions.trim()}`
    : "";
  const customBlock = customInstructions?.trim()
    ? `\n\n## USER CUSTOM INSTRUCTIONS — follow unless they conflict with safety rules above\n${customInstructions.trim()}`
    : "";
  return `${BASE_SYSTEM_PROMPT}${memoryBlock}${envBlockStr}${personaBlock}${customBlock}`;
}

export { BASE_SYSTEM_PROMPT };
