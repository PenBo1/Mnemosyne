import type { AppPage } from "@/types/app";

export interface Route {
  name: AppPage;
  params?: Record<string, string>;
}

export interface ChatRoute extends Route {
  name: "chat" | "main-agent";
}

export interface WorkspaceRoute extends Route {
  name: "overview" | "characters" | "worldbuilding" | "plot" | "timeline" | "research";
  workspaceId?: string;
}

export interface NovelReaderRoute extends Route {
  name: "novel-reader";
  novelId: string;
  novelTitle: string;
}