// 跨模块共享类型：页面路由与全局状态。
// 业务领域类型已内聚到各 modules/<area>/types/，此处仅保留真正跨模块的。

/**
 * 设置子页路由。每个设置子页都是独立的路由页面，
 * 通过 sidebar 点击直接路由跳转，与其他主页面（chat/novels/skills 等）一致。
 *
 * 扁平化导航项（23 个）：
 * - settings.general - 基础设置（语言、主题、通知、启动）
 * - settings.system - 系统设置（数据目录、日志级别）
 * - settings.shortcuts - 快捷键设置
 * - settings.about - 关于页面
 * - settings.userProfile - 用户画像
 * - settings.genres - 题材管理
 * - settings.styles - 风格管理
 * - settings.model - AI 模型设置
 * - settings.embedding - Embedding 设置
 * - settings.prompts - 提示词管理
 * - settings.agents - Agent 设置
 * - settings.bookSources - 书源管理
 * - settings.shortTerm - 短期记忆
 * - settings.project - 项目记忆
 * - settings.learned - 学习偏好
 * - settings.daily - 每日摘要
 * - settings.skillMemory - 技能记忆
 * - settings.rules - 安全规则
 * - settings.events - 审计事件
 * - settings.network - 网络设置
 * - settings.git - Git 设置
 * - settings.limits - 工具限制
 * - settings.usageStats - 使用统计
 */
export type SettingsPage =
  | "settings.general"
  | "settings.system"
  | "settings.shortcuts"
  | "settings.about"
  | "settings.userProfile"
  | "settings.genres"
  | "settings.styles"
  | "settings.model"
  | "settings.embedding"
  | "settings.prompts"
  | "settings.agents"
  | "settings.bookSources"
  | "settings.shortTerm"
  | "settings.project"
  | "settings.learned"
  | "settings.daily"
  | "settings.skillMemory"
  | "settings.rules"
  | "settings.events"
  | "settings.network"
  | "settings.git"
  | "settings.limits"
  | "settings.usageStats"
  | "settings.archive";

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