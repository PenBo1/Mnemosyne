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
  // 合并后的设置页面（10 个）
  "settings.general": {
    loader: () => import("@/features/settings/sections/GeneralSettingsUnified").then((m) => ({ default: m.GeneralSettingsUnified })),
    layout: "default",
  },
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
  "settings.ai": {
    loader: () => import("@/features/settings/sections/AISettings").then((m) => ({ default: m.AISettings })),
    layout: "default",
  },
  "settings.bookSources": {
    loader: () => import("@/features/settings/sections/BookSourcesSettings").then((m) => ({ default: m.BookSourcesSettings })),
    layout: "default",
  },
  "settings.memory": {
    loader: () => import("@/features/settings/sections/MemorySettings").then((m) => ({ default: m.MemorySettings })),
    layout: "default",
  },
  "settings.security": {
    loader: () => import("@/features/settings/sections/SecuritySettings").then((m) => ({ default: m.SecuritySettings })),
    layout: "default",
  },
  "settings.networkTools": {
    loader: () => import("@/features/settings/sections/NetworkToolsSettings").then((m) => ({ default: m.NetworkToolsSettings })),
    layout: "default",
  },
  "settings.usageStats": {
    loader: () => import("@/features/settings/sections/UsageStatsSettings").then((m) => ({ default: m.UsageStatsSettings })),
    layout: "default",
  },
  overview: {
    loader: () => import("@/features/story/pages/novel/OverviewPage").then((m) => ({ default: m.OverviewPage })),
    layout: "workspace",
  },
  characters: {
    loader: () => import("@/features/story/pages/novel/CharactersPage").then((m) => ({ default: m.CharactersPage })),
    layout: "workspace",
  },
  worldbuilding: {
    loader: () => import("@/features/story/pages/novel/WorldbuildingPage").then((m) => ({ default: m.WorldbuildingPage })),
    layout: "workspace",
  },
  plot: {
    loader: () => import("@/features/story/pages/novel/PlotPage").then((m) => ({ default: m.PlotPage })),
    layout: "workspace",
  },
  timeline: {
    loader: () => import("@/features/story/pages/novel/TimelinePage").then((m) => ({ default: m.TimelinePage })),
    layout: "workspace",
  },
  research: {
    loader: () => import("@/features/story/pages/novel/ResearchPage").then((m) => ({ default: m.ResearchPage })),
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
    loader: () => import("@/features/audit/pages/AuditPage").then((m) => ({ default: m.AuditPage })),
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