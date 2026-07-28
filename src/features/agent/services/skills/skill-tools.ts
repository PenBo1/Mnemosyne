// Skill Tools —— Agent 可调用的技能工具。
//
// 工具列表:
// - run_skill: 执行技能
//
// 这些工具会被 Rust 侧的 agent engine 调用。

import { z } from "zod";
import type {
  SkillExecutionContext,
  SkillExecutionResult,
} from "./types";
import { getSkillRegistry } from "./registry";

export const RunSkillSchema = z.object({
  skill: z.string().min(1).max(100),
  params: z.record(z.string(), z.unknown()).optional().default({}),
  effort: z.enum(["low", "medium", "high", "xhigh", "max"]).optional(),
});

export type RunSkillParams = z.infer<typeof RunSkillSchema>;

export interface SkillToolDefinition {
  name: string;
  description: string;
  parameters: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
}

export const RUN_SKILL_DESC_KEY = "skills.runSkillDesc";
export const DEFAULT_RUN_SKILL_DESC =
  "Execute a skill with specified parameters. Skills are reusable multi-phase workflows.";

export function getRunSkillTool(): SkillToolDefinition {
  return {
    name: "run_skill",
    description: DEFAULT_RUN_SKILL_DESC,
    parameters: {
      type: "object",
      properties: {
        skill: {
          type: "string",
          description: "Skill name to execute (loop, code-review, deep-research)",
        },
        params: {
          type: "object",
          description: "Skill-specific parameters as key-value pairs",
        },
        effort: {
          type: "string",
          enum: ["low", "medium", "high", "xhigh", "max"],
          description: "Effort level controlling depth and thoroughness",
        },
      },
      required: ["skill"],
    },
  };
}

export async function runSkill(
  params: RunSkillParams,
  ctx: SkillToolContext
): Promise<SkillExecutionResult> {
  const { skill, params: skillParams, effort } = RunSkillSchema.parse(params);

  const registry = getSkillRegistry();
  if (!registry.has(skill)) {
    return {
      skillName: skill,
      status: "failed",
      output: "",
      phases: [],
      durationMs: 0,
      errorMessage: `Skill "${skill}" not found. Available skills: ${registry.listNames().join(", ")}`,
    };
  }

  const execCtx: SkillExecutionContext = {
    sessionId: ctx.sessionId,
    workspaceId: ctx.workspaceId,
    parameters: skillParams,
    effort,
  };

  return registry.execute(skill, execCtx);
}

export const SKILL_TOOLS = [
  {
    name: "run_skill",
    description: DEFAULT_RUN_SKILL_DESC,
    parameters: {
      type: "object",
      properties: {
        skill: {
          type: "string",
          description: "Skill name to execute",
        },
        params: {
          type: "object",
          description: "Skill-specific parameters",
        },
        effort: {
          type: "string",
          enum: ["low", "medium", "high", "xhigh", "max"],
          description: "Effort level",
        },
      },
      required: ["skill"],
    },
  },
];

export interface SkillToolContext {
  sessionId: string;
  workspaceId?: string;
  userId?: string;
  role?: string;
}

export function getSkillTools(_ctx: SkillToolContext): SkillToolDefinition[] {
  return [getRunSkillTool()];
}

export function getSkillToolSchemas(): Record<string, unknown> {
  return {
    run_skill: {
      type: "object",
      properties: {
        skill: { type: "string", minLength: 1, maxLength: 100 },
        params: { type: "object" },
        effort: { type: "string", enum: ["low", "medium", "high", "xhigh", "max"] },
      },
      required: ["skill"],
    },
  };
}

export async function executeSkillTool(
  name: string,
  args: Record<string, unknown>,
  ctx: SkillToolContext
): Promise<string> {
  switch (name) {
    case "run_skill": {
      const result = await runSkill({
        skill: args.skill as string,
        params: (args.params as Record<string, unknown>) || {},
        effort: args.effort as RunSkillParams["effort"],
      }, ctx);
      return JSON.stringify(result);
    }
    default:
      throw new Error(`Unknown skill tool: ${name}`);
  }
}