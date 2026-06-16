export type SettingsTab = "general" | "model" | "prompts" | "agents" | "audit" | "system";

export type WorkspacePage = "overview" | "characters" | "worldbuilding" | "plot" | "timeline" | "research";
export type AppPage = WorkspacePage | "settings" | "trends" | "novels" | "skills" | "chat" | "memory" | "dashboard" | "knowledge";

// ── Workspace ──────────────────────────────────────────────

export interface Workspace {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkspaceRequest {
  name: string;
  path?: string;
}

export interface AppState {
  currentPage: AppPage;
  settingsTab: SettingsTab;
}

export interface WorkspaceState {
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  loading: boolean;
  error: string | null;
  loadWorkspaces: () => Promise<void>;
  addWorkspace: (name: string, path?: string) => Promise<void>;
  removeWorkspace: (id: string) => Promise<void>;
  setActiveWorkspace: (id: string) => void;
}

// ── Prompt ─────────────────────────────────────────────────

export interface Prompt {
  id: string;
  name: string;
  content: string;
  category: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ── Novel / Book ───────────────────────────────────────────

export interface Novel {
  id: string;
  workspace_id: string;
  title: string;
  genre: string;
  status: string;
  word_count: number;
  chapter_count: number;
  created_at: string;
  updated_at: string;
}

export interface BookConfig {
  id: string;
  title: string;
  genre: string;
  platform: string;
  status: "drafting" | "writing" | "reviewing" | "completed" | "paused";
  language: string;
  chapter_words: number;
  target_chapters: number;
  created_at: string;
  updated_at: string;
}

export interface WriteResult {
  chapter_number: number;
  title: string;
  content: string;
  word_count: number;
  audit: AuditResult;
}

export interface AuditResult {
  passed: boolean;
  score: number;
  issues: AuditIssue[];
  summary: string;
}

export interface AuditIssue {
  severity: "critical" | "warning" | "info";
  category: string;
  description: string;
  suggestion: string;
}

// ── Provider / Model ───────────────────────────────────────

export interface ModelInfo {
  id: string;
  provider: string;
  name: string;
  context_window: number;
  supports_tools: boolean;
  supports_streaming: boolean;
}

export interface ProviderInfo {
  name: string;
  models: ModelInfo[];
}

export interface ProviderConfig {
  api_key: string;
  base_url: string | null;
}

export interface ProviderSettings {
  default_provider: string;
  default_model: string;
  configs: Record<string, ProviderConfig>;
}

// ── Agent ──────────────────────────────────────────────────

export interface Agent {
  id: string;
  name: string;
  description: string;
  model: string;
  systemPrompt: string;
  temperature: number;
  maxTokens: number;
  status: "active" | "inactive";
  created_at: string;
}

// ── Session ────────────────────────────────────────────────

export interface Session {
  id: string;
  novel_id: string | null;
  title: string;
  summary: string | null;
  message_count: number;
  input_tokens: number;
  output_tokens: number;
  cost: number;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  tool_calls: string | null;
  tool_results: string | null;
  token_count: number | null;
  created_at: string;
}

// ── Agent Events / IPC ─────────────────────────────────────

export interface AgentEvent {
  type: "TurnStarted" | "StreamDelta" | "ToolCallBegin" | "ToolCallEnd" | "TurnCompleted" | "Error" | "CompactionTriggered";
  session_id: string;
  content?: string;
  tool_call_id?: string;
  tool?: string;
  args?: string;
  output?: string;
  is_error?: boolean;
  input_tokens?: number;
  output_tokens?: number;
  error?: string;
}

export interface SendMessageParams {
  session_id: string;
  content: string;
  [key: string]: unknown;
}

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

// ── Memory ─────────────────────────────────────────────────

export interface Memory {
  id: string;
  title: string;
  content: string;
  category: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export const MEMORY_CATEGORIES = ["character", "world", "plot", "style", "reference", "other"] as const;

// ── Radar ──────────────────────────────────────────────────

export interface RadarScan {
  id: string;
  market_summary: string;
  recommendations: RadarRecommendation[];
  raw_rankings: PlatformRankings[];
  created_at: string;
}

export interface RadarRecommendation {
  platform: string;
  genre: string;
  concept: string;
  confidence: number;
  reasoning: string;
  benchmark_titles: string[];
}

export interface PlatformRankings {
  platform: string;
  entries: RankingEntry[];
}

export interface RankingEntry {
  title: string;
  author: string;
  category: string;
  extra: string;
}

// ── Character ─────────────────────────────────────────────

export interface Character {
  id: string;
  novel_id: string;
  name: string;
  role: string;
  age: string;
  gender: string;
  appearance: string;
  personality: string;
  backstory: string;
  motivation: string;
  fears: string;
  skills: string;
  description: string;
  traits: string[];
  custom_fields: string;
  created_at: string;
  updated_at: string;
}

export interface CharacterRelationship {
  id: string;
  novel_id: string;
  character_a_id: string;
  character_b_id: string;
  relationship_type: string;
  description: string;
  created_at: string;
}

// ── World Setting ─────────────────────────────────────────

export type WorldCategory =
  | "location"
  | "faction"
  | "species"
  | "culture"
  | "history"
  | "magic_system"
  | "language"
  | "architecture";

export interface WorldSetting {
  id: string;
  novel_id: string;
  category: WorldCategory;
  name: string;
  description: string;
  content: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ── Plot Point ────────────────────────────────────────────

export type PlotPointType = "act" | "chapter" | "scene";

export interface PlotPoint {
  id: string;
  novel_id: string;
  type: PlotPointType;
  parent_id: string | null;
  title: string;
  description: string;
  sort_order: number;
  chapter_number: number | null;
  pov_character_id: string | null;
  location_id: string | null;
  goals: string;
  conflicts: string;
  outcome: string;
  status: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ── Timeline Event ────────────────────────────────────────

export type TimelineEventType = "event" | "milestone" | "turning_point";

export interface TimelineEvent {
  id: string;
  novel_id: string;
  title: string;
  description: string;
  event_date: string;
  sort_order: number;
  chapter_number: number | null;
  character_ids: string[];
  location_id: string | null;
  event_type: TimelineEventType;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ── Research Item ─────────────────────────────────────────

export type ResearchCategory = "reference" | "inspiration" | "note" | "link";

export interface ResearchItem {
  id: string;
  novel_id: string;
  title: string;
  content: string;
  category: ResearchCategory;
  tags: string[];
  source_url: string | null;
  created_at: string;
  updated_at: string;
}

// ── Hook Record ──────────────────────────────────────────

export type HookStatus = "Open" | "Progressing" | "Deferred" | "Resolved";

export interface HookRecord {
  hook_id: string;
  name: string;
  hook_type: string;
  start_chapter: number;
  status: HookStatus;
  expected_payoff: string;
  last_advanced_chapter: number;
  core_hook: boolean;
  created_at: string;
  updated_at: string;
}

// ── Chapter Summary ──────────────────────────────────────

export interface ChapterSummary {
  chapter: number;
  title: string;
  characters: string[];
  events: string[];
  state_changes: string[];
  hook_activity: string[];
  mood: string;
  chapter_type: string;
  created_at: string;
}

// ── Knowledge ──────────────────────────────────────────

export interface KnowledgeEntry {
  id: string;
  title: string;
  content: string;
  category: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// ── Story Fact ───────────────────────────────────────────

export interface StoryFact {
  fact_id: string;
  subject: string;
  predicate: string;
  object: string;
  valid_from_chapter: number;
  valid_until_chapter: number | null;
  source_chapter: number;
  created_at: string;
}
