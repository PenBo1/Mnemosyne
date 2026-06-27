export type SettingsTab = "general" | "model" | "prompts" | "agents" | "audit" | "system" | "bookSources";

export type WorkspacePage = "overview" | "characters" | "worldbuilding" | "plot" | "timeline" | "research";
export type AppPage = WorkspacePage | "settings" | "trends" | "novels" | "skills" | "chat" | "memory" | "dashboard" | "knowledge" | "main-agent" | "wiki" | "version" | "kanban" | "loops";

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
  platform: string;
  status: "drafting" | "paused" | "completed" | "archived";
  language: "zh" | "en";
  word_count: number;
  chapter_count: number;
  target_chapters: number;
  chapter_words: number;
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

export interface AgentIdentity {
  role: string;
  soul: string;
  context: string;
  memory: string;
}

// ── User Profile ──────────────────────────────────────────

export interface Session {
  id: string;
  novel_id: string | null;
  session_type: "chat" | "pipeline" | "review";
  title: string;
  summary: string | null;
  message_count: number;
  input_tokens: number;
  output_tokens: number;
  cost: number;
  status: "active" | "paused" | "completed" | "archived";
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

// ── Book Source (Novel Download) ──────────────────────

export interface BookSource {
  url: string;
  name: string;
  comment: string;
  disabled: boolean;
  search?: SearchRule;
  book?: BookRule;
  toc?: TocRule;
  chapter?: ChapterRule;
}

export interface SearchRule {
  disabled: boolean;
  url: string;
  method: string;
  data: string;
  cookies: string;
  result: string;
  book_name: string;
  author: string;
  category: string;
  word_count: string;
  status: string;
  latest_chapter: string;
  last_update_time: string;
  pagination: boolean;
  next_page: string;
}

export interface BookRule {
  url: string;
  book_name: string;
  author: string;
  intro: string;
  category: string;
  cover_url: string;
  latest_chapter: string;
  last_update_time: string;
  status: string;
}

export interface TocRule {
  base_uri: string;
  url: string;
  item: string;
  is_desc: boolean;
  pagination: boolean;
  next_page: string;
}

export interface ChapterRule {
  title: string;
  content: string;
  paragraph_tag_closed: boolean;
  paragraph_tag: string;
  filter_txt: string;
  filter_tag: string;
  pagination: boolean;
  next_page: string;
}

export interface SearchBookResult {
  book_name: string;
  author: string;
  url: string;
  category: string;
  word_count: string;
  status: string;
  latest_chapter: string;
  last_update_time: string;
  source_name: string;
  source_url: string;
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

// ── Wiki Entry ─────────────────────────────────────────────

export type WikiCategory = "character" | "location" | "event" | "concept" | "item" | "other";

export interface WikiEntry {
  id: string;
  novel_id: string;
  title: string;
  content: string;
  category: WikiCategory;
  tags: string[];
  importance: number;
  source_chapter: number | null;
  created_at: string;
  updated_at: string;
}

export interface CreateWikiEntryRequest {
  title: string;
  content: string;
  category: WikiCategory;
  tags?: string[];
  importance?: number;
  source_chapter?: number;
}

export interface UpdateWikiEntryRequest {
  title?: string;
  content?: string;
  category?: WikiCategory;
  tags?: string[];
  importance?: number;
  source_chapter?: number;
}

export interface WikiEntityLink {
  id: string;
  novel_id: string;
  source_entry_id: string;
  target_entry_id: string;
  link_type: string;
  description: string;
  created_at: string;
}

export interface WikiGraphNode {
  id: string;
  title: string;
  category: WikiCategory;
  importance: number;
}

export interface WikiGraphLink {
  source: string;
  target: string;
  type: string;
}

export interface WikiGraphView {
  nodes: WikiGraphNode[];
  links: WikiGraphLink[];
}

// ── Chapter Version ────────────────────────────────────────

export type RevisionMode = "auto" | "polish" | "rewrite" | "rework" | "spot_fix" | "manual";

export interface ChapterVersion {
  id: string;
  novel_id: string;
  chapter_number: number;
  version_number: number;
  content: string;
  content_hash: string;
  word_count: number;
  revision_reason: string;
  revision_mode: RevisionMode;
  created_at: string;
}

export type DiffLineType = "added" | "removed" | "context";

export interface DiffLine {
  line_type: DiffLineType;
  content: string;
  old_number: number | null;
  new_number: number | null;
}

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  lines: DiffLine[];
}

export interface DiffStats {
  lines_added: number;
  lines_removed: number;
  lines_modified: number;
  chars_added: number;
  chars_removed: number;
}

export interface LineDiffResult {
  hunks: DiffHunk[];
  stats: DiffStats;
}

// ── Kanban ─────────────────────────────────────────────────

export type KanbanTaskStatus = "plan" | "compose" | "write" | "audit" | "revise" | "done" | "cancelled";
export type KanbanPriority = "low" | "medium" | "high" | "urgent";

export interface KanbanTask {
  id: string;
  novel_id: string;
  title: string;
  description: string;
  status: KanbanTaskStatus;
  priority: KanbanPriority;
  assigned_agent: string | null;
  chapter_id: string | null;
  parent_task_id: string | null;
  tags: string[];
  sort_order: number;
  due_date: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateKanbanTaskRequest {
  title: string;
  description?: string;
  status?: KanbanTaskStatus;
  priority?: KanbanPriority;
  assigned_agent?: string;
  chapter_id?: string;
  parent_task_id?: string;
  tags?: string[];
  due_date?: string;
}

export interface UpdateKanbanTaskRequest {
  title?: string;
  description?: string;
  status?: KanbanTaskStatus;
  priority?: KanbanPriority;
  assigned_agent?: string;
  chapter_id?: string;
  parent_task_id?: string;
  sort_order?: number;
  due_date?: string;
  tags?: string[];
}

export interface KanbanColumn {
  id: string;
  novel_id: string;
  name: string;
  status_key: string;
  color: string;
  sort_order: number;
  wip_limit: number | null;
  created_at: string;
}

export interface CreateKanbanColumnRequest {
  name: string;
  status_key: string;
  color?: string;
  sort_order?: number;
  wip_limit?: number;
}

export interface UpdateKanbanColumnRequest {
  name?: string;
  color?: string;
  sort_order?: number;
  wip_limit?: number;
}

export interface KanbanState {
  tasks: KanbanTask[];
  columns: KanbanColumn[];
  loading: boolean;
  error: string | null;
  loadTasks: (novelId: string) => Promise<void>;
  loadColumns: (novelId: string) => Promise<void>;
  createTask: (novelId: string, req: CreateKanbanTaskRequest) => Promise<KanbanTask>;
  updateTask: (taskId: string, req: UpdateKanbanTaskRequest) => Promise<void>;
  deleteTask: (taskId: string) => Promise<void>;
  moveTask: (taskId: string, newStatus: KanbanTaskStatus) => Promise<void>;
  reorderTasks: (taskIds: string[]) => Promise<void>;
}

// ── Loop Engineering ───────────────────────────────────────

export type LoopStatus = "idle" | "running" | "paused" | "error";
export type ReadinessLevel = "L0" | "L1" | "L2" | "L3";
export type LoopRunStatus = "success" | "partial" | "failed" | "escalated";
export type RiskLevel = "low" | "medium" | "high";

export interface LoopState {
  id: string;
  novel_id: string;
  pattern_id: string;
  status: LoopStatus;
  readiness_level: ReadinessLevel;
  state_payload: Record<string, unknown>;
  config: LoopConfig;
  token_usage_today: number;
  token_cap_daily: number;
  last_run_at: string | null;
  last_run_result: LoopRunResult | null;
  created_at: string;
  updated_at: string;
}

export interface LoopConfig {
  cadence: string;
  denylist: string[];
  human_gates: string[];
  max_retries: number;
}

export interface LoopRunResult {
  findings: string[];
  actions: string[];
  escalations: string[];
}

export interface CreateLoopStateRequest {
  pattern_id: string;
  readiness_level?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  token_cap_daily?: number;
}

export interface UpdateLoopStateRequest {
  status?: LoopStatus;
  readiness_level?: ReadinessLevel;
  config?: Partial<LoopConfig>;
  token_cap_daily?: number;
}

export interface LoopRunLog {
  id: string;
  loop_state_id: string;
  pattern_id: string;
  status: LoopRunStatus;
  phase_results: PhaseResult[];
  tokens_used: number;
  duration_ms: number;
  findings: string[];
  actions_taken: string[];
  escalations: string[];
  error_message: string | null;
  created_at: string;
}

export interface PhaseResult {
  phase: string;
  status: string;
  output: string;
  duration_ms: number;
}

export interface LoopPattern {
  id: string;
  name: string;
  description: string;
  goal: string;
  cadence: string;
  risk_level: RiskLevel;
  phases: PhaseDef[];
  human_gates: string[];
  cost_config: CostConfig;
  skills_required: string[];
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PhaseDef {
  name: string;
  description: string;
  type: "discover" | "deliver" | "verify" | "persist" | "schedule";
}

export interface CostConfig {
  tokens_noop: number;
  tokens_report: number;
  tokens_action: number;
  daily_cap: number;
  early_exit_required: boolean;
}

export interface UpsertLoopPatternRequest {
  name: string;
  description?: string;
  goal?: string;
  cadence?: string;
  risk_level?: RiskLevel;
  phases?: PhaseDef[];
  human_gates?: string[];
  cost_config?: Partial<CostConfig>;
  skills_required?: string[];
  state_schema?: Record<string, unknown>;
  is_active?: boolean;
}

export interface LoopEngineState {
  states: LoopState[];
  patterns: LoopPattern[];
  runLogs: LoopRunLog[];
  loading: boolean;
  error: string | null;
  loadStates: (novelId: string) => Promise<void>;
  loadPatterns: () => Promise<void>;
  createState: (novelId: string, req: CreateLoopStateRequest) => Promise<LoopState>;
  deleteState: (stateId: string) => Promise<void>;
  runCycle: (stateId: string) => Promise<LoopRunLog>;
  pauseLoop: (stateId: string) => Promise<void>;
  resumeLoop: (stateId: string) => Promise<void>;
  loadRunLogs: (stateId: string) => Promise<void>;
}
