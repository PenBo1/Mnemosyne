// Subagent Registry —— 子 Agent 注册表，管理 Pipeline 子 Agent 的元数据。
//
// 功能:
// - 定义 SubagentType 枚举（与 Rust 端 pipeline agents 对齐）
// - 注册子 Agent 的工具白名单
// - 提供查询接口（get/list）
//
// 子 Agent 类型:
// - Pipeline agents: architect, planner, composer, writer, auditor, reviser, observer, settler
// - Review agents: code_reviewer
// - Research agents: deep_researcher

export type SubagentType =
  | "architect"
  | "planner"
  | "composer"
  | "writer"
  | "auditor"
  | "reviser"
  | "observer"
  | "settler"
  | "code_reviewer"
  | "deep_researcher";

export interface SubagentDescriptor {
  id: SubagentType;
  name: string;
  nameZh: string;
  description: string;
  descriptionZh: string;
  toolWhitelist: readonly string[];
  modelOverride?: string;
}

const SUBAGENT_REGISTRY: Record<SubagentType, SubagentDescriptor> = {
  architect: {
    id: "architect",
    name: "Architect",
    nameZh: "架构师",
    description: "Creates book structure and initial settings",
    descriptionZh: "构建小说整体结构，产出卷/章结构与初始设定",
    toolWhitelist: ["read_file", "list_directory"],
  },
  planner: {
    id: "planner",
    name: "Planner",
    nameZh: "规划师",
    description: "Generates chapter memo for each chapter",
    descriptionZh: "为每一章生成章节备忘录",
    toolWhitelist: ["read_file", "list_directory"],
  },
  composer: {
    id: "composer",
    name: "Composer",
    nameZh: "编排师",
    description: "Assembles context for writer (semantic section selection + compressible context compilation)",
    descriptionZh: "为 Writer 组装上下文（语义选段 + 可压缩上下文编译）",
    toolWhitelist: ["read_file"],
  },
  writer: {
    id: "writer",
    name: "Writer",
    nameZh: "写作者",
    description: "Generates chapter prose based on chapter memo",
    descriptionZh: "根据章节备忘录生成章节正文",
    toolWhitelist: ["read_file", "write_file"],
  },
  auditor: {
    id: "auditor",
    name: "Auditor",
    nameZh: "审计员",
    description: "Audits chapter continuity across 37 dimensions",
    descriptionZh: "对章节进行 37 维度结构审计",
    toolWhitelist: ["read_file"],
  },
  reviser: {
    id: "reviser",
    name: "Reviser",
    nameZh: "修订者",
    description: "Revises chapter based on auditor issues",
    descriptionZh: "根据审计问题修订章节",
    toolWhitelist: ["read_file", "write_file"],
  },
  observer: {
    id: "observer",
    name: "Observer",
    nameZh: "观察者",
    description: "Extracts chapter facts for truth files",
    descriptionZh: "提取章节事实，更新真相文件",
    toolWhitelist: ["read_file"],
  },
  settler: {
    id: "settler",
    name: "Settler",
    nameZh: "结算者",
    description: "Settles chapter state into truth files",
    descriptionZh: "结算章节状态到真相文件",
    toolWhitelist: ["read_file"],
  },
  code_reviewer: {
    id: "code_reviewer",
    name: "Code Reviewer",
    nameZh: "代码审查员",
    description: "Reviews code with 10 finder angles and 3-vote adversarial verification",
    descriptionZh: "以 10 个视角审查代码，三票对抗式验证",
    toolWhitelist: ["read_file", "grep", "glob"],
  },
  deep_researcher: {
    id: "deep_researcher",
    name: "Deep Researcher",
    nameZh: "深度研究员",
    description: "Conducts 5-phase deep research with web search and fetch",
    descriptionZh: "执行 5 阶段深度研究（网络搜索 + 抓取）",
    toolWhitelist: ["web_search", "web_fetch"],
  },
};

export function getSubagent(id: SubagentType): SubagentDescriptor {
  return SUBAGENT_REGISTRY[id];
}

export function listSubagents(): SubagentDescriptor[] {
  return Object.values(SUBAGENT_REGISTRY);
}

export function getSubagentTools(id: SubagentType): readonly string[] {
  return SUBAGENT_REGISTRY[id].toolWhitelist;
}

export function isSubagentType(value: string): value is SubagentType {
  return value in SUBAGENT_REGISTRY;
}