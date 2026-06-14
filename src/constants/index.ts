import type { PromptCategoryInfo, GenreConfig, BookRules } from "@/types/prompts";

export const DEFAULT_PAGE = "chat" as const;

export const AVAILABLE_MODELS = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo", "claude-3-opus", "claude-3-sonnet"] as const;

export const PROMPT_CATEGORIES: PromptCategoryInfo[] = [
  {
    id: "writer",
    name: "Writer Prompts",
    description: "System prompts for generating novel prose",
    icon: "pen-tool",
  },
  {
    id: "planner",
    name: "Planner Prompts",
    description: "Prompts for chapter outlines and memos",
    icon: "map",
  },
  {
    id: "settler",
    name: "Settler Prompts",
    description: "Prompts for tracking story state changes",
    icon: "check-circle",
  },
  {
    id: "observer",
    name: "Observer Prompts",
    description: "Prompts for extracting chapter facts",
    icon: "eye",
  },
  {
    id: "short_fiction",
    name: "Short Fiction Prompts",
    description: "Prompts for short story creation",
    icon: "file-text",
  },
  {
    id: "fanfic",
    name: "Fanfic Prompts",
    description: "Prompts for fan fiction creation",
    icon: "copy",
  },
  {
    id: "custom",
    name: "Custom Prompts",
    description: "User-defined prompts",
    icon: "settings",
  },
];

export const DEFAULT_GENRE_CONFIG: GenreConfig = {
  id: "other",
  name: "Other",
  language: "zh",
  fatigue_words: [],
  pacing_rule: "",
  chapter_types: ["transition", "conflict", "climax", "resolution"],
  numerical_system: false,
  power_scaling: false,
};

export const DEFAULT_BOOK_RULES: BookRules = {
  personality_lock: [],
  behavioral_constraints: [],
  prohibitions: [],
  enable_full_cast_tracking: false,
  genre_forbidden: [],
};
