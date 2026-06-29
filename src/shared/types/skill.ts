// ── Skill ──────────────────────────────────────────────────

export interface SkillMeta {
  name: string;
  description: string;
  category: string;
  requires_tools: string[];
  platforms: string[] | null;
}

export interface Skill extends SkillMeta {
  content: string;
  path: string;
}

export const SKILL_CATEGORIES = ["writing", "editing", "research", "analysis", "other"] as const;

export interface CreateSkillParams {
  name: string;
  description: string;
  category: string;
  content: string;
  [key: string]: unknown;
}

export interface UpdateSkillParams {
  name: string;
  description: string;
  category: string;
  content: string;
  [key: string]: unknown;
}
