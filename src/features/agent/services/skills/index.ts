// Skills 模块入口 —— 导出所有类型和函数。

export * from "./types";
export * from "./registry";
export * from "./skill-tools";

// Builtin Skills
export {
  LOOP_SKILL_MARKDOWN,
  LOOP_SKILL_DEFINITION,
  getLoopSkillDefinition,
} from "./builtin/loop-skill";

export {
  CODE_REVIEW_SKILL_MARKDOWN,
  CODE_REVIEW_SKILL_DEFINITION,
  FINDER_ANGLES,
  EFFORT_ANGLE_COUNT,
  selectFinderAngles,
  getCodeReviewSkillDefinition,
} from "./builtin/code-review-skill";

export {
  DEEP_RESEARCH_SKILL_MARKDOWN,
  DEEP_RESEARCH_SKILL_DEFINITION,
  getDeepResearchSkillDefinition,
} from "./builtin/deep-research-skill";

// Initialization helper
import { getSkillRegistry } from "./registry";
import { getLoopSkillDefinition } from "./builtin/loop-skill";
import { getCodeReviewSkillDefinition } from "./builtin/code-review-skill";
import { getDeepResearchSkillDefinition } from "./builtin/deep-research-skill";

export function registerBuiltinSkills(): void {
  const registry = getSkillRegistry();
  registry.register(getLoopSkillDefinition());
  registry.register(getCodeReviewSkillDefinition());
  registry.register(getDeepResearchSkillDefinition());
}