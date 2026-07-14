// 跨模块共享类型：页面路由与全局状态。
// 业务领域类型已内聚到各 modules/<area>/types/，此处仅保留真正跨模块的。

/**
 * 设置子页路由。每个设置子页都是独立的路由页面，
 * 通过 sidebar 点击直接路由跳转，与其他主页面（chat/novels/skills 等）一致。
 */
export type SettingsPage =
  | "settings.general"
  | "settings.userProfile"
  | "settings.genres"
  | "settings.styles"
  | "settings.model"
  | "settings.embedding"
  | "settings.prompts"
  | "settings.agents"
  | "settings.bookSources"
  | "settings.network"
  | "settings.audit"
  | "settings.git"
  | "settings.shortcuts"
  | "settings.system"
  | "settings.logs"
  | "settings.skillEvolution"
  | "settings.learnedPreferences"
  | "settings.shortTermMemory"
  | "settings.agentAudit"
  | "settings.dailySummary"
  | "settings.projectMemory"
  | "settings.toolLimits"
  | "settings.about";

/** 设置子页默认入口 */
export const DEFAULT_SETTINGS_PAGE: SettingsPage = "settings.general";

/** 判断页面是否属于设置子页 */
export function isSettingsPage(page: string): page is SettingsPage {
  return page.startsWith("settings.");
}

export type WorkspacePage = "overview" | "characters" | "worldbuilding" | "plot" | "timeline" | "research";

export type AppPage =
  | WorkspacePage
  | SettingsPage
  | "settings"
  | "trends" | "novels" | "skills" | "chat" | "memory"
  | "dashboard" | "knowledge" | "main-agent" | "wiki" | "version"
  | "loops" | "git" | "novel-reader" | "pipeline" | "audit";

export interface AppState {
  currentPage: AppPage;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  extension: string | null;
  size: number;
}