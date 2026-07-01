// ── App-level routing types ─────────────────────────────────

import type { Workspace } from "./workspace";

export type SettingsTab = "general" | "model" | "prompts" | "agents" | "audit" | "system" | "bookSources" | "git";

export type WorkspacePage = "overview" | "characters" | "worldbuilding" | "plot" | "timeline" | "research";
export type AppPage = WorkspacePage | "settings" | "trends" | "novels" | "skills" | "chat" | "memory" | "dashboard" | "knowledge" | "main-agent" | "wiki" | "version" | "loops" | "git";

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
