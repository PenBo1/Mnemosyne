import { ipc } from "@/lib/ipc";

export interface WriteCycleResult {
  chapter: number;
  word_count: number;
  audit_passed: boolean;
  elapsed_secs: number;
}

export interface ScheduledTask {
  id: string;
  name: string;
  book_id: string;
  task_type: string;
  status: string;
  last_run: string | null;
  next_run: string | null;
  error_count: number;
}

export interface SearchResult {
  chunk: {
    id: string;
    content: string;
    source: string;
    chunk_index: number;
  };
  score: number;
  match_type: string;
}

export interface MemoryEntry {
  id: string;
  content: string;
  entry_type: string;
  chapter: number | null;
  timestamp: string;
  tags: string[];
}

export interface GraphState {
  values: Record<string, unknown>;
  current_node: string | null;
  history: string[];
  checkpoint_id: string | null;
}

// ── Scheduler Lifecycle ──────────────────────────────────────

export async function initScheduler(workspaceId: string): Promise<string> {
  return ipc<string>("scheduler_init", { workspaceId });
}

export async function getSchedulerStatus(): Promise<string> {
  return ipc<string>("scheduler_status");
}

export async function pauseScheduler(): Promise<void> {
  await ipc<void>("scheduler_pause");
}

export async function resumeScheduler(): Promise<void> {
  await ipc<void>("scheduler_resume");
}

export async function stopScheduler(): Promise<void> {
  await ipc<void>("scheduler_stop");
}

// ── Write Cycles ─────────────────────────────────────────────

export async function executeWriteCycle(bookId: string): Promise<WriteCycleResult> {
  return ipc<WriteCycleResult>("scheduler_write_cycle", { bookId });
}

// ── Task Management ──────────────────────────────────────────

export async function listScheduledTasks(): Promise<ScheduledTask[]> {
  return ipc<ScheduledTask[]>("scheduler_list_tasks");
}

// ── RAG Search ───────────────────────────────────────────────

export async function searchRAG(query: string, topK?: number): Promise<SearchResult[]> {
  return ipc<SearchResult[]>("scheduler_search_rag", { query, topK: topK ?? 5 });
}

// ── Memory Search ────────────────────────────────────────────

export async function searchMemory(
  bookId: string,
  query: string,
  topK?: number
): Promise<MemoryEntry[]> {
  return ipc<MemoryEntry[]>("scheduler_search_memory", {
    bookId,
    query,
    topK: topK ?? 5,
  });
}

// ── Feedback Lessons ─────────────────────────────────────────

export async function getLessons(): Promise<string> {
  return ipc<string>("scheduler_get_lessons");
}

// ── Checkpoints ──────────────────────────────────────────────

export async function restoreCheckpoint(bookId: string): Promise<GraphState | null> {
  return ipc<GraphState | null>("scheduler_restore_checkpoint", { bookId });
}
