import type { PageConfig } from "./types";

export const pageRegistry: Record<string, PageConfig> = {
  chat: {
    loader: () => import("@/features/chat/ChatPage"),
    layout: "default",
  },
  memory: {
    loader: () => import("@/features/memory/MemoryPage").then((m) => ({ default: m.MemoryPage })),
    layout: "default",
  },
  settings: {
    loader: () => import("@/features/settings/SettingsPage").then((m) => ({ default: m.SettingsPage })),
    layout: "default",
  },
  // ── 基础设置组 ────────────────────────────────────────────────────────
  "settings.general": {
    loader: () => import("@/features/settings/sections/GeneralSettings").then((m) => ({ default: m.GeneralSettings })),
    layout: "default",
  },
  "settings.system": {
    loader: () => import("@/features/settings/sections/SystemSettings").then((m) => ({ default: m.SystemSettings })),
    layout: "default",
  },
  "settings.shortcuts": {
    loader: () => import("@/features/settings/sections/ShortcutsSettings").then((m) => ({ default: m.ShortcutsSettings })),
    layout: "default",
  },
  "settings.about": {
    loader: () => import("@/features/settings/sections/AboutSettings").then((m) => ({ default: m.AboutSettings })),
    layout: "default",
  },
  // ── 用户与内容组 ────────────────────────────────────────────────────────
  "settings.userProfile": {
    loader: () => import("@/features/settings/sections/UserProfileSettings").then((m) => ({ default: m.UserProfileSettings })),
    layout: "default",
  },
  "settings.genres": {
    loader: () => import("@/features/settings/sections/GenreSettings").then((m) => ({ default: m.GenreSettings })),
    layout: "default",
  },
  "settings.styles": {
    loader: () => import("@/features/settings/sections/StyleSettings").then((m) => ({ default: m.StyleSettings })),
    layout: "default",
  },
  // ── AI 模型组 ────────────────────────────────────────────────────────
  "settings.model": {
    loader: () => import("@/features/settings/sections/ModelSettings").then((m) => ({ default: m.ModelSettings })),
    layout: "default",
  },
  "settings.embedding": {
    loader: () => import("@/features/settings/sections/EmbeddingSettings").then((m) => ({ default: m.EmbeddingSettings })),
    layout: "default",
  },
  "settings.prompts": {
    loader: () => import("@/features/settings/sections/PromptsSettings").then((m) => ({ default: m.PromptsSettings })),
    layout: "default",
  },
  "settings.agents": {
    loader: () => import("@/features/settings/sections/AgentsSettings").then((m) => ({ default: m.AgentsSettings })),
    layout: "default",
  },
  // ── 内容与记忆组 ────────────────────────────────────────────────────────
  "settings.bookSources": {
    loader: () => import("@/features/settings/sections/BookSourcesSettings").then((m) => ({ default: m.BookSourcesSettings })),
    layout: "default",
  },
  "settings.shortTerm": {
    loader: () => import("@/features/settings/sections/ShortTermMemorySettings").then((m) => ({ default: m.ShortTermMemorySettings })),
    layout: "default",
  },
  "settings.project": {
    loader: () => import("@/features/settings/sections/ProjectMemorySettings").then((m) => ({ default: m.ProjectMemorySettings })),
    layout: "default",
  },
  "settings.learned": {
    loader: () => import("@/features/settings/sections/LearnedPreferencesSettings").then((m) => ({ default: m.LearnedPreferencesSettings })),
    layout: "default",
  },
  "settings.daily": {
    loader: () => import("@/features/settings/sections/DailySummarySettings").then((m) => ({ default: m.DailySummarySettings })),
    layout: "default",
  },
  "settings.skillMemory": {
    loader: () => import("@/features/settings/sections/SkillMemorySettings").then((m) => ({ default: m.SkillMemorySettings })),
    layout: "default",
  },
  // ── 安全与网络组 ────────────────────────────────────────────────────────
  "settings.rules": {
    loader: () => import("@/features/settings/sections/SecurityRulesSettings").then((m) => ({ default: m.SecurityRulesSettings })),
    layout: "default",
  },
  "settings.events": {
    loader: () => import("@/features/settings/sections/AuditEventsSettings").then((m) => ({ default: m.AuditEventsSettings })),
    layout: "default",
  },
  "settings.network": {
    loader: () => import("@/features/settings/sections/NetworkSettings").then((m) => ({ default: m.NetworkSettings })),
    layout: "default",
  },
  "settings.git": {
    loader: () => import("@/features/settings/sections/GitSettings").then((m) => ({ default: m.GitSettings })),
    layout: "default",
  },
  "settings.limits": {
    loader: () => import("@/features/settings/sections/ToolLimitsSettings").then((m) => ({ default: m.ToolLimitsSettings })),
    layout: "default",
  },
  // ── 统计 ────────────────────────────────────────────────────────
  "settings.usageStats": {
    loader: () => import("@/features/settings/sections/UsageStatsSettings").then((m) => ({ default: m.UsageStatsSettings })),
    layout: "default",
  },
  "settings.archive": {
    loader: () => import("@/features/settings/sections/ArchiveSettings").then((m) => ({ default: m.ArchiveSettings })),
    layout: "default",
  },
  // ── 其他页面 ────────────────────────────────────────────────────────
  overview: {
    loader: () => import("@/features/story/OverviewPage").then((m) => ({ default: m.OverviewPage })),
    layout: "workspace",
  },
  characters: {
    loader: () => import("@/features/story/CharactersPage").then((m) => ({ default: m.CharactersPage })),
    layout: "workspace",
  },
  worldbuilding: {
    loader: () => import("@/features/story/WorldbuildingPage").then((m) => ({ default: m.WorldbuildingPage })),
    layout: "workspace",
  },
  plot: {
    loader: () => import("@/features/story/PlotPage").then((m) => ({ default: m.PlotPage })),
    layout: "workspace",
  },
  timeline: {
    loader: () => import("@/features/story/TimelinePage").then((m) => ({ default: m.TimelinePage })),
    layout: "workspace",
  },
  research: {
    loader: () => import("@/features/story/ResearchPage").then((m) => ({ default: m.ResearchPage })),
    layout: "workspace",
  },
  trends: {
    loader: () => import("@/features/radar/TrendsPage").then((m) => ({ default: m.TrendsPage })),
    layout: "default",
  },
  novels: {
    loader: () => import("@/features/novel/NovelDownloadPage").then((m) => ({ default: m.NovelDownloadPage })),
    layout: "default",
  },
  skills: {
    loader: () => import("@/features/skill/SkillsPage").then((m) => ({ default: m.SkillsPage })),
    layout: "default",
  },
  knowledge: {
    loader: () => import("@/features/knowledge/KnowledgePage").then((m) => ({ default: m.KnowledgePage })),
    layout: "default",
  },
  wiki: {
    loader: () => import("@/features/wiki/WikiPage").then((m) => ({ default: m.WikiPage })),
    layout: "default",
  },
  git: {
    loader: () => import("@/features/git/GitPage").then((m) => ({ default: m.GitPage })),
    layout: "default",
  },
  audit: {
    loader: () => import("@/features/audit/AuditPage").then((m) => ({ default: m.AuditPage })),
    layout: "default",
  },
  loops: {
    loader: () => import("@/features/loop/LoopPage"),
    layout: "default",
  },
  pipeline: {
    loader: () => import("@/features/pipeline/PipelinePage"),
    layout: "default",
  },
  editor: {
    loader: () => import("@/features/editor/EditorPage").then((m) => ({ default: m.EditorPage })),
    layout: "default",
  },
};