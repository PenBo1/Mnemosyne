// 跨模块共享类型：页面路由与全局状态。
// 业务领域类型已内聚到各 modules/<area>/types/，此处仅保留真正跨模块的。

/**
 * 设置子页路由。每个设置子页都是独立的路由页面，
 * 通过 sidebar 点击直接路由跳转，与其他主页面（chat/novels/skills 等）一致。
 *
 * 合并后的导航项（10 个）：
 * - settings.general - 通用设置（合并 System + Shortcuts + About）
 * - settings.userProfile - 用户画像
 * - settings.genres - 题材管理
 * - settings.styles - 风格管理
 * - settings.ai - AI 与智能体（合并 Model + Embedding + Prompts + Agents）
 * - settings.bookSources - 书源管理
 * - settings.memory - 记忆与学习（合并 5 个记忆相关页面）
 * - settings.security - 安全审计（合并 Audit + AgentAudit）
 * - settings.networkTools - 网络与工具（合并 Network + Git + ToolLimits）
 * - settings.usageStats - 使用统计
 */
export type SettingsPage =
  | "settings.general"
  | "settings.userProfile"
  | "settings.genres"
  | "settings.styles"
  | "settings.ai"
  | "settings.bookSources"
  | "settings.memory"
  | "settings.security"
  | "settings.networkTools"
  | "settings.usageStats";

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
  | "knowledge" | "main-agent" | "wiki" | "version"
  | "loops" | "git" | "novel-reader" | "pipeline" | "audit" | "editor";

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