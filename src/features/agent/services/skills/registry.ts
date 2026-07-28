// Skill Registry —— 技能注册表，管理所有技能定义和执行。
//
// 功能:
// - register(): 注册技能定义
// - get(): 获取技能定义
// - list(): 列出所有技能
// - execute(): 执行技能
// - parseMarkdownSkill(): 解析 Markdown 格式技能文件（YAML frontmatter）
//
// 内置技能通过 builtin/ 目录下的模块加载。

import type {
  SkillDefinition,
  SkillPhase,
  SkillExecutionContext,
  SkillExecutionResult,
  PhaseExecutionResult,
  SkillYamlFrontmatter,
} from "./types";

export class SkillRegistry {
  private skills: Map<string, SkillDefinition> = new Map();

  register(skill: SkillDefinition): void {
    const name = skill.metadata.name;
    if (this.skills.has(name)) {
      console.warn(`Skill "${name}" already registered, overwriting`);
    }
    this.skills.set(name, skill);
  }

  get(name: string): SkillDefinition | undefined {
    return this.skills.get(name);
  }

  list(): SkillDefinition[] {
    return Array.from(this.skills.values());
  }

  listNames(): string[] {
    return Array.from(this.skills.keys());
  }

  has(name: string): boolean {
    return this.skills.has(name);
  }

  async execute(
    name: string,
    ctx: SkillExecutionContext
  ): Promise<SkillExecutionResult> {
    const skill = this.skills.get(name);
    if (!skill) {
      return {
        skillName: name,
        status: "failed",
        output: "",
        phases: [],
        durationMs: 0,
        errorMessage: `Skill "${name}" not found. Available skills: ${this.listNames().join(", ")}`,
      };
    }

    const startTime = Date.now();
    const phaseResults: PhaseExecutionResult[] = [];
    let output = "";
    let totalTokens = 0;
    let hasError = false;

    for (const phase of skill.phases) {
      const phaseStart = Date.now();
      try {
        const phaseOutput = await this.executePhase(phase, ctx, phaseResults);
        phaseResults.push({
          phaseName: phase.name,
          status: "success",
          output: phaseOutput,
          durationMs: Date.now() - phaseStart,
        });
        output += phaseOutput + "\n";
      } catch (error) {
        hasError = true;
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        phaseResults.push({
          phaseName: phase.name,
          status: "failed",
          output: "",
          durationMs: Date.now() - phaseStart,
          tokensUsed: 0,
        });
        return {
          skillName: name,
          status: "failed",
          output: "",
          phases: phaseResults,
          durationMs: Date.now() - startTime,
          tokensUsed: totalTokens,
          errorMessage: `Phase "${phase.name}" failed: ${errorMessage}`,
        };
      }
    }

    return {
      skillName: name,
      status: hasError ? "partial" : "success",
      output: output.trim(),
      phases: phaseResults,
      durationMs: Date.now() - startTime,
      tokensUsed: totalTokens,
    };
  }

  private async executePhase(
    phase: SkillPhase,
    ctx: SkillExecutionContext,
    previousPhases: PhaseExecutionResult[]
  ): Promise<string> {
    // 实际的 Phase 执行由各个技能自行实现
    // 这里提供基础的模板渲染和参数注入
    let instructions = phase.instructions;

    // 替换参数占位符
    if (ctx.parameters) {
      for (const [key, value] of Object.entries(ctx.parameters)) {
        const placeholder = `{{${key}}}`;
        if (instructions.includes(placeholder)) {
          instructions = instructions.replace(
            new RegExp(placeholder, "g"),
            String(value)
          );
        }
      }
    }

    // 替换上下文占位符
    instructions = instructions.replace(/\{\{sessionId\}\}/g, ctx.sessionId);
    if (ctx.workspaceId) {
      instructions = instructions.replace(
        /\{\{workspaceId\}\}/g,
        ctx.workspaceId
      );
    }

    // 替换之前的 Phase 输出
    for (const prevPhase of previousPhases) {
      const placeholder = `{{${prevPhase.phaseName}.output}}`;
      if (instructions.includes(placeholder)) {
        instructions = instructions.replace(
          new RegExp(placeholder.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g"),
          prevPhase.output
        );
      }
    }

    return instructions;
  }

  static parseMarkdownSkill(markdown: string): SkillDefinition {
    const frontmatterMatch = markdown.match(
      /^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/
    );

    if (!frontmatterMatch) {
      throw new Error("Invalid skill format: missing YAML frontmatter");
    }

    const [, frontmatterYaml, body] = frontmatterMatch;
    const parsedFrontmatter = this.parseYamlFrontmatter(frontmatterYaml);
    const phases = this.parsePhases(body);

    return {
      metadata: {
        name: parsedFrontmatter.name,
        description: parsedFrontmatter.description,
        whenToUse: parsedFrontmatter.when_to_use,
        version: parsedFrontmatter.version,
        author: parsedFrontmatter.author,
        tags: parsedFrontmatter.tags,
      },
      phases,
      outputFormat: parsedFrontmatter.output_format || "markdown",
      parameters: parsedFrontmatter.parameters,
    };
  }

  private static parseYamlFrontmatter(
    yaml: string
  ): SkillYamlFrontmatter {
    const lines = yaml.split("\n");
    const result: Record<string, unknown> = {};

    for (const line of lines) {
      const colonIndex = line.indexOf(":");
      if (colonIndex === -1) continue;

      const key = line.slice(0, colonIndex).trim();
      const value = line.slice(colonIndex + 1).trim();

      // 简单的 YAML 解析（支持字符串、数组）
      if (value.startsWith("[")) {
        // 数组格式: [item1, item2]
        const arrayMatch = value.match(/\[(.*)\]/);
        if (arrayMatch) {
          result[key] = arrayMatch[1]
            .split(",")
            .map((item) => item.trim().replace(/^["']|["']$/g, ""));
        }
      } else if (value.startsWith('"') && value.endsWith('"')) {
        result[key] = value.slice(1, -1);
      } else if (value.startsWith("'") && value.endsWith("'")) {
        result[key] = value.slice(1, -1);
      } else {
        result[key] = value;
      }
    }

    return result as unknown as SkillYamlFrontmatter;
  }

  private static parsePhases(body: string): SkillPhase[] {
    const phases: SkillPhase[] = [];
    const phaseRegex = /^##\s+Phase\s+(\d+):\s+(.+)$/gm;
    let match: RegExpExecArray | null;

    const matches: Array<{ index: number; phaseNum: string; name: string }> =
      [];
    while ((match = phaseRegex.exec(body)) !== null) {
      matches.push({
        index: match.index,
        phaseNum: match[1],
        name: match[2],
      });
    }

    for (let i = 0; i < matches.length; i++) {
      const current = matches[i];
      const next = matches[i + 1];
      const startIndex = current.index + body.slice(current.index).indexOf("\n") + 1;
      const endIndex = next ? next.index : body.length;
      const instructions = body.slice(startIndex, endIndex).trim();

      phases.push({
        name: current.name,
        instructions,
        type: this.inferPhaseType(current.phaseNum),
      });
    }

    return phases;
  }

  private static inferPhaseType(
    phaseNum: string
  ): "discover" | "deliver" | "verify" | "persist" | "schedule" {
    const num = parseInt(phaseNum, 10);
    if (num === 0) return "discover";
    if (num <= 2) return "deliver";
    if (num === 3) return "verify";
    if (num === 4) return "persist";
    return "schedule";
  }
}

// 全局单例
let globalRegistry: SkillRegistry | null = null;

export function getSkillRegistry(): SkillRegistry {
  if (!globalRegistry) {
    globalRegistry = new SkillRegistry();
  }
  return globalRegistry;
}

export function resetSkillRegistry(): void {
  globalRegistry = null;
}